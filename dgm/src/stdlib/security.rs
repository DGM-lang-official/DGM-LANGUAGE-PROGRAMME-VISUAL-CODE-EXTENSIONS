use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::error::{DgmError, ErrorCode};
use crate::interpreter::{DgmValue, NativeFunction};
use std::rc::Rc;

#[derive(Debug, Clone)]
pub enum ProgramPolicy {
    AllowAll,
    AllowList(HashSet<String>),
}

#[derive(Debug, Clone)]
pub struct SecurityConfig {
    pub allow_fs: bool,
    pub allow_exec: bool,
    pub allow_net: bool,
    pub sandbox_root: Option<PathBuf>,
    pub allowed_hosts: Option<Vec<String>>,
    pub allowed_programs: ProgramPolicy,
    pub max_http_body_bytes: usize,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            allow_fs: true,
            allow_exec: false,
            allow_net: false,
            sandbox_root: None,
            allowed_hosts: None,
            allowed_programs: ProgramPolicy::AllowAll,
            max_http_body_bytes: 1_048_576,
        }
    }
}

thread_local! {
    static SECURITY: RefCell<SecurityConfig> = RefCell::new(SecurityConfig::default());
}

pub fn set_config(config: SecurityConfig) {
    SECURITY.with(|s| *s.borrow_mut() = config);
}

pub fn get_config() -> SecurityConfig {
    SECURITY.with(|s| s.borrow().clone())
}

pub fn check_fs() -> Result<(), crate::error::DgmError> {
    let cfg = get_config();
    if !cfg.allow_fs {
        return Err(DgmError::runtime("fs: filesystem access denied by security policy"));
    }
    Ok(())
}

pub fn check_exec() -> Result<(), crate::error::DgmError> {
    let cfg = get_config();
    if !cfg.allow_exec {
        return Err(DgmError::runtime("os: exec access denied by security policy"));
    }
    Ok(())
}

pub fn check_net() -> Result<(), crate::error::DgmError> {
    let cfg = get_config();
    if !cfg.allow_net {
        return Err(DgmError::runtime("net: network access denied by security policy"));
    }
    Ok(())
}

pub fn check_program(program: &str) -> Result<(), DgmError> {
    check_exec()?;
    let cfg = get_config();
    match &cfg.allowed_programs {
        ProgramPolicy::AllowAll => Ok(()),
        ProgramPolicy::AllowList(allowlist) => {
            let normalized = normalize_program_name(program);
            if is_shell_program(&normalized) {
                return Err(DgmError::runtime_code(
                    ErrorCode::ShellExecutionDisabled,
                    "os: shell execution disabled under allowlist",
                ));
            }
            if !allowlist.contains(&normalized) {
                return Err(DgmError::runtime_code(
                    ErrorCode::ProgramNotAllowed,
                    format!("os: program '{}' not allowed", normalized),
                ));
            }
            Ok(())
        }
    }
}

pub fn check_shell_execution() -> Result<(), DgmError> {
    check_exec()?;
    match get_config().allowed_programs {
        ProgramPolicy::AllowAll => Ok(()),
        ProgramPolicy::AllowList(_) => Err(DgmError::runtime_code(
            ErrorCode::ShellExecutionDisabled,
            "os: shell execution disabled under allowlist",
        )),
    }
}

pub fn check_host(host: &str) -> Result<(), crate::error::DgmError> {
    check_net()?;
    let cfg = get_config();
    if let Some(ref whitelist) = cfg.allowed_hosts {
        let h = host.to_lowercase();
        if !whitelist.iter().any(|allowed| h == allowed.to_lowercase()) {
            return Err(DgmError::runtime(format!(
                "net: host '{}' not in allowed hosts",
                host
            )));
        }
    }
    Ok(())
}

pub fn normalize_program_name(program: &str) -> String {
    Path::new(program)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or(program)
        .to_string()
}

fn is_shell_program(program_name: &str) -> bool {
    ["sh", "bash", "zsh", "fish", "cmd", "powershell", "pwsh"]
        .iter()
        .any(|shell| program_name.eq_ignore_ascii_case(shell))
}

/// Normalize and validate path against sandbox_root.
/// Returns canonical-safe resolved path.
pub fn resolve_sandboxed_path(raw: &str) -> Result<PathBuf, crate::error::DgmError> {
    check_fs()?;
    let cfg = get_config();
    let path = Path::new(raw);

    // Make absolute
    let abs = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| DgmError::runtime(format!("fs: cannot get cwd: {}", e)))?
            .join(path)
    };

    // Normalize: resolve .. and . without requiring the path to exist
    let normalized = normalize_path(&abs);

    if let Some(ref root) = cfg.sandbox_root {
        let norm_root = normalize_path(root);
        if !normalized.starts_with(&norm_root) {
            return Err(DgmError::runtime(format!(
                "fs: path '{}' escapes sandbox root '{}'",
                normalized.display(),
                norm_root.display()
            )));
        }
    }

    Ok(normalized)
}

