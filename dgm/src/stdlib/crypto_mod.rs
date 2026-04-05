use std::collections::HashMap;
use std::cell::RefCell;
use std::rc::Rc;
use crate::interpreter::DgmValue;
use crate::error::DgmError;

pub fn module() -> HashMap<String, DgmValue> {
    let mut m = HashMap::new();
    let fns: &[(&str, fn(Vec<DgmValue>) -> Result<DgmValue, DgmError>)] = &[
        ("sha256", crypto_sha256), ("md5", crypto_md5),
        ("base64_encode", crypto_b64_encode), ("base64_decode", crypto_b64_decode),
        ("random_bytes", crypto_random_bytes),
    ];
    for (name, func) in fns { m.insert(name.to_string(), DgmValue::NativeFunction { name: format!("crypto.{}", name), func: *func }); }
    m
}
fn crypto_sha256(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    use sha2::{Sha256, Digest};
    match a.first() { Some(DgmValue::Str(s)) => { let mut h = Sha256::new(); h.update(s.as_bytes()); Ok(DgmValue::Str(format!("{:x}", h.finalize()))) } _ => Err(DgmError::RuntimeError { msg: "sha256(str) required".into() }) }
}
fn crypto_md5(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    use md5::{Md5, Digest};
    match a.first() { Some(DgmValue::Str(s)) => { let mut h = Md5::new(); h.update(s.as_bytes()); Ok(DgmValue::Str(format!("{:x}", h.finalize()))) } _ => Err(DgmError::RuntimeError { msg: "md5(str) required".into() }) }
}
fn crypto_b64_encode(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    use base64::Engine;
    match a.first() { Some(DgmValue::Str(s)) => Ok(DgmValue::Str(base64::engine::general_purpose::STANDARD.encode(s.as_bytes()))), _ => Err(DgmError::RuntimeError { msg: "base64_encode(str) required".into() }) }
}
fn crypto_b64_decode(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    use base64::Engine;
    match a.first() { Some(DgmValue::Str(s)) => { let bytes = base64::engine::general_purpose::STANDARD.decode(s).map_err(|e| DgmError::RuntimeError { msg: format!("base64_decode: {}", e) })?; Ok(DgmValue::Str(String::from_utf8_lossy(&bytes).to_string())) } _ => Err(DgmError::RuntimeError { msg: "base64_decode(str) required".into() }) }
}
fn crypto_random_bytes(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    use rand::RngCore;
    match a.first() {
        Some(DgmValue::Int(n)) => {
            let mut bytes = vec![0u8; *n as usize];
            rand::thread_rng().fill_bytes(&mut bytes);
            let items: Vec<DgmValue> = bytes.iter().map(|b| DgmValue::Int(*b as i64)).collect();
            Ok(DgmValue::List(Rc::new(RefCell::new(items))))
        }
        _ => Err(DgmError::RuntimeError { msg: "random_bytes(n) required".into() }),
    }
}
