#[cfg(test)]
mod tests {
    use std::path::PathBuf;

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
        use crate::interpreter::DgmValue;

        security::set_config(SecurityConfig::default());

        let test_dir = "/tmp/dgm_fs_test";
        let _ = std::fs::create_dir_all(test_dir);
        let test_file = format!("{}/test_rwd.txt", test_dir);

        let fns = fs_mod::module();
        let write_fn = match fns.get("write").unwrap() {
            DgmValue::NativeFunction { func, .. } => *func,
            _ => panic!("not a function"),
        };
        let result = write_fn(vec![
            DgmValue::Str(test_file.clone()),
            DgmValue::Str("hello dgm".into()),
        ]);
        assert!(result.is_ok());

        let read_fn = match fns.get("read").unwrap() {
            DgmValue::NativeFunction { func, .. } => *func,
            _ => panic!("not a function"),
        };
        let result = read_fn(vec![DgmValue::Str(test_file.clone())]);
        assert!(result.is_ok());
        match result.unwrap() {
            DgmValue::Str(s) => assert_eq!(s, "hello dgm"),
            _ => panic!("expected string"),
        }

        let exists_fn = match fns.get("exists").unwrap() {
            DgmValue::NativeFunction { func, .. } => *func,
            _ => panic!("not a function"),
        };
        let result = exists_fn(vec![DgmValue::Str(test_file.clone())]);
        assert!(matches!(result.unwrap(), DgmValue::Bool(true)));

        let delete_fn = match fns.get("delete").unwrap() {
            DgmValue::NativeFunction { func, .. } => *func,
            _ => panic!("not a function"),
        };
        let result = delete_fn(vec![DgmValue::Str(test_file.clone())]);
        assert!(result.is_ok());

        let result = exists_fn(vec![DgmValue::Str(test_file.clone())]);
        assert!(matches!(result.unwrap(), DgmValue::Bool(false)));

        let _ = std::fs::remove_dir_all(test_dir);
    }

    #[test]
    fn test_fs_append() {
        use crate::stdlib::security::{self, SecurityConfig};
        use crate::stdlib::fs_mod;
        use crate::interpreter::DgmValue;

        security::set_config(SecurityConfig::default());

        let test_dir = "/tmp/dgm_fs_test_append";
        let _ = std::fs::create_dir_all(test_dir);
        let test_file = format!("{}/append.txt", test_dir);

        let fns = fs_mod::module();
        let write_fn = match fns.get("write").unwrap() {
            DgmValue::NativeFunction { func, .. } => *func,
            _ => panic!("not a function"),
        };
        let append_fn = match fns.get("append").unwrap() {
            DgmValue::NativeFunction { func, .. } => *func,
            _ => panic!("not a function"),
        };
        let read_fn = match fns.get("read").unwrap() {
            DgmValue::NativeFunction { func, .. } => *func,
            _ => panic!("not a function"),
        };

        write_fn(vec![
            DgmValue::Str(test_file.clone()),
            DgmValue::Str("aaa".into()),
        ]).unwrap();

        append_fn(vec![
            DgmValue::Str(test_file.clone()),
            DgmValue::Str("bbb".into()),
        ]).unwrap();

        let result = read_fn(vec![DgmValue::Str(test_file.clone())]).unwrap();
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
        use crate::interpreter::DgmValue;

        security::set_config(SecurityConfig::default());

        let test_dir = "/tmp/dgm_fs_test_list";
        let _ = std::fs::remove_dir_all(test_dir);
        std::fs::create_dir_all(test_dir).unwrap();
        std::fs::write(format!("{}/a.txt", test_dir), "a").unwrap();
        std::fs::write(format!("{}/b.txt", test_dir), "b").unwrap();

        let fns = fs_mod::module();
        let list_fn = match fns.get("list").unwrap() {
            DgmValue::NativeFunction { func, .. } => *func,
            _ => panic!("not a function"),
        };
        let result = list_fn(vec![DgmValue::Str(test_dir.into())]).unwrap();
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
        use crate::interpreter::DgmValue;

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
        let read_fn = match fns.get("read").unwrap() {
            DgmValue::NativeFunction { func, .. } => *func,
            _ => panic!("not a function"),
        };

        let result = read_fn(vec![DgmValue::Str("/etc/hostname".into())]);
        assert!(result.is_err());

        let result = read_fn(vec![DgmValue::Str(
            format!("{}/../../../etc/hostname", sandbox),
        )]);
        assert!(result.is_err());

        security::set_config(SecurityConfig::default());
        let _ = std::fs::remove_dir_all(sandbox);
    }

    // ─── OS module tests ───

    #[test]
    fn test_os_exec_enabled() {
        use crate::stdlib::security::{self, SecurityConfig};
        use crate::stdlib::os_mod;
        use crate::interpreter::DgmValue;

        security::set_config(SecurityConfig::default());

        let fns = os_mod::module();
        let exec_fn = match fns.get("exec").unwrap() {
            DgmValue::NativeFunction { func, .. } => *func,
            _ => panic!("not a function"),
        };

        let result = exec_fn(vec![DgmValue::Str("echo hello".into())]).unwrap();
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
        use crate::interpreter::DgmValue;

        security::set_config(SecurityConfig {
            allow_fs: true,
            allow_exec: false,
            allow_net: true,
            sandbox_root: None,
            allowed_hosts: None,
        });

        let fns = os_mod::module();
        let exec_fn = match fns.get("exec").unwrap() {
            DgmValue::NativeFunction { func, .. } => *func,
            _ => panic!("not a function"),
        };

        let result = exec_fn(vec![DgmValue::Str("echo hello".into())]);
        assert!(result.is_err());

        let spawn_fn = match fns.get("spawn").unwrap() {
            DgmValue::NativeFunction { func, .. } => *func,
            _ => panic!("not a function"),
        };
        let result = spawn_fn(vec![DgmValue::Str("echo hello".into())]);
        assert!(result.is_err());

        security::set_config(SecurityConfig::default());
    }

    #[test]
    fn test_os_cwd_chdir() {
        use crate::stdlib::os_mod;
        use crate::interpreter::DgmValue;

        let fns = os_mod::module();
        let cwd_fn = match fns.get("cwd").unwrap() {
            DgmValue::NativeFunction { func, .. } => *func,
            _ => panic!("not a function"),
        };
        let chdir_fn = match fns.get("chdir").unwrap() {
            DgmValue::NativeFunction { func, .. } => *func,
            _ => panic!("not a function"),
        };

        let original = cwd_fn(vec![]).unwrap();
        let orig_path = match &original {
            DgmValue::Str(s) => s.clone(),
            _ => panic!("expected string"),
        };

        chdir_fn(vec![DgmValue::Str("/tmp".into())]).unwrap();

        let new_cwd = cwd_fn(vec![]).unwrap();
        match &new_cwd {
            DgmValue::Str(s) => assert!(s.contains("tmp")),
            _ => panic!("expected string"),
        }

        chdir_fn(vec![DgmValue::Str(orig_path)]).unwrap();
    }
}
