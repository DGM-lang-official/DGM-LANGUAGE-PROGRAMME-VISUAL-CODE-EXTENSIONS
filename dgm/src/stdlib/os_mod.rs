use std::collections::HashMap;
use std::cell::RefCell;
use std::io::Read;
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::rc::Rc;
use std::sync::{Mutex, OnceLock};
use crate::interpreter::{DgmValue, NativeFunction};
use crate::error::DgmError;
use super::security;

// ─── Managed child process store (replaces mem::forget) ───
static CHILDREN: OnceLock<Mutex<HashMap<i64, Child>>> = OnceLock::new();

fn get_children() -> &'static Mutex<HashMap<i64, Child>> {
    CHILDREN.get_or_init(|| Mutex::new(HashMap::new()))
}

fn reap_children() {
    let mut children = get_children().lock().unwrap();
    children.retain(|_, child| matches!(child.try_wait(), Ok(None)));
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
    reap_children();
    security::check_shell_execution()?;
    match a.first() {
        Some(DgmValue::Str(cmd)) => {
            let shell = if cfg!(target_os = "windows") { "cmd" } else { "sh" };
            let flag = if cfg!(target_os = "windows") { "/C" } else { "-c" };
            let output = Command::new(shell)
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
    reap_children();
    security::check_shell_execution()?;
    match a.first() {
        Some(DgmValue::Str(cmd)) => {
            let shell = if cfg!(target_os = "windows") { "cmd" } else { "sh" };
            let flag = if cfg!(target_os = "windows") { "/C" } else { "-c" };
            let child = Command::new(shell)
                .arg(flag)
                .arg(cmd)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|e| rt("os.spawn", &e))?;
            let pid = child.id() as i64;
            get_children().lock().unwrap().insert(pid, child);
            Ok(make_spawn_map(pid))
        }
        _ => Err(rt_msg("os.spawn(cmd) required")),
    }
}

// ─── os.run(program, args_list) — safe, no shell ───
fn os_run(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    reap_children();
    security::check_exec()?;
    let program = match a.get(0) {
        Some(DgmValue::Str(s)) => s.clone(),
        _ => return Err(rt_msg("os.run(program, args_list) required")),
    };
    security::check_program(&program)?;
    let args = extract_string_list(&a, 1)?;
    let output = Command::new(&program)
        .args(&args)
        .output()
        .map_err(|e| rt("os.run", &e))?;
    Ok(make_output_map(&output))
}

// ─── os.run_timeout(program, args_list, timeout_ms) — safe, no shell, with timeout ───
fn os_run_timeout(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    reap_children();
    security::check_exec()?;
    let program = match a.get(0) {
        Some(DgmValue::Str(s)) => s.clone(),
        _ => return Err(rt_msg("os.run_timeout(program, args_list, timeout_ms) required")),
    };
    security::check_program(&program)?;
    let args = extract_string_list(&a, 1)?;
    let timeout_ms = match a.get(2) {
        Some(DgmValue::Int(n)) if *n >= 0 => *n as u64,
        _ => return Err(rt_msg("os.run_timeout: timeout_ms (int) required")),
    };

    let mut child = Command::new(&program)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| rt("os.run_timeout", &e))?;

    match wait_with_timeout(&mut child, std::time::Duration::from_millis(timeout_ms))? {
        Some(status) => Ok(make_timed_output_map(&mut child, status, false)),
        None => {
            let _ = child.kill();
            let status = child.wait().map_err(|e| rt("os.run_timeout", &e))?;
            Ok(make_timed_output_map(&mut child, status, true))
        }
    }
}

