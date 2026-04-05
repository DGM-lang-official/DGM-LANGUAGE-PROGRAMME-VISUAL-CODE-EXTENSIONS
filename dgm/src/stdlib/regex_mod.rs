use std::collections::HashMap;
use std::cell::RefCell;
use std::rc::Rc;
use crate::interpreter::DgmValue;
use crate::error::DgmError;

pub fn module() -> HashMap<String, DgmValue> {
    let mut m = HashMap::new();
    let fns: &[(&str, fn(Vec<DgmValue>) -> Result<DgmValue, DgmError>)] = &[
        ("match_first", regex_match), ("find_all", regex_find_all),
        ("replace", regex_replace), ("test", regex_test), ("split", regex_split),
    ];
    for (name, func) in fns { m.insert(name.to_string(), DgmValue::NativeFunction { name: format!("regex.{}", name), func: *func }); }
    m
}
fn regex_match(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match (a.get(0), a.get(1)) {
        (Some(DgmValue::Str(text)), Some(DgmValue::Str(pat))) => {
            let re = regex::Regex::new(pat).map_err(|e| DgmError::RuntimeError { msg: format!("regex: {}", e) })?;
            Ok(re.find(text).map(|m| DgmValue::Str(m.as_str().to_string())).unwrap_or(DgmValue::Null))
        }
        _ => Err(DgmError::RuntimeError { msg: "regex.match(text, pattern) required".into() }),
    }
}
fn regex_find_all(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match (a.get(0), a.get(1)) {
        (Some(DgmValue::Str(text)), Some(DgmValue::Str(pat))) => {
            let re = regex::Regex::new(pat).map_err(|e| DgmError::RuntimeError { msg: format!("regex: {}", e) })?;
            let matches: Vec<DgmValue> = re.find_iter(text).map(|m| DgmValue::Str(m.as_str().to_string())).collect();
            Ok(DgmValue::List(Rc::new(RefCell::new(matches))))
        }
        _ => Err(DgmError::RuntimeError { msg: "regex.find_all(text, pattern) required".into() }),
    }
}
fn regex_replace(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match (a.get(0), a.get(1), a.get(2)) {
        (Some(DgmValue::Str(text)), Some(DgmValue::Str(pat)), Some(DgmValue::Str(rep))) => {
            let re = regex::Regex::new(pat).map_err(|e| DgmError::RuntimeError { msg: format!("regex: {}", e) })?;
            Ok(DgmValue::Str(re.replace_all(text, rep.as_str()).to_string()))
        }
        _ => Err(DgmError::RuntimeError { msg: "regex.replace(text, pattern, replacement) required".into() }),
    }
}
fn regex_test(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match (a.get(0), a.get(1)) {
        (Some(DgmValue::Str(text)), Some(DgmValue::Str(pat))) => {
            let re = regex::Regex::new(pat).map_err(|e| DgmError::RuntimeError { msg: format!("regex: {}", e) })?;
            Ok(DgmValue::Bool(re.is_match(text)))
        }
        _ => Err(DgmError::RuntimeError { msg: "regex.test(text, pattern) required".into() }),
    }
}
fn regex_split(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match (a.get(0), a.get(1)) {
        (Some(DgmValue::Str(text)), Some(DgmValue::Str(pat))) => {
            let re = regex::Regex::new(pat).map_err(|e| DgmError::RuntimeError { msg: format!("regex: {}", e) })?;
            let parts: Vec<DgmValue> = re.split(text).map(|s| DgmValue::Str(s.to_string())).collect();
            Ok(DgmValue::List(Rc::new(RefCell::new(parts))))
        }
        _ => Err(DgmError::RuntimeError { msg: "regex.split(text, pattern) required".into() }),
    }
}
