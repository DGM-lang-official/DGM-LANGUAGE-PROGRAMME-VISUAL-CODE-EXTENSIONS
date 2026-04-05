use std::collections::HashMap;
use std::cell::RefCell;
use std::rc::Rc;
use crate::interpreter::DgmValue;
use crate::error::DgmError;
use quick_xml::events::{Event, BytesStart, BytesEnd, BytesText};
use quick_xml::Reader;
use quick_xml::Writer;

const MAX_DEPTH: usize = 64;

pub fn module() -> HashMap<String, DgmValue> {
    let mut m = HashMap::new();
    let fns: &[(&str, fn(Vec<DgmValue>) -> Result<DgmValue, DgmError>)] = &[
        ("parse", xml_parse),
        ("stringify", xml_stringify),
    ];
    for (name, func) in fns {
        m.insert(
            name.to_string(),
            DgmValue::NativeFunction {
                name: format!("xml.{}", name),
                func: *func,
            },
        );
    }
    m
}

// ─── xml.parse(str) → Map ───
// Returns: { "tag": Str, "attrs": Map, "children": List, "text": Str|Null }

fn xml_parse(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match a.first() {
        Some(DgmValue::Str(s)) => {
            let mut reader = Reader::from_str(s);
            reader.config_mut().trim_text(true);
            let result = parse_element(&mut reader, 0)?;
            match result {
                Some(v) => Ok(v),
                None => Err(rt("xml.parse: empty or invalid XML")),
            }
        }
        _ => Err(rt("xml.parse(str) required")),
    }
}

fn parse_element(reader: &mut Reader<&[u8]>, depth: usize) -> Result<Option<DgmValue>, DgmError> {
    if depth > MAX_DEPTH {
        return Err(rt("xml.parse: max depth exceeded (64)"));
    }
    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let attrs = parse_attrs(e)?;
                let mut children: Vec<DgmValue> = Vec::new();
                let mut text_buf = String::new();

                // Read children until matching end tag
                loop {
                    match reader.read_event() {
                        Ok(Event::Start(ref inner)) => {
                            // Push back by constructing a sub-parse
                            let child_tag = String::from_utf8_lossy(inner.name().as_ref()).to_string();
                            let child_attrs = parse_attrs(inner)?;
                            let child = parse_children_from_start(reader, &child_tag, child_attrs, depth + 1)?;
                            children.push(child);
                        }
                        Ok(Event::Empty(ref inner)) => {
                            let child_tag = String::from_utf8_lossy(inner.name().as_ref()).to_string();
                            let child_attrs = parse_attrs(inner)?;
                            children.push(make_node(&child_tag, child_attrs, vec![], None));
                        }
                        Ok(Event::Text(ref t)) => {
                            let decoded = t.unescape().map_err(|e| rt(&format!("xml.parse: {}", e)))?;
                            text_buf.push_str(&decoded);
                        }
                        Ok(Event::CData(ref t)) => {
                            let decoded = String::from_utf8_lossy(t.as_ref());
                            text_buf.push_str(&decoded);
                        }
                        Ok(Event::End(_)) => break,
                        Ok(Event::Eof) => return Err(rt("xml.parse: unexpected EOF")),
                        Err(e) => return Err(rt(&format!("xml.parse: {}", e))),
                        _ => {}
                    }
                }
                let text = if text_buf.is_empty() { None } else { Some(text_buf) };
                return Ok(Some(make_node(&tag, attrs, children, text)));
            }
            Ok(Event::Empty(ref e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let attrs = parse_attrs(e)?;
                return Ok(Some(make_node(&tag, attrs, vec![], None)));
            }
            Ok(Event::Eof) => return Ok(None),
            Ok(_) => continue, // skip comments, PI, decl
            Err(e) => return Err(rt(&format!("xml.parse: {}", e))),
        }
    }
}

fn parse_children_from_start(
    reader: &mut Reader<&[u8]>,
    tag: &str,
    attrs: HashMap<String, DgmValue>,
    depth: usize,
) -> Result<DgmValue, DgmError> {
    if depth > MAX_DEPTH {
        return Err(rt("xml.parse: max depth exceeded (64)"));
    }
    let mut children: Vec<DgmValue> = Vec::new();
    let mut text_buf = String::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref inner)) => {
                let child_tag = String::from_utf8_lossy(inner.name().as_ref()).to_string();
                let child_attrs = parse_attrs(inner)?;
                let child = parse_children_from_start(reader, &child_tag, child_attrs, depth + 1)?;
                children.push(child);
            }
            Ok(Event::Empty(ref inner)) => {
                let child_tag = String::from_utf8_lossy(inner.name().as_ref()).to_string();
                let child_attrs = parse_attrs(inner)?;
                children.push(make_node(&child_tag, child_attrs, vec![], None));
            }
            Ok(Event::Text(ref t)) => {
                let decoded = t.unescape().map_err(|e| rt(&format!("xml.parse: {}", e)))?;
                text_buf.push_str(&decoded);
            }
            Ok(Event::CData(ref t)) => {
                let decoded = String::from_utf8_lossy(t.as_ref());
                text_buf.push_str(&decoded);
            }
            Ok(Event::End(_)) => break,
            Ok(Event::Eof) => return Err(rt("xml.parse: unexpected EOF")),
            Err(e) => return Err(rt(&format!("xml.parse: {}", e))),
            _ => {}
        }
    }
    let text = if text_buf.is_empty() { None } else { Some(text_buf) };
    Ok(make_node(tag, attrs, children, text))
}

