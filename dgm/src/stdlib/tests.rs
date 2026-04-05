#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::path::PathBuf;

    use crate::ast::Span;
    use crate::error::DgmError;
    use crate::interpreter::{DgmValue, Interpreter};

    fn invoke_native(value: &DgmValue, args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
        let mut interp = Interpreter::new(Arc::new("<stdlib-test>".to_string()));
        let span = Span::new(Arc::new("<stdlib-test>".to_string()), 1, 1);
        match value {
            DgmValue::NativeFunction { func, .. } => func.invoke(&mut interp, args, &span),
            _ => panic!("not a function"),
        }
    }

    // ─── Security tests ───

    #[test]
    fn test_sandbox_path_inside() {
        use crate::stdlib::security::{self, SecurityConfig};
        security::set_config(SecurityConfig {
            allow_fs: true,
            allow_exec: true,
            allow_net: true,
            sandbox_root: Some(PathBuf::from("/tmp/dgm_test_sandbox")),
            allowed_hosts: None,
        });
        assert!(security::resolve_sandboxed_path("/tmp/dgm_test_sandbox/file.txt").is_ok());
        assert!(security::resolve_sandboxed_path("/tmp/dgm_test_sandbox/sub/deep/file").is_ok());
        security::set_config(SecurityConfig::default());
    }

    #[test]
    fn test_sandbox_path_escape_blocked() {
        use crate::stdlib::security::{self, SecurityConfig};
        security::set_config(SecurityConfig {
            allow_fs: true,
            allow_exec: true,
            allow_net: true,
            sandbox_root: Some(PathBuf::from("/tmp/dgm_test_sandbox")),
            allowed_hosts: None,
        });
        assert!(security::resolve_sandboxed_path("/tmp/dgm_test_sandbox/../../etc/passwd").is_err());
        assert!(security::resolve_sandboxed_path("/etc/passwd").is_err());
        assert!(security::resolve_sandboxed_path("/tmp/other_dir/file").is_err());
        security::set_config(SecurityConfig::default());
    }

    #[test]
    fn test_fs_blocked_when_disabled() {
        use crate::stdlib::security::{self, SecurityConfig};
        security::set_config(SecurityConfig {
            allow_fs: false,
            allow_exec: true,
            allow_net: true,
            sandbox_root: None,
            allowed_hosts: None,
        });
        assert!(security::check_fs().is_err());
        assert!(security::resolve_sandboxed_path("/tmp/anything").is_err());
        security::set_config(SecurityConfig::default());
    }

    #[test]
    fn test_exec_blocked_when_disabled() {
        use crate::stdlib::security::{self, SecurityConfig};
        security::set_config(SecurityConfig {
            allow_fs: true,
            allow_exec: false,
            allow_net: true,
            sandbox_root: None,
            allowed_hosts: None,
        });
        assert!(security::check_exec().is_err());
        security::set_config(SecurityConfig::default());
    }

    // ─── FS module native function tests ───

    #[test]
    fn test_fs_write_read_delete_cycle() {
        use crate::stdlib::security::{self, SecurityConfig};
        use crate::stdlib::fs_mod;
        security::set_config(SecurityConfig::default());

        let test_dir = "/tmp/dgm_fs_test";
        let _ = std::fs::create_dir_all(test_dir);
        let test_file = format!("{}/test_rwd.txt", test_dir);

        let fns = fs_mod::module();
        let result = invoke_native(
            fns.get("write").unwrap(),
            vec![
            DgmValue::Str(test_file.clone()),
            DgmValue::Str("hello dgm".into()),
        ],
        );
        assert!(result.is_ok());

        let result = invoke_native(fns.get("read").unwrap(), vec![DgmValue::Str(test_file.clone())]);
        assert!(result.is_ok());
        match result.unwrap() {
            DgmValue::Str(s) => assert_eq!(s, "hello dgm"),
            _ => panic!("expected string"),
        }

        let result = invoke_native(
            fns.get("exists").unwrap(),
            vec![DgmValue::Str(test_file.clone())],
        );
        assert!(matches!(result.unwrap(), DgmValue::Bool(true)));

        let result = invoke_native(
            fns.get("delete").unwrap(),
            vec![DgmValue::Str(test_file.clone())],
        );
        assert!(result.is_ok());

        let result = invoke_native(
            fns.get("exists").unwrap(),
            vec![DgmValue::Str(test_file.clone())],
        );
        assert!(matches!(result.unwrap(), DgmValue::Bool(false)));

        let _ = std::fs::remove_dir_all(test_dir);
    }

    #[test]
    fn test_fs_append() {
        use crate::stdlib::security::{self, SecurityConfig};
        use crate::stdlib::fs_mod;
        security::set_config(SecurityConfig::default());

        let test_dir = "/tmp/dgm_fs_test_append";
        let _ = std::fs::create_dir_all(test_dir);
        let test_file = format!("{}/append.txt", test_dir);

        let fns = fs_mod::module();
        invoke_native(
            fns.get("write").unwrap(),
            vec![
            DgmValue::Str(test_file.clone()),
            DgmValue::Str("aaa".into()),
        ],
        )
        .unwrap();

        invoke_native(
            fns.get("append").unwrap(),
            vec![
            DgmValue::Str(test_file.clone()),
            DgmValue::Str("bbb".into()),
        ],
        )
        .unwrap();

        let result =
            invoke_native(fns.get("read").unwrap(), vec![DgmValue::Str(test_file.clone())])
                .unwrap();
        match result {
            DgmValue::Str(s) => assert_eq!(s, "aaabbb"),
            _ => panic!("expected string"),
        }

        let _ = std::fs::remove_dir_all(test_dir);
    }

    #[test]
    fn test_fs_list() {
        use crate::stdlib::security::{self, SecurityConfig};
        use crate::stdlib::fs_mod;
        security::set_config(SecurityConfig::default());

        let test_dir = "/tmp/dgm_fs_test_list";
        let _ = std::fs::remove_dir_all(test_dir);
        std::fs::create_dir_all(test_dir).unwrap();
        std::fs::write(format!("{}/a.txt", test_dir), "a").unwrap();
        std::fs::write(format!("{}/b.txt", test_dir), "b").unwrap();

        let fns = fs_mod::module();
        let result = invoke_native(fns.get("list").unwrap(), vec![DgmValue::Str(test_dir.into())]).unwrap();
        match result {
            DgmValue::List(l) => {
                let items = l.borrow();
                assert_eq!(items.len(), 2);
            }
            _ => panic!("expected list"),
        }

        let _ = std::fs::remove_dir_all(test_dir);
    }

    #[test]
    fn test_fs_sandbox_violation() {
        use crate::stdlib::security::{self, SecurityConfig};
        use crate::stdlib::fs_mod;
        let sandbox = "/tmp/dgm_sandbox_test";
        let _ = std::fs::create_dir_all(sandbox);

        security::set_config(SecurityConfig {
            allow_fs: true,
            allow_exec: true,
            allow_net: true,
            sandbox_root: Some(PathBuf::from(sandbox)),
            allowed_hosts: None,
        });

        let fns = fs_mod::module();
        let result = invoke_native(fns.get("read").unwrap(), vec![DgmValue::Str("/etc/hostname".into())]);
        assert!(result.is_err());

        let result = invoke_native(
            fns.get("read").unwrap(),
            vec![DgmValue::Str(format!("{}/../../../etc/hostname", sandbox))],
        );
        assert!(result.is_err());

        security::set_config(SecurityConfig::default());
        let _ = std::fs::remove_dir_all(sandbox);
    }

    // ─── OS module tests ───

    #[test]
    fn test_os_exec_enabled() {
        use crate::stdlib::security::{self, SecurityConfig};
        use crate::stdlib::os_mod;
        security::set_config(SecurityConfig::default());

        let fns = os_mod::module();
        let result =
            invoke_native(fns.get("exec").unwrap(), vec![DgmValue::Str("echo hello".into())])
                .unwrap();
        match result {
            DgmValue::Map(m) => {
                let map = m.borrow();
                match map.get("stdout") {
                    Some(DgmValue::Str(s)) => assert!(s.contains("hello")),
                    _ => panic!("expected stdout"),
                }
                assert!(matches!(map.get("ok"), Some(DgmValue::Bool(true))));
            }
            _ => panic!("expected map"),
        }
    }

    #[test]
    fn test_os_exec_blocked() {
        use crate::stdlib::security::{self, SecurityConfig};
        use crate::stdlib::os_mod;
        security::set_config(SecurityConfig {
            allow_fs: true,
            allow_exec: false,
            allow_net: true,
            sandbox_root: None,
            allowed_hosts: None,
        });

        let fns = os_mod::module();
        let result =
            invoke_native(fns.get("exec").unwrap(), vec![DgmValue::Str("echo hello".into())]);
        assert!(result.is_err());

        let result =
            invoke_native(fns.get("spawn").unwrap(), vec![DgmValue::Str("echo hello".into())]);
        assert!(result.is_err());

        security::set_config(SecurityConfig::default());
    }

    #[test]
    fn test_os_cwd_chdir() {
        use crate::stdlib::os_mod;
        let fns = os_mod::module();
        let original = invoke_native(fns.get("cwd").unwrap(), vec![]).unwrap();
        let orig_path = match &original {
            DgmValue::Str(s) => s.clone(),
            _ => panic!("expected string"),
        };

        invoke_native(fns.get("chdir").unwrap(), vec![DgmValue::Str("/tmp".into())]).unwrap();

        let new_cwd = invoke_native(fns.get("cwd").unwrap(), vec![]).unwrap();
        match &new_cwd {
            DgmValue::Str(s) => assert!(s.contains("tmp")),
            _ => panic!("expected string"),
        }

        invoke_native(fns.get("chdir").unwrap(), vec![DgmValue::Str(orig_path)]).unwrap();
    }
}