// ─── os.wait(pid, timeout_ms?) — wait on spawned child ───
fn os_wait(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    reap_children();
    security::check_exec()?;
    let pid = match a.get(0) {
        Some(DgmValue::Int(id)) => *id,
        _ => return Err(rt_msg("os.wait(pid, timeout_ms?) required")),
    };
    let timeout_ms = match a.get(1) {
        Some(DgmValue::Int(n)) if *n >= 0 => Some(*n as u64),
        _ => None,
    };

    let mut child = {
        let mut children = get_children().lock().unwrap();
        children
            .remove(&pid)
            .ok_or_else(|| rt_msg("os.wait: invalid pid"))?
    };

    if let Some(ms) = timeout_ms {
        match wait_with_timeout(&mut child, std::time::Duration::from_millis(ms))? {
            Some(status) => Ok(make_wait_map(status, false)),
            None => {
                get_children().lock().unwrap().insert(pid, child);
                Ok(make_wait_timeout_map())
            }
        }
    } else {
        let status = child.wait().map_err(|e| rt("os.wait", &e))?;
        Ok(make_wait_map(status, false))
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
            std::env::set_var(k, v);
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

fn make_output_map(output: &Output) -> DgmValue {
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let mut result = HashMap::new();
    result.insert("stdout".into(), DgmValue::Str(stdout));
    result.insert("stderr".into(), DgmValue::Str(stderr));
    result.insert("code".into(), DgmValue::Int(status_code(&output.status)));
    result.insert("ok".into(), DgmValue::Bool(output.status.success()));
    DgmValue::Map(Rc::new(RefCell::new(result)))
}

fn extract_string_list(a: &[DgmValue], idx: usize) -> Result<Vec<String>, DgmError> {
    match a.get(idx) {
        Some(DgmValue::List(l)) => l.borrow().iter().map(arg_to_string).collect(),
        None => Ok(vec![]),
        _ => Err(rt_msg("args list required")),
    }
}

fn arg_to_string(value: &DgmValue) -> Result<String, DgmError> {
    match value {
        DgmValue::Str(s) => Ok(s.clone()),
        DgmValue::Int(_) | DgmValue::Float(_) | DgmValue::Bool(_) | DgmValue::Null => {
            Ok(format!("{}", value))
        }
        _ => Err(rt_msg("args list must contain only scalar values")),
    }
}

fn wait_with_timeout(child: &mut Child, timeout: std::time::Duration) -> Result<Option<ExitStatus>, DgmError> {
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(Some(status)),
            Ok(None) => {
                if start.elapsed() >= timeout {
                    return Ok(None);
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Err(e) => return Err(rt("os.wait", &e)),
        }
    }
}

fn read_pipe_to_string<R: Read>(pipe: Option<R>) -> String {
    let mut buf = String::new();
    if let Some(mut reader) = pipe {
        let _ = reader.read_to_string(&mut buf);
    }
    buf
}

fn make_timed_output_map(child: &mut Child, status: ExitStatus, timed_out: bool) -> DgmValue {
    let stdout = read_pipe_to_string(child.stdout.take());
    let stderr = read_pipe_to_string(child.stderr.take());
    let mut result = HashMap::new();
    result.insert("stdout".into(), DgmValue::Str(stdout));
    result.insert("stderr".into(), DgmValue::Str(stderr));
    result.insert("code".into(), DgmValue::Int(status_code(&status)));
    result.insert("ok".into(), DgmValue::Bool(status.success() && !timed_out));
    result.insert("timed_out".into(), DgmValue::Bool(timed_out));
    DgmValue::Map(Rc::new(RefCell::new(result)))
}

fn make_spawn_map(pid: i64) -> DgmValue {
    let mut result = HashMap::new();
    result.insert("pid".into(), DgmValue::Int(pid));
    result.insert("handle".into(), DgmValue::Int(pid));
    result.insert("ok".into(), DgmValue::Bool(true));
    DgmValue::Map(Rc::new(RefCell::new(result)))
}

fn make_wait_map(status: ExitStatus, timed_out: bool) -> DgmValue {
    let mut result = HashMap::new();
    result.insert("code".into(), DgmValue::Int(status_code(&status)));
    result.insert("ok".into(), DgmValue::Bool(status.success() && !timed_out));
    result.insert("timed_out".into(), DgmValue::Bool(timed_out));
    DgmValue::Map(Rc::new(RefCell::new(result)))
}

fn make_wait_timeout_map() -> DgmValue {
    let mut result = HashMap::new();
    result.insert("code".into(), DgmValue::Int(-1));
    result.insert("ok".into(), DgmValue::Bool(false));
    result.insert("timed_out".into(), DgmValue::Bool(true));
    DgmValue::Map(Rc::new(RefCell::new(result)))
}

fn status_code(status: &ExitStatus) -> i64 {
    status.code().unwrap_or(-1) as i64
}

#[inline]
fn rt(ctx: &str, e: &dyn std::fmt::Display) -> DgmError {
    DgmError::runtime(format!("{}: {}", ctx, e))
}

#[inline]
fn rt_msg(msg: &str) -> DgmError {
    DgmError::runtime(msg)
}
