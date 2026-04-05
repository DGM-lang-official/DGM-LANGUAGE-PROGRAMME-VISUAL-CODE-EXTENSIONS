use std::collections::HashMap;
use std::cell::RefCell;
use std::rc::Rc;
use crate::interpreter::{DgmValue, NativeFunction};
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
        ("serve", http_serve),
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
    m
}

// ─── Options helper ───

struct RequestOpts {
    headers: Vec<(String, String)>,
    timeout_ms: Option<u64>,
}

fn parse_opts(a: &[DgmValue], idx: usize) -> RequestOpts {
    let mut opts = RequestOpts { headers: vec![], timeout_ms: None };
    if let Some(DgmValue::Map(m)) = a.get(idx) {
        let map = m.borrow();
        if let Some(DgmValue::Map(h)) = map.get("headers") {
            for (k, v) in h.borrow().iter() {
                let val = match v { DgmValue::Str(s) => s.clone(), other => format!("{}", other) };
                opts.headers.push((k.clone(), val));
            }
        }
        if let Some(DgmValue::Int(ms)) = map.get("timeout") {
            opts.timeout_ms = Some(*ms as u64);
        }
    }
    opts
}

fn build_agent(opts: &RequestOpts) -> ureq::Agent {
    let mut builder = ureq::AgentBuilder::new();
    if let Some(ms) = opts.timeout_ms {
        builder = builder.timeout(std::time::Duration::from_millis(ms));
    }
    builder.build()
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
    let body = resp.into_string().unwrap_or_default();
    let mut map = HashMap::new();
    map.insert("status".into(), DgmValue::Int(status as i64));
    map.insert("body".into(), DgmValue::Str(body));
    map.insert("ok".into(), DgmValue::Bool(status >= 200 && status < 300));
    Ok(DgmValue::Map(Rc::new(RefCell::new(map))))
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

// ─── http.get(url, opts?) ───
fn http_get(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match a.first() {
        Some(DgmValue::Str(url)) => {
            check_url(url)?;
            let opts = parse_opts(&a, 1);
            let agent = build_agent(&opts);
            let req = apply_headers(agent.get(url), &opts);
            let resp = req.call().map_err(|e| rt("http.get", &e))?;
            make_response(resp)
        }
        _ => Err(rt_msg("http.get(url, opts?) required")),
    }
}

// ─── http.post(url, body?, opts?) ───
fn http_post(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match a.get(0) {
        Some(DgmValue::Str(url)) => {
            check_url(url)?;
            let (body, opts_idx) = match a.get(1) {
                Some(DgmValue::Str(b)) => (Some(b.clone()), 2),
                Some(DgmValue::Map(_)) => (None, 1), // opts in position 1, no body
                _ => (None, 2),
            };
            let opts = parse_opts(&a, opts_idx);
            let agent = build_agent(&opts);
            let mut req = apply_headers(agent.post(url), &opts);
            if body.is_some() {
                req = req.set("Content-Type", "application/json");
            }
            let resp = match body {
                Some(b) => req.send_string(&b),
                None => req.call(),
            }.map_err(|e| rt("http.post", &e))?;
            make_response(resp)
        }
        _ => Err(rt_msg("http.post(url, body?, opts?) required")),
    }
}

// ─── http.put(url, body, opts?) ───
fn http_put(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match (a.get(0), a.get(1)) {
        (Some(DgmValue::Str(url)), Some(DgmValue::Str(body))) => {
            check_url(url)?;
            let opts = parse_opts(&a, 2);
            let agent = build_agent(&opts);
            let req = apply_headers(agent.put(url), &opts).set("Content-Type", "application/json");
            let resp = req.send_string(body).map_err(|e| rt("http.put", &e))?;
            make_response(resp)
        }
        _ => Err(rt_msg("http.put(url, body, opts?) required")),
    }
}

// ─── http.delete(url, opts?) ───
fn http_delete(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match a.first() {
        Some(DgmValue::Str(url)) => {
            check_url(url)?;
            let opts = parse_opts(&a, 1);
            let agent = build_agent(&opts);
            let req = apply_headers(agent.delete(url), &opts);
            let resp = req.call().map_err(|e| rt("http.delete", &e))?;
            make_response(resp)
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

    let (body, opts_idx) = match a.get(2) {
        Some(DgmValue::Str(b)) => (Some(b.clone()), 3),
        Some(DgmValue::Null) => (None, 3),
        Some(DgmValue::Map(_)) => (None, 2), // opts in position 2
        _ => (None, 3),
    };

    let opts = parse_opts(&a, opts_idx);
    let agent = build_agent(&opts);
    let mut req = apply_headers(agent.request(&method, &url), &opts);

    if body.is_some() {
        req = req.set("Content-Type", "application/json");
    }

    let resp = match body {
        Some(b) => req.send_string(&b),
        None => req.call(),
    }.map_err(|e| rt("http.request", &e))?;

    make_response(resp)
}

// ─── http.serve(port, routes_map) ───
fn http_serve(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match (a.get(0), a.get(1)) {
        (Some(DgmValue::Int(port)), Some(DgmValue::Map(routes))) => {
            let addr = format!("0.0.0.0:{}", port);
            let server = tiny_http::Server::http(&addr).map_err(|e| rt("http.serve", &e))?;
            println!("DGM HTTP Server listening on {}", addr);
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
        _ => Err(rt_msg("http.serve(port, routes_map) required")),
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
