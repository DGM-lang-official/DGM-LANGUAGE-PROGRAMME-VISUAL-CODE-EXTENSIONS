use std::collections::HashMap;
use std::cell::RefCell;
use std::io::Read;
use std::rc::Rc;
use crate::interpreter::{DgmValue, NativeFunction, Interpreter};
use crate::ast::Span;
use crate::error::DgmError;
use super::security;

pub fn module() -> HashMap<String, DgmValue> {
    let mut m = HashMap::new();
    let fns: &[(&str, fn(Vec<DgmValue>) -> Result<DgmValue, DgmError>)] = &[
        ("get", http_get),
        ("post", http_post),
        ("put", http_put),
        ("delete", http_delete),
        ("request", http_request),
    ];
    for (name, func) in fns {
        m.insert(
            name.to_string(),
            DgmValue::NativeFunction {
                name: format!("http.{}", name),
                func: NativeFunction::simple(*func),
            },
        );
    }
    // serve is contextual - needs interpreter to call handler functions
    m.insert(
        "serve".to_string(),
        DgmValue::NativeFunction {
            name: "http.serve".to_string(),
            func: NativeFunction::contextual(http_serve_ctx),
        },
    );
    m
}

// ─── Options helper ───

struct RequestOpts {
    headers: Vec<(String, String)>,
    timeout_ms: u64,
}

const DEFAULT_HTTP_TIMEOUT_MS: u64 = 5_000;

impl Default for RequestOpts {
    fn default() -> Self {
        Self {
            headers: Vec::new(),
            timeout_ms: DEFAULT_HTTP_TIMEOUT_MS,
        }
    }
}

struct RequestBody {
    content: String,
    is_json: bool,
}

enum BodyLimitResult {
    Ok(Vec<u8>),
    TooLarge,
}

fn parse_opts(a: &[DgmValue], idx: usize) -> Result<RequestOpts, DgmError> {
    match a.get(idx) {
        Some(value) => parse_opts_value(value),
        None => Ok(RequestOpts::default()),
    }
}

fn parse_opts_value(value: &DgmValue) -> Result<RequestOpts, DgmError> {
    let mut opts = RequestOpts::default();
    match value {
        DgmValue::Map(m) => {
            let map = m.borrow();
            if let Some(headers) = map.get("headers") {
                match headers {
                    DgmValue::Map(h) => {
                        for (k, v) in h.borrow().iter() {
                            let val = match v {
                                DgmValue::Str(s) => s.clone(),
                                other => format!("{}", other),
                            };
                            opts.headers.push((k.clone(), val));
                        }
                    }
                    _ => return Err(rt_msg("http opts.headers must be a map")),
                }
            }

            if let Some(timeout) = map.get("timeout") {
                match timeout {
                    DgmValue::Int(ms) if *ms > 0 => opts.timeout_ms = *ms as u64,
                    DgmValue::Int(_) | DgmValue::Null => {}
                    _ => return Err(rt_msg("http opts.timeout must be an int")),
                }
            }

            Ok(opts)
        }
        _ => Err(rt_msg("http opts must be a map")),
    }
}

fn build_agent(opts: &RequestOpts) -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_millis(opts.timeout_ms))
        .build()
}

fn apply_headers(req: ureq::Request, opts: &RequestOpts) -> ureq::Request {
    let mut r = req;
    for (k, v) in &opts.headers {
        r = r.set(k, v);
    }
    r
}

