use std::collections::HashMap;
use crate::interpreter::{DgmValue, NativeFunction};
use crate::error::DgmError;

pub fn module() -> HashMap<String, DgmValue> {
    let mut m = HashMap::new();
    m.insert("PI".into(), DgmValue::Float(std::f64::consts::PI));
    m.insert("E".into(), DgmValue::Float(std::f64::consts::E));
    m.insert("TAU".into(), DgmValue::Float(std::f64::consts::TAU));
    m.insert("INF".into(), DgmValue::Float(f64::INFINITY));
    m.insert("NAN".into(), DgmValue::Float(f64::NAN));
    let fns: &[(&str, fn(Vec<DgmValue>) -> Result<DgmValue, DgmError>)] = &[
        ("sqrt", math_sqrt), ("sin", math_sin), ("cos", math_cos), ("tan", math_tan),
        ("asin", math_asin), ("acos", math_acos), ("atan", math_atan), ("atan2", math_atan2),
        ("abs", math_abs), ("floor", math_floor), ("ceil", math_ceil), ("round", math_round),
        ("log", math_log), ("log2", math_log2), ("log10", math_log10), ("exp", math_exp),
        ("pow", math_pow), ("min", math_min), ("max", math_max), ("clamp", math_clamp),
        ("random", math_random), ("random_int", math_random_int),
        ("is_nan", math_is_nan), ("is_inf", math_is_inf),
        ("sinh", math_sinh), ("cosh", math_cosh), ("tanh", math_tanh),
        ("degrees", math_degrees), ("radians", math_radians),
        ("hypot", math_hypot), ("sign", math_sign),
        ("gcd", math_gcd), ("lcm", math_lcm), ("factorial", math_factorial),
    ];
    for (name, func) in fns {
        m.insert(
            name.to_string(),
            DgmValue::NativeFunction {
                name: format!("math.{}", name),
                func: NativeFunction::simple(*func),
            },
        );
    }
    m
}

fn to_f64(v: &DgmValue) -> Result<f64, DgmError> {
    match v { DgmValue::Int(n) => Ok(*n as f64), DgmValue::Float(f) => Ok(*f), _ => Err(DgmError::runtime("expected number")) }
}

