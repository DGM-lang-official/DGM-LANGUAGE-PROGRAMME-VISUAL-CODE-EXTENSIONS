use std::collections::HashMap;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Mutex, OnceLock};
use crate::interpreter::{DgmValue, NativeFunction};
use crate::error::DgmError;
use super::security;

// ─── Managed child process store (replaces mem::forget) ───
static CHILDREN: OnceLock<Mutex<HashMap<i64, std::process::Child>>> = OnceLock::new();
static NEXT_PID: OnceLock<Mutex<i64>> = OnceLock::new();

fn get_children() -> &'static Mutex<HashMap<i64, std::process::Child>> {
    CHILDREN.get_or_init(|| Mutex::new(HashMap::new()))
}

fn next_child_id() -> i64 {
    let m = NEXT_PID.get_or_init(|| Mutex::new(1));
    let mut id = m.lock().unwrap();
    let v = *id;
    *id += 1;
    v
}

pub fn module() -> HashMap<String, DgmValue> {
    let mut m = HashMap::new();
    let fns: &[(&str, fn(Vec<DgmValue>) -> Result<DgmValue, DgmError>)] = &[
        ("exec", os_exec),
        ("spawn", os_spawn),
        ("run", os_run),
        ("run_timeout", os_run_timeout),
        ("wait", os_wait),
        ("env", os_env),
        ("env_get", os_env),
        ("set_env", os_set_env),
        ("env_set", os_set_env),
        ("platform", os_platform),
        ("exit", os_exit),
        ("args", os_args),
        ("pid", os_pid),
        ("sleep", os_sleep),
        ("home_dir", os_home_dir),
        ("arch", os_arch),
        ("num_cpus", os_num_cpus),
        ("cwd", os_cwd),
        ("chdir", os_chdir),
    ];
    for (name, func) in fns {
        m.insert(
            name.to_string(),
            DgmValue::NativeFunction {
                name: format!("os.{}", name),
                func: NativeFunction::simple(*func),
            },
        );
    }
    m
}

// ─── os.exec(cmd) — shell-based, legacy compat ───
fn os_exec(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    security::check_exec()?;
    match a.first() {
        Some(DgmValue::Str(cmd)) => {
            let shell = if cfg!(target_os = "windows") { "cmd" } else { "sh" };
            let flag = if cfg!(target_os = "windows") { "/C" } else { "-c" };
            let output = std::process::Command::new(shell)
                .arg(flag)
                .arg(cmd)
                .output()
                .map_err(|e| rt("os.exec", &e))?;
            Ok(make_output_map(&output))
        }
        _ => Err(rt_msg("os.exec(cmd) required")),
    }
}

// ─── os.spawn(cmd) — shell-based, now stores child handle ───
fn os_spawn(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    security::check_exec()?;
    match a.first() {
        Some(DgmValue::Str(cmd)) => {
            let shell = if cfg!(target_os = "windows") { "cmd" } else { "sh" };
            let flag = if cfg!(target_os = "windows") { "/C" } else { "-c" };
            let child = std::process::Command::new(shell)
                .arg(flag)
                .arg(cmd)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .map_err(|e| rt("os.spawn", &e))?;
            let real_pid = child.id() as i64;
            let handle_id = next_child_id();
            get_children().lock().unwrap().insert(handle_id, child);
            let mut result = HashMap::new();
            result.insert("handle".into(), DgmValue::Int(handle_id));
            result.insert("pid".into(), DgmValue::Int(real_pid));
            result.insert("ok".into(), DgmValue::Bool(true));
            Ok(DgmValue::Map(Rc::new(RefCell::new(result))))
        }
        _ => Err(rt_msg("os.spawn(cmd) required")),
    }
}

// ─── os.run(program, args_list) — safe, no shell ───
fn os_run(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    security::check_exec()?;
    let program = match a.get(0) {
        Some(DgmValue::Str(s)) => s.clone(),
        _ => return Err(rt_msg("os.run(program, args_list) required")),
    };
    let args = extract_string_list(&a, 1)?;
    let output = std::process::Command::new(&program)
        .args(&args)
        .output()
        .map_err(|e| rt("os.run", &e))?;
    Ok(make_output_map(&output))
}

