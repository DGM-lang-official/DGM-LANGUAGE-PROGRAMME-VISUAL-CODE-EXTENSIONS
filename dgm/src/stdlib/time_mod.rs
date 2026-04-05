use crate::error::DgmError;
use crate::interpreter::{DgmValue, NativeFunction};
use std::collections::HashMap;

pub fn module() -> HashMap<String, DgmValue> {
    let mut m = HashMap::new();
    let fns: &[(&str, fn(Vec<DgmValue>) -> Result<DgmValue, DgmError>)] = &[
        ("now", time_now),
        ("now_ms", time_now_ms),
        ("format", time_format),
        ("parse", time_parse),
        ("elapsed", time_elapsed),
    ];
    for (name, func) in fns {
        m.insert(
            name.to_string(),
            DgmValue::NativeFunction {
                name: format!("time.{}", name),
                func: NativeFunction::simple(*func),
            },
        );
    }
    m
}

fn time_now(_a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    Ok(DgmValue::Int(chrono::Utc::now().timestamp()))
}

fn time_now_ms(_a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    Ok(DgmValue::Int(chrono::Utc::now().timestamp_millis()))
}

fn time_format(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    use chrono::{TimeZone, Utc};

    match (a.get(0), a.get(1)) {
        (Some(DgmValue::Int(ts)), Some(DgmValue::Str(fmt))) => {
            let dt = Utc
                .timestamp_opt(*ts, 0)
                .single()
                .ok_or_else(|| DgmError::runtime("invalid timestamp"))?;
            Ok(DgmValue::Str(dt.format(fmt).to_string()))
        }
        _ => Err(DgmError::runtime("time.format(timestamp, fmt) required")),
    }
}

fn time_parse(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    use chrono::NaiveDateTime;

    match (a.get(0), a.get(1)) {
        (Some(DgmValue::Str(s)), Some(DgmValue::Str(fmt))) => {
            let dt = NaiveDateTime::parse_from_str(s, fmt)
                .map_err(|e| DgmError::runtime(format!("time.parse: {}", e)))?;
            Ok(DgmValue::Int(dt.and_utc().timestamp()))
        }
        _ => Err(DgmError::runtime("time.parse(str, fmt) required")),
    }
}

fn time_elapsed(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match a.first() {
        Some(DgmValue::Int(start_ms)) => Ok(DgmValue::Int(
            chrono::Utc::now().timestamp_millis() - start_ms,
        )),
        _ => Err(DgmError::runtime("time.elapsed(start_ms) required")),
    }
}
