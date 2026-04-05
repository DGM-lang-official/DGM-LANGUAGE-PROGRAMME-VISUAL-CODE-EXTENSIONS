use std::collections::HashMap;
use std::cell::RefCell;
use std::io::{Read, Write};
use std::rc::Rc;
use crate::interpreter::DgmValue;
use crate::error::DgmError;
use super::security;

pub fn module() -> HashMap<String, DgmValue> {
    let mut m = HashMap::new();
    let fns: &[(&str, fn(Vec<DgmValue>) -> Result<DgmValue, DgmError>)] = &[
        ("read", fs_read),
        ("read_bytes", fs_read_bytes),
        ("write", fs_write),
        ("write_bytes", fs_write_bytes),
        ("append", fs_append),
        ("delete", fs_delete),
        ("exists", fs_exists),
        ("list", fs_list),
        ("mkdir", fs_mkdir),
        ("rmdir", fs_rmdir),
        ("rename", fs_rename),
        ("copy", fs_copy),
        ("size", fs_size),
        ("is_file", fs_is_file),
        ("is_dir", fs_is_dir),
        ("metadata", fs_metadata),
    ];
    for (name, func) in fns {
        m.insert(
            name.to_string(),
            DgmValue::NativeFunction {
                name: format!("fs.{}", name),
                func: *func,
            },
        );
    }
    m
}

fn fs_read(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    let path = get_path(&a, 0, "fs.read(path)")?;
    let resolved = security::resolve_sandboxed_path(&path)?;
    let mut file = std::fs::File::open(&resolved)
        .map_err(|e| rt_err("fs.read", &e))?;
    let mut buf = String::new();
    std::io::BufReader::new(&mut file)
        .read_to_string(&mut buf)
        .map_err(|e| rt_err("fs.read", &e))?;
    Ok(DgmValue::Str(buf))
}

fn fs_read_bytes(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    let path = get_path(&a, 0, "fs.read_bytes(path)")?;
    let resolved = security::resolve_sandboxed_path(&path)?;
    let bytes = std::fs::read(&resolved)
        .map_err(|e| rt_err("fs.read_bytes", &e))?;
    let vals: Vec<DgmValue> = bytes.into_iter().map(|b| DgmValue::Int(b as i64)).collect();
    Ok(DgmValue::List(Rc::new(RefCell::new(vals))))
}

fn fs_write(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    let path = get_path(&a, 0, "fs.write(path, data)")?;
    let data = get_str(&a, 1, "fs.write(path, data)")?;
    let resolved = security::resolve_sandboxed_path(&path)?;
    let file = std::fs::File::create(&resolved)
        .map_err(|e| rt_err("fs.write", &e))?;
    let mut writer = std::io::BufWriter::new(file);
    writer.write_all(data.as_bytes())
        .map_err(|e| rt_err("fs.write", &e))?;
    writer.flush().map_err(|e| rt_err("fs.write", &e))?;
    Ok(DgmValue::Null)
}

fn fs_write_bytes(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    let path = get_path(&a, 0, "fs.write_bytes(path, bytes_list)")?;
    let resolved = security::resolve_sandboxed_path(&path)?;
    let bytes = get_byte_list(&a, 1, "fs.write_bytes")?;
    let file = std::fs::File::create(&resolved)
        .map_err(|e| rt_err("fs.write_bytes", &e))?;
    let mut writer = std::io::BufWriter::new(file);
    writer.write_all(&bytes)
        .map_err(|e| rt_err("fs.write_bytes", &e))?;
    writer.flush().map_err(|e| rt_err("fs.write_bytes", &e))?;
    Ok(DgmValue::Null)
}

fn fs_append(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    let path = get_path(&a, 0, "fs.append(path, data)")?;
    let data = get_str(&a, 1, "fs.append(path, data)")?;
    let resolved = security::resolve_sandboxed_path(&path)?;
    let file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(&resolved)
        .map_err(|e| rt_err("fs.append", &e))?;
    let mut writer = std::io::BufWriter::new(file);
    writer.write_all(data.as_bytes())
        .map_err(|e| rt_err("fs.append", &e))?;
    writer.flush().map_err(|e| rt_err("fs.append", &e))?;
    Ok(DgmValue::Null)
}

fn fs_delete(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    let path = get_path(&a, 0, "fs.delete(path)")?;
    let resolved = security::resolve_sandboxed_path(&path)?;
    if resolved.is_dir() {
        std::fs::remove_dir_all(&resolved)
            .map_err(|e| rt_err("fs.delete", &e))?;
    } else {
        std::fs::remove_file(&resolved)
            .map_err(|e| rt_err("fs.delete", &e))?;
    }
    Ok(DgmValue::Bool(true))
}

fn fs_exists(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    let path = get_path(&a, 0, "fs.exists(path)")?;
    let resolved = security::resolve_sandboxed_path(&path)?;
    Ok(DgmValue::Bool(resolved.exists()))
}

fn fs_list(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    let path = get_path(&a, 0, "fs.list(path)")?;
    let resolved = security::resolve_sandboxed_path(&path)?;
    let entries = std::fs::read_dir(&resolved)
        .map_err(|e| rt_err("fs.list", &e))?;
    let mut items: Vec<DgmValue> = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| rt_err("fs.list", &e))?;
        items.push(DgmValue::Str(
            entry.file_name().to_string_lossy().into_owned(),
        ));
    }
    Ok(DgmValue::List(Rc::new(RefCell::new(items))))
}