/// Pure path normalization (no syscalls, no canonicalize).
/// Resolves `.` and `..` components lexically.
fn normalize_path(path: &Path) -> PathBuf {
    let mut components = Vec::new();
    for comp in path.components() {
        match comp {
            std::path::Component::ParentDir => {
                if !components.is_empty() {
                    components.pop();
                }
            }
            std::path::Component::CurDir => {}
            other => components.push(other),
        }
    }
    components.iter().collect()
}

pub fn module() -> HashMap<String, DgmValue> {
    let mut m = HashMap::new();
    let fns: &[(&str, fn(Vec<DgmValue>) -> Result<DgmValue, crate::error::DgmError>)] = &[
        ("configure", security_configure),
        ("status", security_status),
    ];
    for (name, func) in fns {
        m.insert(
            name.to_string(),
            DgmValue::NativeFunction {
                name: format!("security.{}", name),
                func: NativeFunction::simple(*func),
            },
        );
    }
    m
}

/// security.configure(opts_map)
/// Keys: "allow_fs" (bool), "allow_exec" (bool), "allow_net" (bool),
///       "sandbox_root" (str|null), "allowed_hosts" (list of str | null),
///       "allowed_programs" (list of str | null),
///       "max_http_body_bytes" (int)
fn security_configure(a: Vec<DgmValue>) -> Result<DgmValue, crate::error::DgmError> {
    match a.first() {
        Some(DgmValue::Map(m)) => {
            let map = m.borrow();
            let mut cfg = get_config();

            if let Some(DgmValue::Bool(v)) = map.get("allow_fs") { cfg.allow_fs = *v; }
            if let Some(DgmValue::Bool(v)) = map.get("allow_exec") { cfg.allow_exec = *v; }
            if let Some(DgmValue::Bool(v)) = map.get("allow_net") { cfg.allow_net = *v; }

            match map.get("sandbox_root") {
                Some(DgmValue::Str(s)) => cfg.sandbox_root = Some(PathBuf::from(s)),
                Some(DgmValue::Null) => cfg.sandbox_root = None,
                _ => {}
            }

            match map.get("allowed_hosts") {
                Some(DgmValue::List(l)) => {
                    let hosts: Vec<String> = l.borrow().iter().filter_map(|v| {
                        if let DgmValue::Str(s) = v { Some(s.clone()) } else { None }
                    }).collect();
                    cfg.allowed_hosts = if hosts.is_empty() { None } else { Some(hosts) };
                }
                Some(DgmValue::Null) => cfg.allowed_hosts = None,
                _ => {}
            }

            match map.get("allowed_programs") {
                Some(DgmValue::List(l)) => {
                    let programs: HashSet<String> = l
                        .borrow()
                        .iter()
                        .filter_map(|v| {
                            if let DgmValue::Str(s) = v {
                                Some(normalize_program_name(s))
                            } else {
                                None
                            }
                        })
                        .collect();
                    cfg.allowed_programs = ProgramPolicy::AllowList(programs);
                }
                Some(DgmValue::Null) => cfg.allowed_programs = ProgramPolicy::AllowAll,
                _ => {}
            }

            if let Some(DgmValue::Int(n)) = map.get("max_http_body_bytes") {
                if *n >= 0 {
                    cfg.max_http_body_bytes = *n as usize;
                }
            }

            set_config(cfg);
            Ok(DgmValue::Bool(true))
        }
        _ => Err(DgmError::runtime("security.configure(opts_map) required")),
    }
}

