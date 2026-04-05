use std::collections::HashMap;
use std::cell::RefCell;
use std::rc::Rc;
use crate::interpreter::DgmValue;
use crate::error::DgmError;

// Static byte fragments — zero alloc, reused every call
static PREFIX_OK_TRUE: &[u8] = b"{\"ok\":true,\"";
static PREFIX_OK_FALSE: &[u8] = b"{\"ok\":false,\"";
static QUOTE_COLON: &[u8] = b"\":";
static SUFFIX: &[u8] = b"}";
static NULL_BYTES: &[u8] = b"null";
static TRUE_BYTES: &[u8] = b"true";
static FALSE_BYTES: &[u8] = b"false";

pub fn module() -> HashMap<String, DgmValue> {
    let mut m = HashMap::new();
    let fns: &[(&str, fn(Vec<DgmValue>) -> Result<DgmValue, DgmError>)] = &[
        ("parse", json_parse), ("stringify", json_stringify), ("pretty", json_pretty),
        ("raw_parts", json_raw_parts), ("stringify_bytes", json_stringify_bytes),
    ];
    for (name, func) in fns { m.insert(name.to_string(), DgmValue::NativeFunction { name: format!("json.{}", name), func: *func }); }
    m
}

// ─── Direct byte-level serializer — no serde_json::Value intermediate ───

/// Write DgmValue directly to Vec<u8> as JSON bytes. Zero intermediate alloc.
fn write_value_bytes(buf: &mut Vec<u8>, val: &DgmValue) {
    match val {
        DgmValue::Null => buf.extend_from_slice(NULL_BYTES),
        DgmValue::Bool(true) => buf.extend_from_slice(TRUE_BYTES),
        DgmValue::Bool(false) => buf.extend_from_slice(FALSE_BYTES),
        DgmValue::Int(n) => {
            // itoa is faster than format!, but we use write! to avoid dep
            let mut itoa_buf = itoa::Buffer::new();
            buf.extend_from_slice(itoa_buf.format(*n).as_bytes());
        }
        DgmValue::Float(f) => {
            let mut ryu_buf = ryu::Buffer::new();
            buf.extend_from_slice(ryu_buf.format(*f).as_bytes());
        }
        DgmValue::Str(s) => {
            buf.push(b'"');
            // Escape JSON special chars inline
            for byte in s.as_bytes() {
                match byte {
                    b'"' => buf.extend_from_slice(b"\\\""),
                    b'\\' => buf.extend_from_slice(b"\\\\"),
                    b'\n' => buf.extend_from_slice(b"\\n"),
                    b'\r' => buf.extend_from_slice(b"\\r"),
                    b'\t' => buf.extend_from_slice(b"\\t"),
                    b if *b < 0x20 => {
                        buf.extend_from_slice(b"\\u00");
                        let hi = b >> 4;
                        let lo = b & 0x0f;
                        buf.push(if hi < 10 { b'0' + hi } else { b'a' + hi - 10 });
                        buf.push(if lo < 10 { b'0' + lo } else { b'a' + lo - 10 });
                    }
                    _ => buf.push(*byte),
                }
            }
            buf.push(b'"');
        }
        DgmValue::List(l) => {
            buf.push(b'[');
            let items = l.borrow();
            for (i, item) in items.iter().enumerate() {
                if i > 0 { buf.push(b','); }
                write_value_bytes(buf, item);
            }
            buf.push(b']');
        }
        DgmValue::Map(m) => {
            buf.push(b'{');
            let map = m.borrow();
            let mut first = true;
            for (k, v) in map.iter() {
                if !first { buf.push(b','); }
                first = false;
                buf.push(b'"');
                buf.extend_from_slice(k.as_bytes()); // keys assumed safe
                buf.push(b'"');
                buf.push(b':');
                write_value_bytes(buf, v);
            }
            buf.push(b'}');
        }
        _ => {
            // Functions, instances — stringify as "<type>"
            buf.push(b'"');
            let s = format!("{}", val);
            buf.extend_from_slice(s.as_bytes());
            buf.push(b'"');
        }
    }
}