fn make_response(resp: ureq::Response) -> Result<DgmValue, DgmError> {
    let status = resp.status();
    let headers = extract_response_headers(&resp);
    let content_type = resp.header("Content-Type").unwrap_or_default().to_lowercase();
    let max_body_bytes = security::get_config().max_http_body_bytes;
    if response_exceeds_limit(&resp, max_body_bytes) {
        return Ok(make_body_too_large_response(status, headers));
    }

    let body = match read_response_body(resp, max_body_bytes)? {
        BodyLimitResult::Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        BodyLimitResult::TooLarge => return Ok(make_body_too_large_response(status, headers)),
    };
    let mut map = HashMap::new();
    map.insert("status".into(), DgmValue::Int(status as i64));
    map.insert("body".into(), DgmValue::Str(body.clone()));
    map.insert("ok".into(), DgmValue::Bool(status >= 200 && status < 300));
    map.insert("headers".into(), DgmValue::Map(Rc::new(RefCell::new(headers))));
    map.insert(
        "json".into(),
        if content_type.contains("json") {
            parse_json_body(&body).unwrap_or(DgmValue::Null)
        } else {
            DgmValue::Null
        },
    );
    Ok(DgmValue::Map(Rc::new(RefCell::new(map))))
}

fn response_exceeds_limit(resp: &ureq::Response, max_body_bytes: usize) -> bool {
    resp.header("Content-Length")
        .and_then(|value| value.parse::<usize>().ok())
        .map(|len| len > max_body_bytes)
        .unwrap_or(false)
}

