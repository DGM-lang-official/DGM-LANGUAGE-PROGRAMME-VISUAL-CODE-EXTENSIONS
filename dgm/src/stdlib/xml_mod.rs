use std::collections::HashMap;
use std::cell::RefCell;
use std::rc::Rc;
use crate::interpreter::{DgmValue, NativeFunction};
use crate::error::DgmError;
use quick_xml::events::{Event, BytesStart};
use quick_xml::Reader;

const MAX_DEPTH: usize = 64;

pub fn module() -> HashMap<String, DgmValue> {
    let mut m = HashMap::new();
    let fns: &[(&str, fn(Vec<DgmValue>) -> Result<DgmValue, DgmError>)] = &[
        ("parse", xml_parse),
        ("stringify", xml_stringify),
        ("query", xml_query),
    ];
    for (name, func) in fns {
        m.insert(
            name.to_string(),
            DgmValue::NativeFunction {
                name: format!("xml.{}", name),
                func: NativeFunction::simple(*func),
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
                        Ok(Event::End(ref end)) => {
                            let end_tag = String::from_utf8_lossy(end.name().as_ref()).into_owned();
                            if end_tag != tag {
                                return Err(rt(&format!(
                                    "xml.parse: mismatched closing tag </{}> for <{}>",
                                    end_tag, tag
                                )));
                            }
                            break;
                        }
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
            Ok(Event::End(ref end)) => {
                let end_tag = String::from_utf8_lossy(end.name().as_ref()).into_owned();
                if end_tag != tag {
                    return Err(rt(&format!(
                        "xml.parse: mismatched closing tag </{}> for <{}>",
                        end_tag, tag
                    )));
                }
                break;
            }
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

// ─── xml.query(node, path) → Map | Null ───

fn xml_query(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match (a.first(), a.get(1)) {
        (Some(node @ DgmValue::Map(_)), Some(DgmValue::Str(path))) => {
            let mut current = node.clone();
            let mut segments: Vec<&str> = path.split('.').filter(|segment| !segment.is_empty()).collect();

            if segments.is_empty() {
                return Ok(current);
            }

            if node_tag(&current).as_deref() == Some(segments[0]) {
                segments.remove(0);
            }

            for segment in segments {
                let Some(next) = find_child_by_tag(&current, segment) else {
                    return Ok(DgmValue::Null);
                };
                current = next;
            }

            Ok(current)
        }
        _ => Err(rt("xml.query(node, path) required")),
    }
}

fn node_tag(node: &DgmValue) -> Option<String> {
    match node {
        DgmValue::Map(map) => match map.borrow().get("tag") {
            Some(DgmValue::Str(tag)) => Some(tag.clone()),
            _ => None,
        },
        _ => None,
    }
}

fn find_child_by_tag(node: &DgmValue, tag: &str) -> Option<DgmValue> {
    match node {
        DgmValue::Map(map) => match map.borrow().get("children") {
            Some(DgmValue::List(children)) => children.borrow().iter().find_map(|child| {
                if node_tag(child).as_deref() == Some(tag) {
                    Some(child.clone())
                } else {
                    None
                }
            }),
            _ => None,
        },
        _ => None,
    }
}

#[inline]
fn rt(msg: &str) -> DgmError {
    DgmError::runtime(msg)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_doc(xml: &str) -> DgmValue {
        xml_parse(vec![DgmValue::Str(xml.into())]).unwrap()
    }

    #[test]
    fn parse_and_stringify_preserve_basic_structure() {
        let doc = parse_doc(r#"<root lang="en"><item id="1">hello</item><empty/></root>"#);
        match &doc {
            DgmValue::Map(node) => {
                let map = node.borrow();
                assert!(matches!(map.get("tag"), Some(DgmValue::Str(tag)) if tag == "root"));
                match map.get("attrs") {
                    Some(DgmValue::Map(attrs)) => {
                        assert!(matches!(attrs.borrow().get("lang"), Some(DgmValue::Str(lang)) if lang == "en"));
                    }
                    other => panic!("expected attrs map, got {:?}", other),
                }
                match map.get("children") {
                    Some(DgmValue::List(children)) => assert_eq!(children.borrow().len(), 2),
                    other => panic!("expected children list, got {:?}", other),
                }
            }
            other => panic!("expected root node map, got {}", other),
        }

        let xml = xml_stringify(vec![doc]).unwrap();
        match xml {
            DgmValue::Str(text) => {
                assert!(text.contains(r#"<root lang="en">"#));
                assert!(text.contains(r#"<item id="1">hello</item>"#));
                assert!(text.contains("<empty/>"));
            }
            other => panic!("expected string, got {}", other),
        }
    }

    #[test]
    fn query_returns_root_and_nested_child() {
        let doc = parse_doc("<root><item>hello</item><other>world</other></root>");

        let root = xml_query(vec![doc.clone(), DgmValue::Str("root".into())]).unwrap();
        assert!(matches!(root, DgmValue::Map(_)));

        let item = xml_query(vec![doc, DgmValue::Str("root.item".into())]).unwrap();
        match item {
            DgmValue::Map(node) => {
                let map = node.borrow();
                assert!(matches!(map.get("tag"), Some(DgmValue::Str(tag)) if tag == "item"));
                assert!(matches!(map.get("text"), Some(DgmValue::Str(text)) if text == "hello"));
            }
            other => panic!("expected xml node map, got {}", other),
        }
    }

    #[test]
    fn query_returns_null_for_missing_path() {
        let doc = parse_doc("<root><item>hello</item></root>");
        let result = xml_query(vec![doc, DgmValue::Str("root.missing".into())]).unwrap();
        assert!(matches!(result, DgmValue::Null));
    }

    #[test]
    fn parse_rejects_mismatched_closing_tags() {
        let result = xml_parse(vec![DgmValue::Str("<root><item></root>".into())]);
        assert!(result.is_err());
    }
}