// ─── os.run_timeout(program, args_list, timeout_ms) — safe, no shell, with timeout ───
fn os_run_timeout(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    security::check_exec()?;
    let program = match a.get(0) {
        Some(DgmValue::Str(s)) => s.clone(),
        _ => return Err(rt_msg("os.run_timeout(program, args_list, timeout_ms) required")),
    };
    let args = extract_string_list(&a, 1)?;
    let timeout_ms = match a.get(2) {
        Some(DgmValue::Int(n)) => *n as u64,
        _ => return Err(rt_msg("os.run_timeout: timeout_ms (int) required")),
    };

    let mut child = std::process::Command::new(&program)
        .args(&args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| rt("os.run_timeout", &e))?;

    let timeout = std::time::Duration::from_millis(timeout_ms);
    let start = std::time::Instant::now();

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let stdout = child.stdout.take()
                    .map(|mut s| { let mut b = String::new(); std::io::Read::read_to_string(&mut s, &mut b).ok(); b })
                    .unwrap_or_default();
                let stderr = child.stderr.take()
                    .map(|mut s| { let mut b = String::new(); std::io::Read::read_to_string(&mut s, &mut b).ok(); b })
                    .unwrap_or_default();
                let mut result = HashMap::new();
                result.insert("stdout".into(), DgmValue::Str(stdout));
                result.insert("stderr".into(), DgmValue::Str(stderr));
                result.insert("code".into(), DgmValue::Int(status.code().unwrap_or(-1) as i64));
                result.insert("ok".into(), DgmValue::Bool(status.success()));
                result.insert("timed_out".into(), DgmValue::Bool(false));
                return Ok(DgmValue::Map(Rc::new(RefCell::new(result))));
            }
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    let mut result = HashMap::new();
                    result.insert("stdout".into(), DgmValue::Str(String::new()));
                    result.insert("stderr".into(), DgmValue::Str(String::new()));
                    result.insert("code".into(), DgmValue::Int(-1));
                    result.insert("ok".into(), DgmValue::Bool(false));
                    result.insert("timed_out".into(), DgmValue::Bool(true));
                    return Ok(DgmValue::Map(Rc::new(RefCell::new(result))));
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Err(e) => return Err(rt("os.run_timeout", &e)),
        }
    }
}

// ─── os.wait(handle, timeout_ms?) — wait on spawned child ───
fn os_wait(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    let handle_id = match a.get(0) {
        Some(DgmValue::Int(id)) => *id,
        _ => return Err(rt_msg("os.wait(handle, timeout_ms?) required")),
    };
    let timeout_ms = match a.get(1) {
        Some(DgmValue::Int(n)) => Some(*n as u64),
        _ => None,
    };

    let mut children = get_children().lock().unwrap();
    let child = children.get_mut(&handle_id)
        .ok_or_else(|| rt_msg("os.wait: invalid handle"))?;

    if let Some(ms) = timeout_ms {
        let timeout = std::time::Duration::from_millis(ms);
        let start = std::time::Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    let _c = children.remove(&handle_id);
                    drop(children);
                    let mut result = HashMap::new();
                    result.insert("code".into(), DgmValue::Int(status.code().unwrap_or(-1) as i64));
                    result.insert("ok".into(), DgmValue::Bool(status.success()));
                    result.insert("timed_out".into(), DgmValue::Bool(false));
                    return Ok(DgmValue::Map(Rc::new(RefCell::new(result))));
                }
                Ok(None) => {
                    if start.elapsed() >= timeout {
                        let mut result = HashMap::new();
                        result.insert("code".into(), DgmValue::Int(-1));
                        result.insert("ok".into(), DgmValue::Bool(false));
                        result.insert("timed_out".into(), DgmValue::Bool(true));
                        return Ok(DgmValue::Map(Rc::new(RefCell::new(result))));
                    }
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Err(e) => return Err(rt("os.wait", &e)),
            }
        }
    } else {
        let status = child.wait().map_err(|e| rt("os.wait", &e))?;
        children.remove(&handle_id);
        drop(children);
        let mut result = HashMap::new();
        result.insert("code".into(), DgmValue::Int(status.code().unwrap_or(-1) as i64));
        result.insert("ok".into(), DgmValue::Bool(status.success()));
        result.insert("timed_out".into(), DgmValue::Bool(false));
        Ok(DgmValue::Map(Rc::new(RefCell::new(result))))
    }
}

