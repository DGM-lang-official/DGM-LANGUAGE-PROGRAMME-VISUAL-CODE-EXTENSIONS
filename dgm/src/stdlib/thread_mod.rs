use std::collections::HashMap;
use crate::interpreter::{DgmValue, NativeFunction};
use crate::error::DgmError;

pub fn module() -> HashMap<String, DgmValue> {
    let mut m = HashMap::new();
    let fns: &[(&str, fn(Vec<DgmValue>) -> Result<DgmValue, DgmError>)] = &[
        ("sleep", thread_sleep),
        ("available_cpus", thread_cpus),
    ];
    for (name, func) in fns {
        m.insert(
            name.to_string(),
            DgmValue::NativeFunction {
                name: format!("thread.{}", name),
                func: NativeFunction::simple(*func),
            },
        );
    }
    m
}
fn thread_sleep(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match a.first() { Some(DgmValue::Int(ms)) => { std::thread::sleep(std::time::Duration::from_millis(*ms as u64)); Ok(DgmValue::Null) } _ => Err(DgmError::runtime("thread.sleep(ms) required")) }
}
fn thread_cpus(_a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    Ok(DgmValue::Int(std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1) as i64))
}
