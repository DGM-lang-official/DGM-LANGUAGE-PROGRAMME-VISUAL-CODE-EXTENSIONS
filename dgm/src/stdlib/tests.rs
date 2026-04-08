#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::Arc;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;
    use std::path::PathBuf;
    use std::rc::Rc;

    use crate::ast::Span;
    use crate::error::{DgmError, ErrorCode};
    use crate::interpreter::{DgmValue, Interpreter};

    fn invoke_native(value: &DgmValue, args: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
        let mut interp = Interpreter::new(Arc::new("<stdlib-test>".to_string()));
        let span = Span::new(Arc::new("<stdlib-test>".to_string()), 1, 1);
        match value {
            DgmValue::NativeFunction { func, .. } => func.invoke(&mut interp, args, &span),
            _ => panic!("not a function"),
        }
    }

    fn make_map(entries: Vec<(&str, DgmValue)>) -> DgmValue {
        let mut map = HashMap::new();
        for (key, value) in entries {
            map.insert(key.to_string(), value);
        }
        DgmValue::Map(Rc::new(RefCell::new(map)))
    }

    fn make_string_list(items: &[&str]) -> DgmValue {
        DgmValue::List(Rc::new(RefCell::new(
            items.iter().map(|item| DgmValue::Str((*item).to_string())).collect(),
        )))
    }

    fn field(value: &DgmValue, key: &str) -> DgmValue {
        match value {
            DgmValue::Map(map) => map
                .borrow()
                .get(key)
                .cloned()
                .unwrap_or_else(|| panic!("missing field '{}'", key)),
            _ => panic!("expected map"),
        }
    }

    fn str_field(value: &DgmValue, key: &str) -> String {
        match field(value, key) {
            DgmValue::Str(text) => text,
            other => panic!("expected string field '{}', got {}", key, other),
        }
    }

    fn int_field(value: &DgmValue, key: &str) -> i64 {
        match field(value, key) {
            DgmValue::Int(number) => number,
            other => panic!("expected int field '{}', got {}", key, other),
        }
    }

    fn bool_field(value: &DgmValue, key: &str) -> bool {
        match field(value, key) {
            DgmValue::Bool(flag) => flag,
            other => panic!("expected bool field '{}', got {}", key, other),
        }
    }

    fn reserve_local_port() -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        port
    }

    fn build_http_response(status_line: &str, headers: &[(&str, &str)], body: &str) -> String {
        let mut response = format!("HTTP/1.1 {}\r\nContent-Length: {}\r\n", status_line, body.len());
        for (name, value) in headers {
            response.push_str(name);
            response.push_str(": ");
            response.push_str(value);
            response.push_str("\r\n");
        }
        response.push_str("\r\n");
        response.push_str(body);
        response
    }

    fn spawn_http_server_once(
        response: String,
    ) -> (u16, mpsc::Receiver<String>, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let (tx, rx) = mpsc::channel();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
            let request = read_http_request(&mut stream);
            let _ = tx.send(request);
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        });
        (port, rx, handle)
    }

    fn read_http_request(stream: &mut TcpStream) -> String {
        let mut request = Vec::new();
        let mut buf = [0u8; 1024];

        loop {
            match stream.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    request.extend_from_slice(&buf[..n]);
                    if request_complete(&request) {
                        break;
                    }
                }
                Err(err)
                    if err.kind() == std::io::ErrorKind::WouldBlock
                        || err.kind() == std::io::ErrorKind::TimedOut =>
                {
                    break;
                }
                Err(err) => panic!("failed to read HTTP request: {}", err),
            }
        }

        String::from_utf8_lossy(&request).into_owned()
    }

    fn request_complete(bytes: &[u8]) -> bool {
        let Some(header_end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") else {
            return false;
        };
        let body_start = header_end + 4;
        let headers = String::from_utf8_lossy(&bytes[..header_end]).to_lowercase();
        let content_length = headers
            .lines()
            .find_map(|line| line.strip_prefix("content-length:"))
            .and_then(|value| value.trim().parse::<usize>().ok())
            .unwrap_or(0);

        bytes.len() >= body_start + content_length
    }

    fn permissive_security() -> crate::stdlib::security::SecurityConfig {
        crate::stdlib::security::SecurityConfig {
            allow_exec: true,
            allow_net: true,
            ..crate::stdlib::security::SecurityConfig::default()
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
            ..SecurityConfig::default()
        });
        assert!(security::resolve_sandboxed_path("/tmp/dgm_test_sandbox/file.txt").is_ok());
        assert!(security::resolve_sandboxed_path("/tmp/dgm_test_sandbox/sub/deep/file").is_ok());
        security::set_config(crate::stdlib::security::SecurityConfig::default());
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
            ..SecurityConfig::default()
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
            ..SecurityConfig::default()
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
            ..SecurityConfig::default()
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
            ..SecurityConfig::default()
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
        use crate::stdlib::security;
        use crate::stdlib::os_mod;
        security::set_config(permissive_security());

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
            ..SecurityConfig::default()
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

    #[test]
    fn test_security_module_configure_and_status() {
        use crate::stdlib::security::{self, SecurityConfig};

        let fns = security::module();
        invoke_native(
            fns.get("configure").unwrap(),
            vec![make_map(vec![
                ("allow_fs", DgmValue::Bool(true)),
                ("allow_exec", DgmValue::Bool(false)),
                ("allow_net", DgmValue::Bool(false)),
                ("sandbox_root", DgmValue::Str("/tmp/dgm_cfg".into())),
                ("max_http_body_bytes", DgmValue::Int(2048)),
                (
                    "allowed_hosts",
                    DgmValue::List(Rc::new(RefCell::new(vec![
                        DgmValue::Str("127.0.0.1".into()),
                        DgmValue::Str("api.example.com".into()),
                    ]))),
                ),
                (
                    "allowed_programs",
                    DgmValue::List(Rc::new(RefCell::new(vec![
                        DgmValue::Str("/usr/bin/node".into()),
                        DgmValue::Str("sh".into()),
                    ]))),
                ),
            ])],
        )
        .unwrap();

        let status = invoke_native(fns.get("status").unwrap(), vec![]).unwrap();
        assert!(bool_field(&status, "allow_fs"));
        assert!(!bool_field(&status, "allow_exec"));
        assert!(!bool_field(&status, "allow_net"));
        assert_eq!(str_field(&status, "sandbox_root"), "/tmp/dgm_cfg");
        assert_eq!(int_field(&status, "max_http_body_bytes"), 2048);
        match field(&status, "allowed_hosts") {
            DgmValue::List(list) => {
                let items = list.borrow();
                assert_eq!(items.len(), 2);
            }
            other => panic!("expected list, got {}", other),
        }
        match field(&status, "allowed_programs") {
            DgmValue::List(list) => {
                let items = list.borrow();
                let values: Vec<String> = items
                    .iter()
                    .map(|item| match item {
                        DgmValue::Str(s) => s.clone(),
                        other => panic!("expected string, got {}", other),
                    })
                    .collect();
                assert_eq!(values, vec!["node".to_string(), "sh".to_string()]);
            }
            other => panic!("expected list, got {}", other),
        }

        security::set_config(SecurityConfig::default());
    }

    #[test]
    fn test_os_run_spawn_wait_and_timeout() {
        use crate::stdlib::os_mod;
        use crate::stdlib::security;

        security::set_config(permissive_security());
        let fns = os_mod::module();

        let run = invoke_native(
            fns.get("run").unwrap(),
            vec![
                DgmValue::Str("sh".into()),
                make_string_list(&["-c", "printf hello"]),
            ],
        )
        .unwrap();
        assert_eq!(str_field(&run, "stdout"), "hello");
        assert!(bool_field(&run, "ok"));

        let timed = invoke_native(
            fns.get("run_timeout").unwrap(),
            vec![
                DgmValue::Str("sh".into()),
                make_string_list(&["-c", "sleep 0.2"]),
                DgmValue::Int(50),
            ],
        )
        .unwrap();
        assert!(bool_field(&timed, "timed_out"));
        assert!(!bool_field(&timed, "ok"));

        let spawned = invoke_native(
            fns.get("spawn").unwrap(),
            vec![DgmValue::Str("sleep 0.2".into())],
        )
        .unwrap();
        let pid = int_field(&spawned, "pid");
        assert_eq!(pid, int_field(&spawned, "handle"));

        let first_wait = invoke_native(
            fns.get("wait").unwrap(),
            vec![DgmValue::Int(pid), DgmValue::Int(50)],
        )
        .unwrap();
        assert!(bool_field(&first_wait, "timed_out"));

        let second_wait = invoke_native(
            fns.get("wait").unwrap(),
            vec![DgmValue::Int(pid), DgmValue::Int(1000)],
        )
        .unwrap();
        assert!(!bool_field(&second_wait, "timed_out"));
        assert!(bool_field(&second_wait, "ok"));

        security::set_config(crate::stdlib::security::SecurityConfig::default());
    }

    #[test]
    fn test_os_allowlist_enforces_program_and_disables_shell_apis() {
        use crate::stdlib::os_mod;
        use crate::stdlib::security::{self, ProgramPolicy, SecurityConfig};

        let fns = os_mod::module();
        security::set_config(SecurityConfig {
            allow_exec: true,
            allowed_programs: ProgramPolicy::AllowList(["echo".to_string()].into_iter().collect()),
            ..SecurityConfig::default()
        });

        let normalized_run = invoke_native(
            fns.get("run").unwrap(),
            vec![
                DgmValue::Str("/bin/echo".into()),
                make_string_list(&["ok"]),
            ],
        )
        .unwrap();
        assert!(str_field(&normalized_run, "stdout").contains("ok"));

        let blocked_shell = invoke_native(
            fns.get("run").unwrap(),
            vec![
                DgmValue::Str("/bin/sh".into()),
                make_string_list(&["-c", "printf blocked"]),
            ],
        )
        .unwrap_err();
        assert_eq!(blocked_shell.code, ErrorCode::ShellExecutionDisabled);

        let blocked_program = invoke_native(
            fns.get("run").unwrap(),
            vec![
                DgmValue::Str("/usr/bin/env".into()),
                make_string_list(&["printf", "blocked"]),
            ],
        )
        .unwrap_err();
        assert_eq!(blocked_program.code, ErrorCode::ProgramNotAllowed);

        let blocked_exec = invoke_native(
            fns.get("exec").unwrap(),
            vec![DgmValue::Str("echo blocked".into())],
        )
        .unwrap_err();
        assert_eq!(blocked_exec.code, ErrorCode::ShellExecutionDisabled);

        let blocked_spawn = invoke_native(
            fns.get("spawn").unwrap(),
            vec![DgmValue::Str("sleep 0.1".into())],
        )
        .unwrap_err();
        assert_eq!(blocked_spawn.code, ErrorCode::ShellExecutionDisabled);

        security::set_config(SecurityConfig::default());
    }

    #[test]
    fn test_http_get_supports_headers_timeout_and_json_auto_parse() {
        use crate::stdlib::http_mod;
        use crate::stdlib::security::{self, SecurityConfig};

        security::set_config(permissive_security());
        let fns = http_mod::module();
        let body = r#"{"message":"ok","count":2}"#;
        let response = build_http_response(
            "200 OK",
            &[("Content-Type", "application/json"), ("X-Trace", "abc123")],
            body,
        );
        let (port, rx, handle) = spawn_http_server_once(response);

        let opts = make_map(vec![
            ("headers", make_map(vec![("X-Test", DgmValue::Str("demo".into()))])),
            ("timeout", DgmValue::Int(500)),
        ]);

        let result = invoke_native(
            fns.get("get").unwrap(),
            vec![DgmValue::Str(format!("http://127.0.0.1:{port}/users")), opts],
        )
        .unwrap();

        let request = rx.recv_timeout(Duration::from_secs(1)).unwrap().to_lowercase();
        assert!(request.contains("x-test: demo"));
        assert_eq!(int_field(&result, "status"), 200);
        assert!(bool_field(&result, "ok"));
        assert_eq!(str_field(&result, "body"), body);

        let headers = field(&result, "headers");
        assert!(str_field(&headers, "content-type").contains("application/json"));
        assert_eq!(str_field(&headers, "x-trace"), "abc123");

        let json = field(&result, "json");
        assert_eq!(str_field(&json, "message"), "ok");
        assert_eq!(int_field(&json, "count"), 2);

        handle.join().unwrap();
        security::set_config(SecurityConfig::default());
    }

    #[test]
    fn test_http_post_serializes_structured_body_as_json() {
        use crate::stdlib::http_mod;
        use crate::stdlib::security::{self, SecurityConfig};

        security::set_config(permissive_security());
        let fns = http_mod::module();
        let response = build_http_response("201 Created", &[("Content-Type", "text/plain")], "created");
        let (port, rx, handle) = spawn_http_server_once(response);

        let result = invoke_native(
            fns.get("post").unwrap(),
            vec![
                DgmValue::Str(format!("http://127.0.0.1:{port}/users")),
                make_map(vec![("name", DgmValue::Str("dgm".into()))]),
            ],
        )
        .unwrap();

        let request = rx.recv_timeout(Duration::from_secs(1)).unwrap().to_lowercase();
        assert!(request.contains("content-type: application/json"));
        assert!(request.contains("{\"name\":\"dgm\"}"));
        assert_eq!(int_field(&result, "status"), 201);
        assert_eq!(str_field(&result, "body"), "created");
        assert!(matches!(field(&result, "json"), DgmValue::Null));

        handle.join().unwrap();
        security::set_config(SecurityConfig::default());
    }

    #[test]
    fn test_http_respects_network_security_and_timeout() {
        use crate::stdlib::http_mod;
        use crate::stdlib::security::{self, SecurityConfig};

        let fns = http_mod::module();
        security::set_config(SecurityConfig {
            allow_fs: true,
            allow_exec: true,
            allow_net: false,
            sandbox_root: None,
            allowed_hosts: None,
            ..SecurityConfig::default()
        });
        assert!(invoke_native(
            fns.get("get").unwrap(),
            vec![DgmValue::Str("http://127.0.0.1:1/blocked".into())],
        )
        .is_err());

        security::set_config(SecurityConfig {
            allow_fs: true,
            allow_exec: true,
            allow_net: true,
            sandbox_root: None,
            allowed_hosts: Some(vec!["127.0.0.1".into()]),
            ..SecurityConfig::default()
        });
        assert!(invoke_native(
            fns.get("get").unwrap(),
            vec![DgmValue::Str("http://localhost:1/blocked".into())],
        )
        .is_err());

        security::set_config(permissive_security());
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = thread::spawn(move || {
            let (_stream, _) = listener.accept().unwrap();
            thread::sleep(Duration::from_millis(200));
        });

        let result = invoke_native(
            fns.get("get").unwrap(),
            vec![
                DgmValue::Str(format!("http://127.0.0.1:{port}/slow")),
                make_map(vec![("timeout", DgmValue::Int(50))]),
            ],
        );
        assert!(result.is_err());

        handle.join().unwrap();
        security::set_config(SecurityConfig::default());
    }

    #[test]
    fn test_http_body_limit_returns_soft_error_map() {
        use crate::stdlib::http_mod;
        use crate::stdlib::security::{self, SecurityConfig};

        security::set_config(SecurityConfig {
            allow_net: true,
            max_http_body_bytes: 1_024,
            ..SecurityConfig::default()
        });
        let fns = http_mod::module();
        let body = "x".repeat(10 * 1024 * 1024);
        let response = build_http_response("200 OK", &[("Content-Type", "text/plain")], &body);
        let (port, _rx, handle) = spawn_http_server_once(response);

        let result = invoke_native(
            fns.get("get").unwrap(),
            vec![DgmValue::Str(format!("http://127.0.0.1:{port}/big"))],
        )
        .unwrap();

        assert_eq!(int_field(&result, "status"), 200);
        assert!(!bool_field(&result, "ok"));
        assert_eq!(str_field(&result, "error"), "body too large");
        assert_eq!(str_field(&result, "body"), "");
        assert!(matches!(field(&result, "json"), DgmValue::Null));

        handle.join().unwrap();
        security::set_config(SecurityConfig::default());
    }

    #[test]
    fn test_net_connect_send_recv_and_timeout() {
        use crate::stdlib::net_mod;
        use crate::stdlib::security::{self, SecurityConfig};

        security::set_config(permissive_security());
        let fns = net_mod::module();

        let echo_server = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = echo_server.local_addr().unwrap().port();
        let echo_handle = thread::spawn(move || {
            let (mut stream, _) = echo_server.accept().unwrap();
            let mut buf = [0u8; 4];
            stream.read_exact(&mut buf).unwrap();
            stream.write_all(b"pong").unwrap();
        });

        let socket = invoke_native(
            fns.get("connect").unwrap(),
            vec![DgmValue::Str("127.0.0.1".into()), DgmValue::Int(port as i64)],
        )
        .unwrap();
        let socket_id = match socket {
            DgmValue::Int(id) => id,
            other => panic!("expected socket id, got {}", other),
        };

        invoke_native(
            fns.get("set_timeout").unwrap(),
            vec![DgmValue::Int(socket_id), DgmValue::Int(500)],
        )
        .unwrap();
        invoke_native(
            fns.get("send").unwrap(),
            vec![DgmValue::Int(socket_id), DgmValue::Str("ping".into())],
        )
        .unwrap();
        let response = invoke_native(
            fns.get("recv").unwrap(),
            vec![DgmValue::Int(socket_id), DgmValue::Int(4)],
        )
        .unwrap();
        match response {
            DgmValue::Str(text) => assert_eq!(text, "pong"),
            other => panic!("expected string, got {}", other),
        }
        invoke_native(fns.get("close").unwrap(), vec![DgmValue::Int(socket_id)]).unwrap();
        echo_handle.join().unwrap();

        let sleepy_server = TcpListener::bind("127.0.0.1:0").unwrap();
        let sleepy_port = sleepy_server.local_addr().unwrap().port();
        let sleepy_handle = thread::spawn(move || {
            let (_stream, _) = sleepy_server.accept().unwrap();
            thread::sleep(Duration::from_millis(200));
        });

        let sleepy_socket = invoke_native(
            fns.get("connect").unwrap(),
            vec![DgmValue::Str("127.0.0.1".into()), DgmValue::Int(sleepy_port as i64)],
        )
        .unwrap();
        let sleepy_socket_id = match sleepy_socket {
            DgmValue::Int(id) => id,
            other => panic!("expected socket id, got {}", other),
        };
        invoke_native(
            fns.get("set_timeout").unwrap(),
            vec![DgmValue::Int(sleepy_socket_id), DgmValue::Int(50)],
        )
        .unwrap();
        assert!(invoke_native(
            fns.get("recv").unwrap(),
            vec![DgmValue::Int(sleepy_socket_id), DgmValue::Int(16)],
        )
        .is_err());
        invoke_native(
            fns.get("close").unwrap(),
            vec![DgmValue::Int(sleepy_socket_id)],
        )
        .unwrap();
        sleepy_handle.join().unwrap();

        security::set_config(SecurityConfig::default());
    }

    #[test]
    fn test_net_listen_accept_and_security() {
        use crate::stdlib::net_mod;
        use crate::stdlib::security::{self, SecurityConfig};

        security::set_config(permissive_security());
        let fns = net_mod::module();
        let port = reserve_local_port();

        let listener = invoke_native(
            fns.get("listen").unwrap(),
            vec![DgmValue::Str("127.0.0.1".into()), DgmValue::Int(port as i64)],
        )
        .unwrap();
        let listener_id = match listener {
            DgmValue::Int(id) => id,
            other => panic!("expected listener id, got {}", other),
        };

        let client = thread::spawn(move || {
            thread::sleep(Duration::from_millis(25));
            let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
            stream.write_all(b"ping").unwrap();
            let mut buf = [0u8; 4];
            stream.read_exact(&mut buf).unwrap();
            String::from_utf8_lossy(&buf).into_owned()
        });

        let accepted = invoke_native(
            fns.get("accept").unwrap(),
            vec![DgmValue::Int(listener_id)],
        )
        .unwrap();
        let socket_id = int_field(&accepted, "socket");
        assert!(str_field(&accepted, "addr").contains("127.0.0.1"));

        let inbound = invoke_native(
            fns.get("recv").unwrap(),
            vec![DgmValue::Int(socket_id), DgmValue::Int(4)],
        )
        .unwrap();
        match inbound {
            DgmValue::Str(text) => assert_eq!(text, "ping"),
            other => panic!("expected string, got {}", other),
        }
        invoke_native(
            fns.get("send").unwrap(),
            vec![DgmValue::Int(socket_id), DgmValue::Str("pong".into())],
        )
        .unwrap();
        invoke_native(fns.get("close").unwrap(), vec![DgmValue::Int(socket_id)]).unwrap();
        invoke_native(
            fns.get("close_listener").unwrap(),
            vec![DgmValue::Int(listener_id)],
        )
        .unwrap();
        assert_eq!(client.join().unwrap(), "pong");

        security::set_config(SecurityConfig {
            allow_fs: true,
            allow_exec: true,
            allow_net: false,
            sandbox_root: None,
            allowed_hosts: None,
            ..SecurityConfig::default()
        });
        assert!(invoke_native(
            fns.get("listen").unwrap(),
            vec![DgmValue::Str("127.0.0.1".into()), DgmValue::Int(port as i64)],
        )
        .is_err());

        security::set_config(SecurityConfig {
            allow_fs: true,
            allow_exec: true,
            allow_net: true,
            sandbox_root: None,
            allowed_hosts: Some(vec!["127.0.0.1".into()]),
            ..SecurityConfig::default()
        });
        assert!(invoke_native(
            fns.get("connect").unwrap(),
            vec![DgmValue::Str("localhost".into()), DgmValue::Int(1)],
        )
        .is_err());

        security::set_config(SecurityConfig::default());
    }
}
