#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use regex::Regex;
use serde::Serialize;
use serde_json::{json, Map, Value};

pub fn repo_tests_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("tests")
}

pub fn fixture_dirs(root: &Path) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = fs::read_dir(root)
        .unwrap()
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            path.is_dir().then_some(path)
        })
        .collect();
    dirs.sort();
    dirs
}

pub fn load_source(case_dir: &Path) -> String {
    fs::read_to_string(case_dir.join("input.dgm")).unwrap()
}

pub fn load_expected(case_dir: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(case_dir.join("expected.json")).unwrap()).unwrap()
}

pub fn fixture_command(case_dir: &Path, default: &str) -> String {
    let command_file = case_dir.join("command.txt");
    if command_file.exists() {
        fs::read_to_string(command_file).unwrap().trim().to_string()
    } else {
        default.to_string()
    }
}

pub fn assert_case(case_dir: &Path, actual: Value) {
    let expected = load_expected(case_dir);
    assert_eq!(
        actual,
        expected,
        "fixture mismatch for {}\nactual:\n{}",
        case_dir.display(),
        serde_json::to_string_pretty(&actual).unwrap()
    );
}

pub fn snapshot_base(source: &str) -> Value {
    json!({
        "version": 1,
        "input": source,
        "tokens": Value::Null,
        "ast": Value::Null,
        "status": 0,
        "stdout": "",
        "stderr": "",
        "error": Value::Null,
    })
}

pub fn with_snapshot_field<T>(source: &str, field: &str, value: T) -> Value
where
    T: Serialize,
{
    let mut snapshot = snapshot_base(source);
    let value = serde_json::to_value(value).unwrap();
    snapshot[field] = if field == "ast" {
        normalize_ast_value(value)
    } else {
        value
    };
    snapshot
}

pub fn parse_error(stderr: &str, case_dir: &Path) -> Value {
    let header_re = Regex::new(r"^\[(E\d{3})\] (.+)$").unwrap();
    let span_re = Regex::new(r"^\s*-->\s+(.+):(\d+):(\d+)$").unwrap();
    let stack_re = Regex::new(r"^\s*at\s+(.+)\s+\((.+):(\d+):(\d+)\)$").unwrap();

    let mut lines = stderr.lines();
    let Some(header) = lines.next() else {
        return Value::Null;
    };
    let Some(caps) = header_re.captures(header) else {
        return Value::Null;
    };

    let remaining: Vec<&str> = lines.collect();
    let span = remaining.iter().find_map(|line| {
        span_re.captures(line).map(|caps| {
            json!({
                "file": normalize_path(&caps[1], case_dir),
                "line": caps[2].parse::<usize>().unwrap(),
                "col": caps[3].parse::<usize>().unwrap(),
            })
        })
    });

    let stack: Vec<Value> = remaining
        .iter()
        .filter_map(|line| {
            stack_re.captures(line).map(|caps| {
                json!({
                    "function": caps[1],
                    "span": {
                        "file": normalize_path(&caps[2], case_dir),
                        "line": caps[3].parse::<usize>().unwrap(),
                        "col": caps[4].parse::<usize>().unwrap(),
                    }
                })
            })
        })
        .collect();

    json!({
        "code": caps[1],
        "message": caps[2],
        "span": span.unwrap_or(Value::Null),
        "stack": stack,
    })
}

pub fn command_snapshot(case_dir: &Path, source: &str, default_command: &str) -> Value {
    let command = fixture_command(case_dir, default_command);
    let output = Command::new(env!("CARGO_BIN_EXE_dgm"))
        .arg(&command)
        .arg("input.dgm")
        .current_dir(case_dir)
        .output()
        .unwrap();
    let stderr = normalize_stderr(&String::from_utf8_lossy(&output.stderr), case_dir);

    json!({
        "version": 1,
        "input": source,
        "tokens": Value::Null,
        "ast": Value::Null,
        "status": output.status.code().unwrap_or(-1),
        "stdout": String::from_utf8_lossy(&output.stdout),
        "stderr": stderr,
        "error": parse_error(&stderr, case_dir),
    })
}

fn normalize_ast_value(value: Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.into_iter().map(normalize_ast_value).collect()),
        Value::Object(mut map) => {
            if let Some(kind) = map.remove("kind") {
                return normalize_ast_value(kind);
            }

            map.remove("span");
            let normalized = map
                .into_iter()
                .map(|(key, value)| (key, normalize_ast_value(value)))
                .collect::<Map<String, Value>>();
            Value::Object(normalized)
        }
        other => other,
    }
}

fn normalize_path(path: &str, case_dir: &Path) -> String {
    let path = Path::new(path);
    if path.is_absolute() {
        let canonical_case_dir =
            std::fs::canonicalize(case_dir).unwrap_or_else(|_| case_dir.to_path_buf());
        if let Ok(relative) = path.strip_prefix(&canonical_case_dir) {
            return relative.display().to_string();
        }
    }
    path.display().to_string()
}

fn normalize_stderr(stderr: &str, case_dir: &Path) -> String {
    let span_re = Regex::new(r"^(\s*-->\s+)(.+):(\d+):(\d+)$").unwrap();
    let stack_re = Regex::new(r"^(\s*at\s+.+\s+\()(.+):(\d+):(\d+)\)$").unwrap();

    stderr
        .lines()
        .map(|line| {
            if let Some(caps) = span_re.captures(line) {
                return format!(
                    "{}{}:{}:{}",
                    &caps[1],
                    normalize_path(&caps[2], case_dir),
                    &caps[3],
                    &caps[4]
                );
            }
            if let Some(caps) = stack_re.captures(line) {
                return format!(
                    "{}{}:{}:{})",
                    &caps[1],
                    normalize_path(&caps[2], case_dir),
                    &caps[3],
                    &caps[4]
                );
            }
            line.to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
        + if stderr.ends_with('\n') { "\n" } else { "" }
}