fn math_sqrt(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> { Ok(DgmValue::Float(to_f64(a.first().unwrap_or(&DgmValue::Int(0)))?.sqrt())) }
fn math_sin(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> { Ok(DgmValue::Float(to_f64(a.first().unwrap_or(&DgmValue::Int(0)))?.sin())) }
fn math_cos(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> { Ok(DgmValue::Float(to_f64(a.first().unwrap_or(&DgmValue::Int(0)))?.cos())) }
fn math_tan(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> { Ok(DgmValue::Float(to_f64(a.first().unwrap_or(&DgmValue::Int(0)))?.tan())) }
fn math_asin(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> { Ok(DgmValue::Float(to_f64(a.first().unwrap_or(&DgmValue::Int(0)))?.asin())) }
fn math_acos(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> { Ok(DgmValue::Float(to_f64(a.first().unwrap_or(&DgmValue::Int(0)))?.acos())) }
fn math_atan(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> { Ok(DgmValue::Float(to_f64(a.first().unwrap_or(&DgmValue::Int(0)))?.atan())) }
fn math_atan2(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> { Ok(DgmValue::Float(to_f64(&a[0])?.atan2(to_f64(&a[1])?))) }
fn math_abs(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> { match a.first() { Some(DgmValue::Int(n)) => Ok(DgmValue::Int(n.abs())), Some(DgmValue::Float(f)) => Ok(DgmValue::Float(f.abs())), _ => Err(DgmError::runtime("abs() requires number")) } }
fn math_floor(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> { Ok(DgmValue::Int(to_f64(a.first().unwrap_or(&DgmValue::Int(0)))?.floor() as i64)) }
fn math_ceil(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> { Ok(DgmValue::Int(to_f64(a.first().unwrap_or(&DgmValue::Int(0)))?.ceil() as i64)) }
fn math_round(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> { Ok(DgmValue::Int(to_f64(a.first().unwrap_or(&DgmValue::Int(0)))?.round() as i64)) }
fn math_log(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    let x = to_f64(&a[0])?;
    if a.len() > 1 { let base = to_f64(&a[1])?; Ok(DgmValue::Float(x.log(base))) }
    else { Ok(DgmValue::Float(x.ln())) }
}
fn math_log2(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> { Ok(DgmValue::Float(to_f64(&a[0])?.log2())) }
fn math_log10(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> { Ok(DgmValue::Float(to_f64(&a[0])?.log10())) }
fn math_exp(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> { Ok(DgmValue::Float(to_f64(&a[0])?.exp())) }
fn math_pow(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> { Ok(DgmValue::Float(to_f64(&a[0])?.powf(to_f64(&a[1])?))) }
fn math_min(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> { let x = to_f64(&a[0])?; let y = to_f64(&a[1])?; Ok(DgmValue::Float(x.min(y))) }
fn math_max(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> { let x = to_f64(&a[0])?; let y = to_f64(&a[1])?; Ok(DgmValue::Float(x.max(y))) }
fn math_clamp(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> { let x = to_f64(&a[0])?; let lo = to_f64(&a[1])?; let hi = to_f64(&a[2])?; Ok(DgmValue::Float(x.max(lo).min(hi))) }
fn math_random(_a: Vec<DgmValue>) -> Result<DgmValue, DgmError> { use rand::Rng; Ok(DgmValue::Float(rand::thread_rng().gen::<f64>())) }
fn math_random_int(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    use rand::Rng;
    match (a.get(0), a.get(1)) {
        (Some(DgmValue::Int(lo)), Some(DgmValue::Int(hi))) => Ok(DgmValue::Int(rand::thread_rng().gen_range(*lo..*hi))),
        _ => Err(DgmError::runtime("random_int(lo, hi) requires ints")),
    }
}
fn math_is_nan(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> { Ok(DgmValue::Bool(matches!(a.first(), Some(DgmValue::Float(f)) if f.is_nan()))) }
fn math_is_inf(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> { Ok(DgmValue::Bool(matches!(a.first(), Some(DgmValue::Float(f)) if f.is_infinite()))) }
fn math_sinh(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> { Ok(DgmValue::Float(to_f64(&a[0])?.sinh())) }
fn math_cosh(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> { Ok(DgmValue::Float(to_f64(&a[0])?.cosh())) }
fn math_tanh(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> { Ok(DgmValue::Float(to_f64(&a[0])?.tanh())) }
fn math_degrees(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> { Ok(DgmValue::Float(to_f64(&a[0])?.to_degrees())) }
fn math_radians(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> { Ok(DgmValue::Float(to_f64(&a[0])?.to_radians())) }
fn math_hypot(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> { Ok(DgmValue::Float(to_f64(&a[0])?.hypot(to_f64(&a[1])?))) }
fn math_sign(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match a.first() { Some(DgmValue::Int(n)) => Ok(DgmValue::Int(n.signum())), Some(DgmValue::Float(f)) => Ok(DgmValue::Float(f.signum())), _ => Err(DgmError::runtime("sign() requires number")) }
}
fn math_gcd(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match (a.get(0), a.get(1)) {
        (Some(DgmValue::Int(x)), Some(DgmValue::Int(y))) => { let (mut a, mut b) = (x.abs(), y.abs()); while b != 0 { let t = b; b = a % b; a = t; } Ok(DgmValue::Int(a)) }
        _ => Err(DgmError::runtime("gcd() requires ints")),
    }
}
fn math_lcm(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match (a.get(0), a.get(1)) {
        (Some(DgmValue::Int(x)), Some(DgmValue::Int(y))) => {
            let (mut a, mut b) = (x.abs(), y.abs()); let prod = a * b;
            while b != 0 { let t = b; b = a % b; a = t; }
            Ok(DgmValue::Int(prod / a))
        }
        _ => Err(DgmError::runtime("lcm() requires ints")),
    }
}
fn math_factorial(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match a.first() {
        Some(DgmValue::Int(n)) => { if *n < 0 { return Err(DgmError::runtime("factorial of negative")); } let mut r: i64 = 1; for i in 2..=*n { r = r.wrapping_mul(i); } Ok(DgmValue::Int(r)) }
        _ => Err(DgmError::runtime("factorial() requires int")),
    }
}