fn fs_mkdir(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    let path = get_path(&a, 0, "fs.mkdir(path)")?;
    let resolved = security::resolve_sandboxed_path(&path)?;
    std::fs::create_dir_all(&resolved)
        .map_err(|e| rt_err("fs.mkdir", &e))?;
    Ok(DgmValue::Null)
}

fn fs_rmdir(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    let path = get_path(&a, 0, "fs.rmdir(path)")?;
    let resolved = security::resolve_sandboxed_path(&path)?;
    std::fs::remove_dir_all(&resolved)
        .map_err(|e| rt_err("fs.rmdir", &e))?;
    Ok(DgmValue::Null)
}

fn fs_rename(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    let from = get_path(&a, 0, "fs.rename(from, to)")?;
    let to = get_path(&a, 1, "fs.rename(from, to)")?;
    let res_from = security::resolve_sandboxed_path(&from)?;
    let res_to = security::resolve_sandboxed_path(&to)?;
    std::fs::rename(&res_from, &res_to)
        .map_err(|e| rt_err("fs.rename", &e))?;
    Ok(DgmValue::Null)
}

fn fs_copy(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    let from = get_path(&a, 0, "fs.copy(from, to)")?;
    let to = get_path(&a, 1, "fs.copy(from, to)")?;
    let res_from = security::resolve_sandboxed_path(&from)?;
    let res_to = security::resolve_sandboxed_path(&to)?;
    let bytes_copied = std::fs::copy(&res_from, &res_to)
        .map_err(|e| rt_err("fs.copy", &e))?;
    Ok(DgmValue::Int(bytes_copied as i64))
}

fn fs_size(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    let path = get_path(&a, 0, "fs.size(path)")?;
    let resolved = security::resolve_sandboxed_path(&path)?;
    let meta = std::fs::metadata(&resolved)
        .map_err(|e| rt_err("fs.size", &e))?;
    Ok(DgmValue::Int(meta.len() as i64))
}

fn fs_is_file(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    let path = get_path(&a, 0, "fs.is_file(path)")?;
    let resolved = security::resolve_sandboxed_path(&path)?;
    Ok(DgmValue::Bool(resolved.is_file()))
}

fn fs_is_dir(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    let path = get_path(&a, 0, "fs.is_dir(path)")?;
    let resolved = security::resolve_sandboxed_path(&path)?;
    Ok(DgmValue::Bool(resolved.is_dir()))
}

fn fs_metadata(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    let path = get_path(&a, 0, "fs.metadata(path)")?;
    let resolved = security::resolve_sandboxed_path(&path)?;
    let meta = std::fs::metadata(&resolved)
        .map_err(|e| rt_err("fs.metadata", &e))?;
    let mut map = HashMap::new();
    map.insert("size".into(), DgmValue::Int(meta.len() as i64));
    map.insert("is_file".into(), DgmValue::Bool(meta.is_file()));
    map.insert("is_dir".into(), DgmValue::Bool(meta.is_dir()));
    map.insert("readonly".into(), DgmValue::Bool(meta.permissions().readonly()));
    if let Ok(modified) = meta.modified() {
        if let Ok(dur) = modified.duration_since(std::time::UNIX_EPOCH) {
            map.insert("modified".into(), DgmValue::Int(dur.as_secs() as i64));
        }
    }
    Ok(DgmValue::Map(Rc::new(RefCell::new(map))))
}

// ─── Helpers ───

#[inline]
fn get_path(a: &[DgmValue], idx: usize, ctx: &str) -> Result<String, DgmError> {
    match a.get(idx) {
        Some(DgmValue::Str(s)) => Ok(s.clone()),
        _ => Err(DgmError::RuntimeError {
            msg: format!("{} required", ctx),
        }),
    }
}

#[inline]
fn get_str(a: &[DgmValue], idx: usize, ctx: &str) -> Result<String, DgmError> {
    match a.get(idx) {
        Some(DgmValue::Str(s)) => Ok(s.clone()),
        Some(v) => Ok(format!("{}", v)),
        _ => Err(DgmError::RuntimeError {
            msg: format!("{} required", ctx),
        }),
    }
}

fn get_byte_list(a: &[DgmValue], idx: usize, ctx: &str) -> Result<Vec<u8>, DgmError> {
    match a.get(idx) {
        Some(DgmValue::List(l)) => {
            let items = l.borrow();
            let mut bytes = Vec::with_capacity(items.len());
            for item in items.iter() {
                match item {
                    DgmValue::Int(n) => bytes.push(*n as u8),
                    _ => return Err(DgmError::RuntimeError {
                        msg: format!("{}: list must contain ints (0-255)", ctx),
                    }),
                }
            }
            Ok(bytes)
        }
        _ => Err(DgmError::RuntimeError {
            msg: format!("{}: byte list required", ctx),
        }),
    }
}

#[inline]
fn rt_err(ctx: &str, e: &dyn std::fmt::Display) -> DgmError {
    DgmError::RuntimeError {
        msg: format!("{}: {}", ctx, e),
    }
}