// ─── json.raw_parts(key, value) or json.raw_parts(key, value, ok) ───
// Produces: {"ok":true,"<key>":<value_json>}
// Direct byte assembly, no serde, no intermediate String.
fn json_raw_parts(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    let key = match a.get(0) {
        Some(DgmValue::Str(k)) => k,
        _ => return Err(DgmError::RuntimeError { msg: "json.raw_parts(key, value, ok?) required".into() }),
    };
    let value = match a.get(1) {
        Some(v) => v,
        None => return Err(DgmError::RuntimeError { msg: "json.raw_parts(key, value, ok?) required".into() }),
    };
    let ok = match a.get(2) {
        Some(DgmValue::Bool(b)) => *b,
        None => true,
        _ => true,
    };

    // Pre-estimate capacity: prefix + key + colon + value_estimate + suffix
    let est = 32 + key.len() + 128;
    let mut buf: Vec<u8> = Vec::with_capacity(est);

    // Static prefix
    if ok { buf.extend_from_slice(PREFIX_OK_TRUE); }
    else { buf.extend_from_slice(PREFIX_OK_FALSE); }

    // Key (no quotes needed — prefix already has opening quote via pattern)
    buf.extend_from_slice(key.as_bytes());
    buf.extend_from_slice(QUOTE_COLON);

    // Value — direct byte serialization
    write_value_bytes(&mut buf, value);

    // Suffix
    buf.extend_from_slice(SUFFIX);

    // Convert to String without re-validation (we wrote valid UTF-8)
    // SAFETY: all bytes written are valid UTF-8 (JSON is UTF-8)
    let s = unsafe { String::from_utf8_unchecked(buf) };
    Ok(DgmValue::Str(s))
}

// ─── json.stringify_bytes — fast path, no serde_json::Value tree ───
fn json_stringify_bytes(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match a.first() {
        Some(val) => {
            let mut buf: Vec<u8> = Vec::with_capacity(256);
            write_value_bytes(&mut buf, val);
            let s = unsafe { String::from_utf8_unchecked(buf) };
            Ok(DgmValue::Str(s))
        }
        None => Err(DgmError::RuntimeError { msg: "json.stringify_bytes(val) required".into() }),
    }
}

// ─── Original functions (kept for compatibility) ───

fn json_to_dgm(val: &serde_json::Value) -> DgmValue {
    match val {
        serde_json::Value::Null => DgmValue::Null,
        serde_json::Value::Bool(b) => DgmValue::Bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() { DgmValue::Int(i) }
            else { DgmValue::Float(n.as_f64().unwrap_or(0.0)) }
        }
        serde_json::Value::String(s) => DgmValue::Str(s.clone()),
        serde_json::Value::Array(arr) => DgmValue::List(Rc::new(RefCell::new(arr.iter().map(json_to_dgm).collect()))),
        serde_json::Value::Object(obj) => {
            let mut map = HashMap::new();
            for (k, v) in obj { map.insert(k.clone(), json_to_dgm(v)); }
            DgmValue::Map(Rc::new(RefCell::new(map)))
        }
    }
}

fn json_parse(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match a.first() {
        Some(DgmValue::Str(s)) => {
            let val: serde_json::Value = serde_json::from_str(s).map_err(|e| DgmError::RuntimeError { msg: format!("json.parse: {}", e) })?;
            Ok(json_to_dgm(&val))
        }
        _ => Err(DgmError::RuntimeError { msg: "json.parse(str) required".into() }),
    }
}

fn json_stringify(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    // Fast path: use direct byte writer instead of serde intermediate
    match a.first() {
        Some(val) => {
            let mut buf: Vec<u8> = Vec::with_capacity(256);
            write_value_bytes(&mut buf, val);
            let s = unsafe { String::from_utf8_unchecked(buf) };
            Ok(DgmValue::Str(s))
        }
        None => Err(DgmError::RuntimeError { msg: "json.stringify(val) required".into() }),
    }
}

fn json_pretty(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match a.first() {
        // Pretty still uses serde for readability formatting
        Some(val) => { let j = dgm_to_json(val); Ok(DgmValue::Str(serde_json::to_string_pretty(&j).unwrap_or_default())) }
        None => Err(DgmError::RuntimeError { msg: "json.pretty(val) required".into() }),
    }
}

// Only used by json.pretty now
fn dgm_to_json(val: &DgmValue) -> serde_json::Value {
    match val {
        DgmValue::Null => serde_json::Value::Null,
        DgmValue::Bool(b) => serde_json::Value::Bool(*b),
        DgmValue::Int(n) => serde_json::json!(*n),
        DgmValue::Float(f) => serde_json::json!(*f),
        DgmValue::Str(s) => serde_json::Value::String(s.clone()),
        DgmValue::List(l) => serde_json::Value::Array(l.borrow().iter().map(dgm_to_json).collect()),
        DgmValue::Map(m) => {
            let mut obj = serde_json::Map::new();
            for (k, v) in m.borrow().iter() { obj.insert(k.clone(), dgm_to_json(v)); }
            serde_json::Value::Object(obj)
        }
        _ => serde_json::Value::String(format!("{}", val)),
    }
}