fn parse_attrs(e: &BytesStart) -> Result<HashMap<String, DgmValue>, DgmError> {
    let mut attrs = HashMap::new();
    for attr in e.attributes() {
        let attr = attr.map_err(|e| rt(&format!("xml.parse attr: {}", e)))?;
        let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
        let val = String::from_utf8_lossy(&attr.value).to_string();
        attrs.insert(key, DgmValue::Str(val));
    }
    Ok(attrs)
}

fn make_node(
    tag: &str,
    attrs: HashMap<String, DgmValue>,
    children: Vec<DgmValue>,
    text: Option<String>,
) -> DgmValue {
    let mut map = HashMap::new();
    map.insert("tag".into(), DgmValue::Str(tag.to_string()));
    map.insert("attrs".into(), DgmValue::Map(Rc::new(RefCell::new(attrs))));
    map.insert("children".into(), DgmValue::List(Rc::new(RefCell::new(children))));
    map.insert(
        "text".into(),
        text.map(DgmValue::Str).unwrap_or(DgmValue::Null),
    );
    DgmValue::Map(Rc::new(RefCell::new(map)))
}

// ─── xml.stringify(map) → Str ───

fn xml_stringify(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match a.first() {
        Some(node @ DgmValue::Map(_)) => {
            let mut buf: Vec<u8> = Vec::with_capacity(256);
            write_node(&mut buf, node, 0)?;
            let s = unsafe { String::from_utf8_unchecked(buf) };
            Ok(DgmValue::Str(s))
        }
        _ => Err(rt("xml.stringify(map) required")),
    }
}

fn write_node(buf: &mut Vec<u8>, val: &DgmValue, depth: usize) -> Result<(), DgmError> {
    if depth > MAX_DEPTH {
        return Err(rt("xml.stringify: max depth exceeded (64)"));
    }
    let map = match val {
        DgmValue::Map(m) => m.borrow(),
        _ => return Err(rt("xml.stringify: node must be map")),
    };

    let tag = match map.get("tag") {
        Some(DgmValue::Str(s)) => s.clone(),
        _ => return Err(rt("xml.stringify: node missing 'tag' string")),
    };

    // Open tag
    buf.push(b'<');
    buf.extend_from_slice(tag.as_bytes());

    // Attrs
    if let Some(DgmValue::Map(attrs)) = map.get("attrs") {
        for (k, v) in attrs.borrow().iter() {
            buf.push(b' ');
            buf.extend_from_slice(k.as_bytes());
            buf.extend_from_slice(b"=\"");
            let val_str = match v {
                DgmValue::Str(s) => s.clone(),
                other => format!("{}", other),
            };
            // Escape attr value
            for byte in val_str.as_bytes() {
                match byte {
                    b'"' => buf.extend_from_slice(b"&quot;"),
                    b'&' => buf.extend_from_slice(b"&amp;"),
                    b'<' => buf.extend_from_slice(b"&lt;"),
                    _ => buf.push(*byte),
                }
            }
            buf.push(b'"');
        }
    }

    // Check for children or text
    let has_children = match map.get("children") {
        Some(DgmValue::List(l)) => !l.borrow().is_empty(),
        _ => false,
    };
    let text = match map.get("text") {
        Some(DgmValue::Str(s)) => Some(s.clone()),
        _ => None,
    };

    if !has_children && text.is_none() {
        buf.extend_from_slice(b"/>");
        return Ok(());
    }

    buf.push(b'>');

    // Text
    if let Some(ref t) = text {
        for byte in t.as_bytes() {
            match byte {
                b'&' => buf.extend_from_slice(b"&amp;"),
                b'<' => buf.extend_from_slice(b"&lt;"),
                b'>' => buf.extend_from_slice(b"&gt;"),
                _ => buf.push(*byte),
            }
        }
    }

    // Children
    if let Some(DgmValue::List(children)) = map.get("children") {
        for child in children.borrow().iter() {
            write_node(buf, child, depth + 1)?;
        }
    }

    // Close tag
    buf.extend_from_slice(b"</");
    buf.extend_from_slice(tag.as_bytes());
    buf.push(b'>');

    Ok(())
}

#[inline]
fn rt(msg: &str) -> DgmError {
    DgmError::RuntimeError { msg: msg.into() }
}
