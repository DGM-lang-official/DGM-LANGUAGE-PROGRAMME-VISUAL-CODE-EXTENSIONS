use crate::error::DgmError;
use crate::interpreter::{DgmValue, NativeFunction};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub fn module() -> HashMap<String, DgmValue> {
    let mut m = HashMap::new();
    let fns: &[(&str, fn(Vec<DgmValue>) -> Result<DgmValue, DgmError>)] = &[
        ("read_file", io_read_file),
        ("write_file", io_write_file),
        ("append_file", io_append_file),
        ("exists", io_exists),
        ("delete", io_delete),
        ("mkdir", io_mkdir),
        ("list_dir", io_list_dir),
        ("cwd", io_cwd),
        ("input", io_input),
        ("read_lines", io_read_lines),
        ("file_size", io_file_size),
        ("is_dir", io_is_dir),
        ("is_file", io_is_file),
        ("rename", io_rename),
        ("copy", io_copy),
        ("abs_path", io_abs_path),
    ];
    for (name, func) in fns {
        m.insert(
            name.to_string(),
            DgmValue::NativeFunction {
                name: format!("io.{}", name),
                func: NativeFunction::simple(*func),
            },
        );
    }
    m
}

fn io_read_file(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match a.first() {
        Some(DgmValue::Str(p)) => std::fs::read_to_string(p)
            .map(DgmValue::Str)
            .map_err(|e| DgmError::runtime(format!("read_file: {}", e))),
        _ => Err(DgmError::runtime("read_file(path) required")),
    }
}

fn io_write_file(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match (a.get(0), a.get(1)) {
        (Some(DgmValue::Str(p)), Some(DgmValue::Str(c))) => std::fs::write(p, c)
            .map(|_| DgmValue::Null)
            .map_err(|e| DgmError::runtime(format!("write_file: {}", e))),
        _ => Err(DgmError::runtime("write_file(path, content) required")),
    }
}

fn io_append_file(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    use std::io::Write;

    match (a.get(0), a.get(1)) {
        (Some(DgmValue::Str(p)), Some(DgmValue::Str(c))) => {
            let mut f = std::fs::OpenOptions::new()
                .append(true)
                .create(true)
                .open(p)
                .map_err(|e| DgmError::runtime(format!("append: {}", e)))?;
            f.write_all(c.as_bytes())
                .map_err(|e| DgmError::runtime(format!("append: {}", e)))?;
            Ok(DgmValue::Null)
        }
        _ => Err(DgmError::runtime("append_file(path, content) required")),
    }
}

fn io_exists(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match a.first() {
        Some(DgmValue::Str(p)) => Ok(DgmValue::Bool(std::path::Path::new(p).exists())),
        _ => Err(DgmError::runtime("exists(path) required")),
    }
}

fn io_delete(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match a.first() {
        Some(DgmValue::Str(p)) => {
            let path = std::path::Path::new(p);
            if path.is_dir() {
                std::fs::remove_dir_all(p)
                    .map_err(|e| DgmError::runtime(format!("delete: {}", e)))?;
            } else {
                std::fs::remove_file(p)
                    .map_err(|e| DgmError::runtime(format!("delete: {}", e)))?;
            }
            Ok(DgmValue::Null)
        }
        _ => Err(DgmError::runtime("delete(path) required")),
    }
}

fn io_mkdir(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match a.first() {
        Some(DgmValue::Str(p)) => std::fs::create_dir_all(p)
            .map(|_| DgmValue::Null)
            .map_err(|e| DgmError::runtime(format!("mkdir: {}", e))),
        _ => Err(DgmError::runtime("mkdir(path) required")),
    }
}

fn io_list_dir(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match a.first() {
        Some(DgmValue::Str(p)) => {
            let entries: Vec<DgmValue> = std::fs::read_dir(p)
                .map_err(|e| DgmError::runtime(format!("list_dir: {}", e)))?
                .filter_map(|entry| {
                    entry
                        .ok()
                        .map(|entry| DgmValue::Str(entry.file_name().to_string_lossy().to_string()))
                })
                .collect();
            Ok(DgmValue::List(Rc::new(RefCell::new(entries))))
        }
        _ => Err(DgmError::runtime("list_dir(path) required")),
    }
}

fn io_cwd(_a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    std::env::current_dir()
        .map(|p| DgmValue::Str(p.to_string_lossy().to_string()))
        .map_err(|e| DgmError::runtime(format!("cwd: {}", e)))
}

fn io_input(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    if let Some(DgmValue::Str(prompt)) = a.first() {
        print!("{}", prompt);
        use std::io::Write;
        std::io::stdout().flush().ok();
    }
    let mut line = String::new();
    std::io::stdin()
        .read_line(&mut line)
        .map_err(|e| DgmError::runtime(format!("input: {}", e)))?;
    Ok(DgmValue::Str(line.trim().to_string()))
}

fn io_read_lines(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match a.first() {
        Some(DgmValue::Str(p)) => {
            let content = std::fs::read_to_string(p)
                .map_err(|e| DgmError::runtime(format!("read_lines: {}", e)))?;
            let lines: Vec<DgmValue> = content
                .lines()
                .map(|line| DgmValue::Str(line.to_string()))
                .collect();
            Ok(DgmValue::List(Rc::new(RefCell::new(lines))))
        }
        _ => Err(DgmError::runtime("read_lines(path) required")),
    }
}

fn io_file_size(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match a.first() {
        Some(DgmValue::Str(p)) => std::fs::metadata(p)
            .map(|m| DgmValue::Int(m.len() as i64))
            .map_err(|e| DgmError::runtime(format!("file_size: {}", e))),
        _ => Err(DgmError::runtime("file_size(path) required")),
    }
}

fn io_is_dir(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match a.first() {
        Some(DgmValue::Str(p)) => Ok(DgmValue::Bool(std::path::Path::new(p).is_dir())),
        _ => Err(DgmError::runtime("is_dir(path) required")),
    }
}

fn io_is_file(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match a.first() {
        Some(DgmValue::Str(p)) => Ok(DgmValue::Bool(std::path::Path::new(p).is_file())),
        _ => Err(DgmError::runtime("is_file(path) required")),
    }
}

fn io_rename(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match (a.get(0), a.get(1)) {
        (Some(DgmValue::Str(from)), Some(DgmValue::Str(to))) => std::fs::rename(from, to)
            .map(|_| DgmValue::Null)
            .map_err(|e| DgmError::runtime(format!("rename: {}", e))),
        _ => Err(DgmError::runtime("rename(from, to) required")),
    }
}

fn io_copy(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match (a.get(0), a.get(1)) {
        (Some(DgmValue::Str(from)), Some(DgmValue::Str(to))) => std::fs::copy(from, to)
            .map(|_| DgmValue::Null)
            .map_err(|e| DgmError::runtime(format!("copy: {}", e))),
        _ => Err(DgmError::runtime("copy(from, to) required")),
    }
}

fn io_abs_path(a: Vec<DgmValue>) -> Result<DgmValue, DgmError> {
    match a.first() {
        Some(DgmValue::Str(p)) => std::fs::canonicalize(p)
            .map(|p| DgmValue::Str(p.to_string_lossy().to_string()))
            .map_err(|e| DgmError::runtime(format!("abs_path: {}", e))),
        _ => Err(DgmError::runtime("abs_path(path) required")),
    }
}