fn os_env(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match a.first() {
        Some(DgmValue::Str(k)) => Ok(std::env::var(k)
            .map(DgmValue::Str)
            .unwrap_or(DgmValue::Null)),
        _ => Err(rt_msg("os.env(key) required")),
    }
}

fn os_set_env(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match (a.get(0), a.get(1)) {
        (Some(DgmValue::Str(k)), Some(DgmValue::Str(v))) => {
            unsafe { std::env::set_var(k, v) };
            Ok(DgmValue::Null)
        }
        _ => Err(rt_msg("os.set_env(key, val) required")),
    }
}

fn os_cwd(_a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    std::env::current_dir()
        .map(|p| DgmValue::Str(p.to_string_lossy().into_owned()))
        .map_err(|e| rt("os.cwd", &e))
}

fn os_chdir(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match a.first() {
        Some(DgmValue::Str(p)) => {
            std::env::set_current_dir(p).map_err(|e| rt("os.chdir", &e))?;
            Ok(DgmValue::Null)
        }
        _ => Err(rt_msg("os.chdir(path) required")),
    }
}

fn os_platform(_a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    Ok(DgmValue::Str(std::env::consts::OS.to_string()))
}

fn os_exit(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    let code = match a.first() {
        Some(DgmValue::Int(n)) => *n as i32,
        _ => 0,
    };
    std::process::exit(code);
}

fn os_args(_a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    let args: Vec<DgmValue> = std::env::args().map(DgmValue::Str).collect();
    Ok(DgmValue::List(Rc::new(RefCell::new(args))))
}

fn os_pid(_a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    Ok(DgmValue::Int(std::process::id() as i64))
}

fn os_sleep(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match a.first() {
        Some(DgmValue::Int(ms)) => {
            std::thread::sleep(std::time::Duration::from_millis(*ms as u64));
            Ok(DgmValue::Null)
        }
        _ => Err(rt_msg("os.sleep(ms) required")),
    }
}

fn os_home_dir(_a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    Ok(DgmValue::Str(std::env::var("HOME").unwrap_or_default()))
}

fn os_arch(_a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    Ok(DgmValue::Str(std::env::consts::ARCH.to_string()))
}

fn os_num_cpus(_a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    Ok(DgmValue::Int(
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1) as i64,
    ))
}

// ─── Helpers ───

fn make_output_map(output: &std::process::Output) -> DgmValue {
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let mut result = HashMap::new();
    result.insert("stdout".into(), DgmValue::Str(stdout));
    result.insert("stderr".into(), DgmValue::Str(stderr));
    result.insert("code".into(), DgmValue::Int(output.status.code().unwrap_or(-1) as i64));
    result.insert("ok".into(), DgmValue::Bool(output.status.success()));
    DgmValue::Map(Rc::new(RefCell::new(result)))
}

fn extract_string_list(a: &[DgmValue], idx: usize) -> Result<Vec<String>, DgmError> {
    match a.get(idx) {
        Some(DgmValue::List(l)) => {
            Ok(l.borrow().iter().map(|v| format!("{}", v)).collect())
        }
        None => Ok(vec![]),
        _ => Err(rt_msg("args list required")),
    }
}

#[inline]
fn rt(ctx: &str, e: &dyn std::fmt::Display) -> DgmError {
    DgmError::runtime(format!("{}: {}", ctx, e))
}

#[inline]
fn rt_msg(msg: &str) -> DgmError {
    DgmError::runtime(msg)
}