fn read_response_body(resp: ureq::Response, max_body_bytes: usize) -> Result<BodyLimitResult, DgmError> {
    let mut reader = resp.into_reader();
    let mut bytes = Vec::with_capacity(max_body_bytes.min(4096));
    reader
        .by_ref()
        .take(max_body_bytes.saturating_add(1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| rt("http.response", &e))?;

    if bytes.len() > max_body_bytes {
        Ok(BodyLimitResult::TooLarge)
    } else {
        Ok(BodyLimitResult::Ok(bytes))
    }
}

fn make_body_too_large_response(
    status: u16,
    headers: HashMap<String, DgmValue>,
) -> DgmValue {
    let mut map = HashMap::new();
    map.insert("status".into(), DgmValue::Int(status as i64));
    map.insert("body".into(), DgmValue::Str(String::new()));
    map.insert("ok".into(), DgmValue::Bool(false));
    map.insert("error".into(), DgmValue::Str("body too large".into()));
    map.insert("headers".into(), DgmValue::Map(Rc::new(RefCell::new(headers))));
    map.insert("json".into(), DgmValue::Null);
    DgmValue::Map(Rc::new(RefCell::new(map)))
}

fn extract_host(url: &str) -> Option<String> {
    // Simple extraction: skip scheme, get host before port/path
    let without_scheme = url.strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    let host = without_scheme.split('/').next().unwrap_or("");
    let host = host.split(':').next().unwrap_or(host);
    if host.is_empty() { None } else { Some(host.to_string()) }
}

fn check_url(url: &str) -> Result<(), DgmError> {
    security::check_net()?;
    if let Some(host) = extract_host(url) {
        security::check_host(&host)?;
    }
    Ok(())
}

fn looks_like_opts(value: &DgmValue) -> bool {
    match value {
        DgmValue::Map(m) => m.borrow().keys().all(|k| k == "headers" || k == "timeout"),
        _ => false,
    }
}

fn parse_optional_body_and_opts(
    a: &[DgmValue],
    body_idx: usize,
) -> Result<(Option<RequestBody>, RequestOpts), DgmError> {
    match a.get(body_idx) {
        None => Ok((None, RequestOpts::default())),
        Some(value) if a.get(body_idx + 1).is_some() => {
            Ok((Some(encode_body(value)?), parse_opts(a, body_idx + 1)?))
        }
        Some(value) if looks_like_opts(value) => Ok((None, parse_opts_value(value)?)),
        Some(DgmValue::Null) => Ok((None, RequestOpts::default())),
        Some(value) => Ok((Some(encode_body(value)?), RequestOpts::default())),
    }
}

fn encode_body(value: &DgmValue) -> Result<RequestBody, DgmError> {
    match value {
        DgmValue::Null => Ok(RequestBody {
            content: String::new(),
            is_json: false,
        }),
        DgmValue::Str(s) => Ok(RequestBody {
            content: s.clone(),
            is_json: seems_json_text(s),
        }),
        DgmValue::Int(_)
        | DgmValue::Float(_)
        | DgmValue::Bool(_)
        | DgmValue::List(_)
        | DgmValue::Map(_) => Ok(RequestBody {
            content: serde_json::to_string(&dgm_to_json(value))
                .map_err(|e| rt("http.request", &e))?,
            is_json: true,
        }),
        _ => Err(rt_msg("http body must be a string, scalar, list, or map")),
    }
}

fn seems_json_text(text: &str) -> bool {
    let trimmed = text.trim();
    (trimmed.starts_with('{') && trimmed.ends_with('}'))
        || (trimmed.starts_with('[') && trimmed.ends_with(']'))
}

fn with_auto_content_type(req: ureq::Request, opts: &RequestOpts, body: &RequestBody) -> ureq::Request {
    if body.is_json && !has_header(opts, "content-type") {
        req.set("Content-Type", "application/json")
    } else {
        req
    }
}

fn has_header(opts: &RequestOpts, name: &str) -> bool {
    opts.headers
        .iter()
        .any(|(header_name, _)| header_name.eq_ignore_ascii_case(name))
}

fn execute_request(
    ctx: &str,
    req: ureq::Request,
    body: Option<&RequestBody>,
) -> Result<DgmValue, DgmError> {
    let response = match body {
        Some(request_body) => handle_ureq_result(ctx, req.send_string(&request_body.content))?,
        None => handle_ureq_result(ctx, req.call())?,
    };
    make_response(response)
}

fn handle_ureq_result(
    ctx: &str,
    result: Result<ureq::Response, ureq::Error>,
) -> Result<ureq::Response, DgmError> {
    match result {
        Ok(resp) => Ok(resp),
        Err(ureq::Error::Status(_, resp)) => Ok(resp),
        Err(err) => Err(rt(ctx, &err)),
    }
}

fn extract_response_headers(resp: &ureq::Response) -> HashMap<String, DgmValue> {
    let mut headers = HashMap::new();
    for name in resp.headers_names() {
        if let Some(value) = resp.header(&name) {
            headers.insert(name.to_lowercase(), DgmValue::Str(value.to_string()));
        }
    }
    headers
}

fn parse_json_body(body: &str) -> Option<DgmValue> {
    let value: serde_json::Value = serde_json::from_str(body).ok()?;
    Some(json_to_dgm(&value))
}

fn json_to_dgm(value: &serde_json::Value) -> DgmValue {
    match value {
        serde_json::Value::Null => DgmValue::Null,
        serde_json::Value::Bool(flag) => DgmValue::Bool(*flag),
        serde_json::Value::Number(number) => {
            if let Some(int) = number.as_i64() {
                DgmValue::Int(int)
            } else {
                DgmValue::Float(number.as_f64().unwrap_or(0.0))
            }
        }
        serde_json::Value::String(text) => DgmValue::Str(text.clone()),
        serde_json::Value::Array(items) => DgmValue::List(Rc::new(RefCell::new(
            items.iter().map(json_to_dgm).collect(),
        ))),
        serde_json::Value::Object(obj) => {
            let mut map = HashMap::new();
            for (key, inner) in obj {
                map.insert(key.clone(), json_to_dgm(inner));
            }
            DgmValue::Map(Rc::new(RefCell::new(map)))
        }
    }
}

fn dgm_to_json(value: &DgmValue) -> serde_json::Value {
    match value {
        DgmValue::Null => serde_json::Value::Null,
        DgmValue::Bool(flag) => serde_json::Value::Bool(*flag),
        DgmValue::Int(number) => serde_json::json!(*number),
        DgmValue::Float(number) => serde_json::json!(*number),
        DgmValue::Str(text) => serde_json::Value::String(text.clone()),
        DgmValue::List(items) => {
            serde_json::Value::Array(items.borrow().iter().map(dgm_to_json).collect())
        }
        DgmValue::Map(map) => {
            let mut obj = serde_json::Map::new();
            for (key, inner) in map.borrow().iter() {
                obj.insert(key.clone(), dgm_to_json(inner));
            }
            serde_json::Value::Object(obj)
        }
        other => serde_json::Value::String(format!("{}", other)),
    }
}

// ─── http.get(url, opts?) ───
fn http_get(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match a.first() {
        Some(DgmValue::Str(url)) => {
            check_url(url)?;
            let opts = parse_opts(&a, 1)?;
            let agent = build_agent(&opts);
            let req = apply_headers(agent.get(url), &opts);
            execute_request("http.get", req, None)
        }
        _ => Err(rt_msg("http.get(url, opts?) required")),
    }
}

// ─── http.post(url, body?, opts?) ───
fn http_post(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match a.get(0) {
        Some(DgmValue::Str(url)) => {
            check_url(url)?;
            let (body, opts) = parse_optional_body_and_opts(&a, 1)?;
            let agent = build_agent(&opts);
            let req = apply_headers(agent.post(url), &opts);
            let req = match body.as_ref() {
                Some(request_body) => with_auto_content_type(req, &opts, request_body),
                None => req,
            };
            execute_request("http.post", req, body.as_ref())
        }
        _ => Err(rt_msg("http.post(url, body?, opts?) required")),
    }
}

// ─── http.put(url, body, opts?) ───
fn http_put(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match a.get(0) {
        Some(DgmValue::Str(url)) => {
            check_url(url)?;
            let (body, opts) = parse_optional_body_and_opts(&a, 1)?;
            let body = body.ok_or_else(|| rt_msg("http.put(url, body, opts?) required"))?;
            let agent = build_agent(&opts);
            let req = apply_headers(agent.put(url), &opts);
            let req = with_auto_content_type(req, &opts, &body);
            execute_request("http.put", req, Some(&body))
        }
        _ => Err(rt_msg("http.put(url, body, opts?) required")),
    }
}

// ─── http.delete(url, opts?) ───
fn http_delete(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match a.first() {
        Some(DgmValue::Str(url)) => {
            check_url(url)?;
            let opts = parse_opts(&a, 1)?;
            let agent = build_agent(&opts);
            let req = apply_headers(agent.delete(url), &opts);
            execute_request("http.delete", req, None)
        }
        _ => Err(rt_msg("http.delete(url, opts?) required")),
    }
}

// ─── http.request(method, url, body?, opts?) — generic ───
fn http_request(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    let method = match a.get(0) {
        Some(DgmValue::Str(m)) => m.to_uppercase(),
        _ => return Err(rt_msg("http.request(method, url, body?, opts?) required")),
    };
    let url = match a.get(1) {
        Some(DgmValue::Str(u)) => u.clone(),
        _ => return Err(rt_msg("http.request(method, url, body?, opts?) required")),
    };
    check_url(&url)?;

    let (body, opts) = parse_optional_body_and_opts(&a, 2)?;
    let agent = build_agent(&opts);
    let req = apply_headers(agent.request(&method, &url), &opts);
    let req = match body.as_ref() {
        Some(request_body) => with_auto_content_type(req, &opts, request_body),
        None => req,
    };
    execute_request("http.request", req, body.as_ref())
}

// ─── http.serve(port, handler_or_routes) ───
fn http_serve_ctx(interp: &mut Interpreter, a: Vec<DgmValue>, span: &Span) -> Result<DgmValue, DgmError> {
    security::check_net()?;
    let port = match a.get(0) {
        Some(DgmValue::Int(p)) => *p,
        _ => return Err(rt_msg("http.serve(port, handler|routes) required")),
    };
    let handler = a.get(1).cloned().ok_or_else(|| rt_msg("http.serve(port, handler|routes) required"))?;
    let addr = format!("0.0.0.0:{}", port);
    let server = tiny_http::Server::http(&addr).map_err(|e| rt("http.serve", &e))?;
    println!("DGM HTTP Server listening on {}", addr);

    match handler {
        DgmValue::Map(routes) => {
            // Static route map mode (backward compat)
            let routes_map = routes.borrow().clone();
            let not_found_bytes: &[u8] = b"404 Not Found";
            let mut key_buf = String::with_capacity(64);
            loop {
                if let Ok(req) = server.recv() {
                    let path = req.url();
                    let method = req.method().as_str();
                    key_buf.clear();
                    key_buf.push_str(method);
                    key_buf.push(' ');
                    key_buf.push_str(path);
                    let body_ref = routes_map.get(&key_buf)
                        .or_else(|| routes_map.get(path));
                    match body_ref {
                        Some(DgmValue::Str(body)) => {
                            let response = tiny_http::Response::from_data(body.as_bytes())
                                .with_header(tiny_http::Header::from_bytes(
                                    &b"Content-Type"[..], &b"application/json"[..]
                                ).unwrap());
                            let _ = req.respond(response);
                        }
                        _ => {
                            let response = tiny_http::Response::from_data(not_found_bytes);
                            let _ = req.respond(response);
                        }
                    }
                }
            }
        }
        handler @ DgmValue::Function { .. } | handler @ DgmValue::NativeFunction { .. } => {
            // Dynamic handler mode
            loop {
                if let Ok(req) = server.recv() {
                    let mut req_map = HashMap::new();
                    req_map.insert("method".to_string(), DgmValue::Str(req.method().as_str().to_string()));
                    req_map.insert("path".to_string(), DgmValue::Str(req.url().to_string()));
                    // Parse headers
                    let mut headers_map = HashMap::new();
                    for h in req.headers() {
                        headers_map.insert(
                            h.field.as_str().as_str().to_lowercase(),
                            DgmValue::Str(h.value.as_str().to_string()),
                        );
                    }
                    req_map.insert("headers".to_string(), DgmValue::Map(Rc::new(RefCell::new(headers_map))));
                    let req_val = DgmValue::Map(Rc::new(RefCell::new(req_map)));

                    // Call handler
                    match interp.call_value(handler.clone(), vec![req_val], span, None, Some("handler")) {
                        Ok(result) => {
                            let (status, body, content_type) = match result {
                                DgmValue::Str(s) => (200, s, "text/plain"),
                                DgmValue::Map(m) => {
                                    let m_ref = m.borrow();
                                    let status = match m_ref.get("status") {
                                        Some(DgmValue::Int(n)) => *n as i32,
                                        _ => 200,
                                    };
                                    let body = match m_ref.get("body") {
                                        Some(DgmValue::Str(s)) => s.clone(),
                                        Some(v) => format!("{}", v),
                                        None => String::new(),
                                    };
                                    (status, body, "application/json")
                                }
                                other => (200, format!("{}", other), "text/plain"),
                            };
                            let response = tiny_http::Response::from_string(body)
                                .with_status_code(status)
                                .with_header(tiny_http::Header::from_bytes(
                                    &b"Content-Type"[..], content_type.as_bytes()
                                ).unwrap());
                            let _ = req.respond(response);
                        }
                        Err(e) => {
                            let err_body = format!("500 Internal Server Error: {}", e.summary());
                            let response = tiny_http::Response::from_string(err_body)
                                .with_status_code(500);
                            let _ = req.respond(response);
                        }
                    }
                }
            }
        }
        _ => Err(rt_msg("http.serve(port, handler|routes) - second arg must be function or map")),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

    #[test]
    fn test_request_opts_default_timeout_is_applied() {
        let opts = parse_opts(&[], 0).unwrap();
        assert_eq!(opts.timeout_ms, DEFAULT_HTTP_TIMEOUT_MS);
    }

    #[test]
    fn test_request_opts_timeout_override_is_applied() {
        let mut map = HashMap::new();
        map.insert("timeout".to_string(), DgmValue::Int(250));
        let opts = parse_opts_value(&DgmValue::Map(Rc::new(RefCell::new(map)))).unwrap();
        assert_eq!(opts.timeout_ms, 250);
    }
}