/// security.status() → Map with current config
fn security_status(_a: Vec<DgmValue>) -> Result<DgmValue, crate::error::DgmError> {
    let cfg = get_config();
    let mut map = HashMap::new();
    map.insert("allow_fs".into(), DgmValue::Bool(cfg.allow_fs));
    map.insert("allow_exec".into(), DgmValue::Bool(cfg.allow_exec));
    map.insert("allow_net".into(), DgmValue::Bool(cfg.allow_net));
    map.insert(
        "max_http_body_bytes".into(),
        DgmValue::Int(i64::try_from(cfg.max_http_body_bytes).unwrap_or(i64::MAX)),
    );
    map.insert(
        "sandbox_root".into(),
        cfg.sandbox_root.map(|p| DgmValue::Str(p.to_string_lossy().into_owned())).unwrap_or(DgmValue::Null),
    );
    map.insert(
        "allowed_hosts".into(),
        cfg.allowed_hosts
            .map(|h| DgmValue::List(Rc::new(RefCell::new(h.into_iter().map(DgmValue::Str).collect()))))
            .unwrap_or(DgmValue::Null),
    );
    map.insert(
        "allowed_programs".into(),
        match cfg.allowed_programs {
            ProgramPolicy::AllowAll => DgmValue::Null,
            ProgramPolicy::AllowList(programs) => {
                let mut items: Vec<String> = programs.into_iter().collect();
                items.sort();
                DgmValue::List(Rc::new(RefCell::new(
                    items.into_iter().map(DgmValue::Str).collect(),
                )))
            }
        },
    );
    Ok(DgmValue::Map(Rc::new(RefCell::new(map))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_normalize_path() {
        assert_eq!(normalize_path(Path::new("/a/b/../c")), PathBuf::from("/a/c"));
        assert_eq!(normalize_path(Path::new("/a/./b/c")), PathBuf::from("/a/b/c"));
        assert_eq!(normalize_path(Path::new("/a/b/../../c")), PathBuf::from("/c"));
    }

    #[test]
    fn test_sandbox_blocks_escape() {
        set_config(SecurityConfig {
            allow_fs: true,
            allow_exec: true,
            allow_net: true,
            sandbox_root: Some(PathBuf::from("/tmp/sandbox")),
            allowed_hosts: None,
            allowed_programs: ProgramPolicy::AllowAll,
            max_http_body_bytes: SecurityConfig::default().max_http_body_bytes,
        });
        assert!(resolve_sandboxed_path("/tmp/sandbox/file.txt").is_ok());
        assert!(resolve_sandboxed_path("/tmp/sandbox/sub/file.txt").is_ok());
        assert!(resolve_sandboxed_path("/tmp/sandbox/../etc/passwd").is_err());
        assert!(resolve_sandboxed_path("/etc/passwd").is_err());
        set_config(SecurityConfig::default());
    }

    #[test]
    fn test_default_config_is_safe() {
        let cfg = SecurityConfig::default();
        assert!(cfg.allow_fs);
        assert!(!cfg.allow_exec);
        assert!(!cfg.allow_net);
        assert!(matches!(cfg.allowed_programs, ProgramPolicy::AllowAll));
    }

    #[test]
    fn test_fs_denied() {
        set_config(SecurityConfig {
            allow_fs: false,
            allow_exec: true,
            allow_net: true,
            sandbox_root: None,
            allowed_hosts: None,
            allowed_programs: ProgramPolicy::AllowAll,
            max_http_body_bytes: SecurityConfig::default().max_http_body_bytes,
        });
        assert!(resolve_sandboxed_path("/tmp/anything").is_err());
        set_config(SecurityConfig::default());
    }

    #[test]
    fn test_exec_denied() {
        set_config(SecurityConfig {
            allow_fs: true,
            allow_exec: false,
            allow_net: true,
            sandbox_root: None,
            allowed_hosts: None,
            allowed_programs: ProgramPolicy::AllowAll,
            max_http_body_bytes: SecurityConfig::default().max_http_body_bytes,
        });
        assert!(check_exec().is_err());
        set_config(SecurityConfig::default());
    }

    #[test]
    fn test_net_denied() {
        set_config(SecurityConfig {
            allow_fs: true,
            allow_exec: true,
            allow_net: false,
            sandbox_root: None,
            allowed_hosts: None,
            allowed_programs: ProgramPolicy::AllowAll,
            max_http_body_bytes: SecurityConfig::default().max_http_body_bytes,
        });
        assert!(check_net().is_err());
        set_config(SecurityConfig::default());
    }

    #[test]
    fn test_host_whitelist() {
        set_config(SecurityConfig {
            allow_fs: true,
            allow_exec: true,
            allow_net: true,
            sandbox_root: None,
            allowed_hosts: Some(vec!["example.com".into(), "api.test.io".into()]),
            allowed_programs: ProgramPolicy::AllowAll,
            max_http_body_bytes: SecurityConfig::default().max_http_body_bytes,
        });
        assert!(check_host("example.com").is_ok());
        assert!(check_host("EXAMPLE.COM").is_ok());
        assert!(check_host("evil.com").is_err());
        set_config(SecurityConfig::default());
    }

    #[test]
    fn test_program_allowlist_normalizes_binary_name() {
        let mut allowed = HashSet::new();
        allowed.insert("node".into());
        set_config(SecurityConfig {
            allow_fs: true,
            allow_exec: true,
            allow_net: true,
            sandbox_root: None,
            allowed_hosts: None,
            allowed_programs: ProgramPolicy::AllowList(allowed),
            max_http_body_bytes: SecurityConfig::default().max_http_body_bytes,
        });
        assert!(check_program("node").is_ok());
        assert!(check_program("/usr/bin/node").is_ok());
        assert!(matches!(
            check_program("/bin/sh").unwrap_err().code,
            ErrorCode::ShellExecutionDisabled
        ));
        assert!(check_program("/usr/bin/python").is_err());
        assert!(matches!(
            check_shell_execution().unwrap_err().code,
            ErrorCode::ShellExecutionDisabled
        ));
        set_config(SecurityConfig::default());
    }
}
