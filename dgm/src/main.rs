use dgm::interpreter::Interpreter;
use dgm::lexer::Lexer;
use dgm::parser::Parser;
use dgm::{run_named_source, validate_named_source};
use std::sync::Arc;
use std::io::{self, Write};

fn run_file(path: &str) {
    // [B] FILE CONVENTION: Enforce .dgm extension
    if !path.ends_with(".dgm") {
        eprintln!("Error: DGM files must have .dgm extension");
        std::process::exit(1);
    }
    
    match std::fs::read_to_string(path) {
        Ok(source) => {
            if let Err(e) = run_named_source(&source, path) {
                eprint!("{}", e.render(path, &source));
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("Cannot read file '{}': {}", path, e);
            std::process::exit(1);
        }
    }
}

fn validate_file(path: &str) {
    if !path.ends_with(".dgm") {
        eprintln!("Error: DGM files must have .dgm extension");
        std::process::exit(1);
    }

    match std::fs::read_to_string(path) {
        Ok(source) => {
            if let Err(e) = validate_named_source(&source, path) {
                eprint!("{}", e.render(path, &source));
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("Cannot read file '{}': {}", path, e);
            std::process::exit(1);
        }
    }
}

fn run_repl() {
    println!("DGM 0.2.0 — Interactive REPL");
    println!("Type 'exit' to quit, 'help' for commands\n");
    let mut interp = Interpreter::new(Arc::new("<repl>".to_string()));

    let config = rustyline::config::Config::builder()
        .history_ignore_space(true)
        .build();
    let mut rl = rustyline::DefaultEditor::with_config(config).unwrap();
    let history_path = dirs_home().map(|h| format!("{}/.dgm_history", h));
    if let Some(ref path) = history_path { let _ = rl.load_history(path); }

    loop {
        let readline = rl.readline(">>> ");
        match readline {
            Ok(line) => {
                let line = line.trim().to_string();
                if line.is_empty() { continue; }
                if line == "exit" || line == "quit" { break; }
                if line == "help" {
                    println!("DGM REPL commands:");
                    println!("  exit/quit  — exit REPL");
                    println!("  help       — show this help");
                    println!("  .clear     — clear screen");
                    println!("\nAvailable modules: math, io, fs, os, json, time, http, crypto, regex, net, thread, xml");
                    println!("Use: imprt <module>\n");
                    continue;
                }
                if line == ".clear" {
                    print!("\x1B[2J\x1B[1;1H");
                    io::stdout().flush().ok();
                    continue;
                }
                let _ = rl.add_history_entry(&line);

                let mut lexer = Lexer::with_file(&line, Arc::new("<repl>".to_string()));
                let tokens = match lexer.tokenize() {
                    Ok(t) => t,
                    Err(e) => { eprintln!("\x1b[31m{}\x1b[0m", e.render("<repl>", &line).trim_end()); continue; }
                };
                let mut parser = Parser::new(tokens);
                let stmts = match parser.parse() {
                    Ok(s) => s,
                    Err(e) => { eprintln!("\x1b[31m{}\x1b[0m", e.render("<repl>", &line).trim_end()); continue; }
                };
                if let Err(e) = interp.run(stmts) {
                    eprintln!("\x1b[31m{}\x1b[0m", e.render("<repl>", &line).trim_end());
                }
            }
            Err(rustyline::error::ReadlineError::Interrupted) => { println!("^C"); continue; }
            Err(rustyline::error::ReadlineError::Eof) => { break; }
            Err(e) => { eprintln!("Error: {}", e); break; }
        }
    }
    if let Some(ref path) = history_path { let _ = rl.save_history(path); }
}

fn dirs_home() -> Option<String> {
    std::env::var("HOME").ok()
}

fn print_version() {
    println!("DGM Programming Language v0.2.0");
    println!("Created by Dang Gia Minh");
    println!("Built with Rust — tree-walk interpreter");
}

fn print_help() {
    println!("DGM Programming Language v0.2.0\n");
    println!("USAGE:");
    println!("  dgm run <file.dgm>       Run a DGM script");
    println!("  dgm validate <file.dgm>  Validate syntax without executing");
    println!("  dgm repl                 Start interactive REPL");
    println!("  dgm version              Show version info");
    println!("  dgm help                 Show this help\n");
    println!("FILE FORMAT:");
    println!("  All DGM scripts must use .dgm extension\n");
    println!("STABLE MODULES:");
    println!("  math     — math functions (sqrt, sin, cos, random, etc.)");
    println!("  io       — file I/O (read_file, write_file, mkdir, etc.)");
    println!("  fs       — sandboxed filesystem (read, write, append, delete, list)");
    println!("  os       — OS operations (exec, env, platform, sleep)");
    println!("  json     — JSON parse/stringify (optimized)");
    println!("  time     — timestamps and formatting");
    println!("  http     — HTTP client/server (get, post, serve)");
    println!("  crypto   — cryptography (sha256, md5, base64, hmac)");
    println!("  regex    — regular expressions");
    println!("  net      — TCP networking");
    println!("  thread   — thread helpers (sleep, available_cpus)");
    println!("  xml      — XML parse/stringify/query\n");
    println!("EXIT CODES:");
    println!("  0        — Success");
    println!("  1        — Runtime error or file not found\n");
    println!("EXAMPLE:");
    println!("  # hello.dgm");
    println!("  let name = \"world\"");
    println!("  writ(f\"Hello, {{name}}!\")\n");
    println!("See LANGUAGE_SPEC.md and STDLIB_SPEC.md for documentation.");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(|s| s.as_str()) {
        Some("run") => {
            if let Some(path) = args.get(2) {
                run_file(path);
            } else {
                eprintln!("Usage: dgm run <file.dgm>");
                std::process::exit(1);
            }
        }
        Some("validate") => {
            if let Some(path) = args.get(2) {
                validate_file(path);
            } else {
                eprintln!("Usage: dgm validate <file.dgm>");
                std::process::exit(1);
            }
        }
        Some("repl") => run_repl(),
        Some("version") | Some("--version") | Some("-v") => print_version(),
        Some("help") | Some("--help") | Some("-h") => print_help(),
        None => run_repl(),
        Some(arg) => {
            // If arg ends with .dgm, treat as file (shorthand for `run`)
            if arg.ends_with(".dgm") {
                run_file(arg);
            } else {
                eprintln!("Error: unknown command '{}'", arg);
                eprintln!("Use 'dgm help' for usage.");
                std::process::exit(1);
            }
        }
    }
}
