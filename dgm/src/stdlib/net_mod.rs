use std::collections::HashMap;
use std::cell::RefCell;
use std::rc::Rc;
use std::io::{Read, Write};
use std::sync::Mutex;
use std::sync::OnceLock;
use std::net::{TcpStream, TcpListener};
use crate::interpreter::{DgmValue, NativeFunction};
use crate::error::DgmError;
use super::security;

static SOCKETS: OnceLock<Mutex<HashMap<i64, TcpStream>>> = OnceLock::new();
static LISTENERS: OnceLock<Mutex<HashMap<i64, TcpListener>>> = OnceLock::new();
static NEXT_ID: OnceLock<Mutex<i64>> = OnceLock::new();

fn get_sockets() -> &'static Mutex<HashMap<i64, TcpStream>> {
    SOCKETS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn get_listeners() -> &'static Mutex<HashMap<i64, TcpListener>> {
    LISTENERS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn next_id() -> i64 {
    let m = NEXT_ID.get_or_init(|| Mutex::new(1));
    let mut id = m.lock().unwrap();
    let v = *id;
    *id += 1;
    v
}

pub fn module() -> HashMap<String, DgmValue> {
    let mut m = HashMap::new();
    let fns: &[(&str, fn(Vec<DgmValue>) -> Result<DgmValue, DgmError>)] = &[
        ("connect", net_connect),
        ("send", net_send),
        ("recv", net_recv),
        ("close", net_close),
        ("listen", net_listen),
        ("accept", net_accept),
        ("close_listener", net_close_listener),
        ("set_timeout", net_set_timeout),
    ];
    for (name, func) in fns {
        m.insert(
            name.to_string(),
            DgmValue::NativeFunction {
                name: format!("net.{}", name),
                func: NativeFunction::simple(*func),
            },
        );
    }
    m
}

fn net_connect(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    security::check_net()?;
    match (a.get(0), a.get(1)) {
        (Some(DgmValue::Str(host)), Some(DgmValue::Int(port))) => {
            security::check_host(host)?;
            let stream = TcpStream::connect(format!("{}:{}", host, port))
                .map_err(|e| rt("net.connect", &e))?;
            let id = next_id();
            get_sockets().lock().unwrap().insert(id, stream);
            Ok(DgmValue::Int(id))
        }
        _ => Err(rt_msg("net.connect(host, port) required")),
    }
}

fn net_send(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    security::check_net()?;
    match (a.get(0), a.get(1)) {
        (Some(DgmValue::Int(id)), Some(DgmValue::Str(data))) => {
            let mut sockets = get_sockets().lock().unwrap();
            let stream = sockets.get_mut(id)
                .ok_or_else(|| rt_msg("invalid socket"))?;
            stream.write_all(data.as_bytes())
                .map_err(|e| rt("net.send", &e))?;
            Ok(DgmValue::Int(data.len() as i64))
        }
        _ => Err(rt_msg("net.send(socket, data) required")),
    }
}

fn net_recv(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    security::check_net()?;
    let bufsize = match a.get(1) { Some(DgmValue::Int(n)) => *n as usize, _ => 4096 };
    match a.first() {
        Some(DgmValue::Int(id)) => {
            let mut sockets = get_sockets().lock().unwrap();
            let stream = sockets.get_mut(id)
                .ok_or_else(|| rt_msg("invalid socket"))?;
            let mut buf = vec![0u8; bufsize];
            let n = stream.read(&mut buf)
                .map_err(|e| rt("net.recv", &e))?;
            Ok(DgmValue::Str(String::from_utf8_lossy(&buf[..n]).to_string()))
        }
        _ => Err(rt_msg("net.recv(socket) required")),
    }
}

fn net_close(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    security::check_net()?;
    match a.first() {
        Some(DgmValue::Int(id)) => { get_sockets().lock().unwrap().remove(id); Ok(DgmValue::Null) }
        _ => Err(rt_msg("net.close(socket) required")),
    }
}

fn net_listen(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    security::check_net()?;
    match (a.get(0), a.get(1)) {
        (Some(DgmValue::Str(host)), Some(DgmValue::Int(port))) => {
            let listener = TcpListener::bind(format!("{}:{}", host, port))
                .map_err(|e| rt("net.listen", &e))?;
            println!("DGM TCP listening on {}:{}", host, port);
            let id = next_id();
            get_listeners().lock().unwrap().insert(id, listener);
            Ok(DgmValue::Int(id))
        }
        _ => Err(rt_msg("net.listen(host, port) required")),
    }
}

// net.accept(listener_id) -> {"socket": id, "addr": str}
fn net_accept(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    security::check_net()?;
    match a.first() {
        Some(DgmValue::Int(listener_id)) => {
            let listener = {
                let listeners = get_listeners().lock().unwrap();
                listeners.get(listener_id)
                    .ok_or_else(|| rt_msg("invalid listener"))?
                    .try_clone()
                    .map_err(|e| rt("net.accept", &e))?
            };
            let (stream, addr) = listener.accept()
                .map_err(|e| rt("net.accept", &e))?;
            let sock_id = next_id();
            get_sockets().lock().unwrap().insert(sock_id, stream);
            let mut result = HashMap::new();
            result.insert("socket".into(), DgmValue::Int(sock_id));
            result.insert("addr".into(), DgmValue::Str(addr.to_string()));
            Ok(DgmValue::Map(Rc::new(RefCell::new(result))))
        }
        _ => Err(rt_msg("net.accept(listener) required")),
    }
}

fn net_close_listener(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    security::check_net()?;
    match a.first() {
        Some(DgmValue::Int(id)) => {
            get_listeners().lock().unwrap().remove(id);
            Ok(DgmValue::Null)
        }
        _ => Err(rt_msg("net.close_listener(listener) required")),
    }
}

// ─── net.set_timeout(socket, ms) ───
fn net_set_timeout(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    security::check_net()?;
    match (a.get(0), a.get(1)) {
        (Some(DgmValue::Int(id)), Some(DgmValue::Int(ms))) => {
            let mut sockets = get_sockets().lock().unwrap();
            let stream = sockets.get_mut(id)
                .ok_or_else(|| rt_msg("invalid socket"))?;
            let dur = if *ms > 0 {
                Some(std::time::Duration::from_millis(*ms as u64))
            } else {
                None
            };
            stream.set_read_timeout(dur).map_err(|e| rt("net.set_timeout", &e))?;
            stream.set_write_timeout(dur).map_err(|e| rt("net.set_timeout", &e))?;
            Ok(DgmValue::Null)
        }
        _ => Err(rt_msg("net.set_timeout(socket, ms) required")),
    }
}

// ─── Helpers ───

#[inline]
fn rt(ctx: &str, e: &dyn std::fmt::Display) -> DgmError {
    DgmError::runtime(format!("{}: {}", ctx, e))
}

#[inline]
fn rt_msg(msg: &str) -> DgmError {
    DgmError::runtime(msg)
}
