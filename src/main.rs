use std::path::{Path, PathBuf};
use std::process::ExitCode;

use rust_lint::lint::{Analyzer, Diagnostic, SourceFile};
use rust_lint::{analyzers, toml_lint};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let mut paths: Vec<PathBuf> = Vec::new();
    let mut selected: Vec<String> = Vec::new();
    let mut fix = false;
    let registry = analyzers::all();

    for arg in &args {
        match arg.as_str() {
            "-h" | "--help" => {
                print_help(&registry);
                return ExitCode::SUCCESS;
            }
            "--version" | "-V" => {
                println!("rust-lint {}", env!("CARGO_PKG_VERSION"));
                return ExitCode::SUCCESS;
            }
            "--list" => {
                for a in &registry {
                    println!("{:<22} {}", a.name(), a.doc());
                }
                return ExitCode::SUCCESS;
            }
            "--fix" => fix = true,
            s if s.starts_with("--") => selected.push(s.trim_start_matches('-').to_string()),
            s => paths.push(PathBuf::from(s)),
        }
    }

    if paths.is_empty() {
        paths.push(PathBuf::from("."));
    }

    let enabled: Vec<&dyn Analyzer> = if selected.is_empty() {
        registry.iter().map(|a| &**a).collect()
    } else {
        let mut chosen: Vec<&dyn Analyzer> = Vec::new();
        for name in &selected {
            match registry.iter().find(|a| a.name() == name) {
                Some(a) => chosen.push(&**a),
                None => {
                    eprintln!("rust-lint: unknown analyzer '--{name}' (try --list)");
                    return ExitCode::from(2);
                }
            }
        }
        chosen
    };

    let mut diagnostics: Vec<(PathBuf, Diagnostic)> = Vec::new();
    let mut errors = 0usize;

    for path in collect_files(&paths, "rs") {
        lint_rs_file(&path, &enabled, &mut diagnostics, &mut errors);
    }

    if selected.is_empty() {
        for path in collect_files(&paths, "toml") {
            lint_toml_file(&path, fix, &mut diagnostics, &mut errors);
        }
    }

    diagnostics.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then(a.1.line.cmp(&b.1.line))
            .then(a.1.col.cmp(&b.1.col))
            .then(a.1.analyzer.cmp(b.1.analyzer))
    });

    for (path, d) in &diagnostics {
        println!(
            "{}:{}:{}: [{}] {}",
            path.display(),
            d.line,
            d.col,
            d.analyzer,
            d.message
        );
    }

    if !diagnostics.is_empty() {
        eprintln!("\nrust-lint: {} finding(s)", diagnostics.len());
    }

    if !diagnostics.is_empty() || errors > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn lint_rs_file(path: &Path, enabled: &[&dyn Analyzer], out: &mut Vec<(PathBuf, Diagnostic)>, errors: &mut usize) {
    let src = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("rust-lint: cannot read {}: {e}", path.display());
            *errors += 1;
            return;
        }
    };
    let file = match SourceFile::parse(path.to_path_buf(), src) {
        Ok(f) => f,
        Err(e) => {
            let (l, c) = (e.span().start().line, e.span().start().column + 1);
            eprintln!("{}:{l}:{c}: parse error: {e}", path.display());
            *errors += 1;
            return;
        }
    };
    let mut found = Vec::new();
    for a in enabled {
        a.check(&file, &mut found);
    }
    out.extend(found.into_iter().map(|d| (path.to_path_buf(), d)));
}

fn lint_toml_file(path: &Path, fix: bool, out: &mut Vec<(PathBuf, Diagnostic)>, errors: &mut usize) {
    let mut src = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("rust-lint: cannot read {}: {e}", path.display());
            *errors += 1;
            return;
        }
    };
    if fix {
        if let Some(fixed) = toml_lint::fix(&src) {
            if let Err(e) = std::fs::write(path, &fixed) {
                eprintln!("rust-lint: cannot write {}: {e}", path.display());
                *errors += 1;
            } else {
                eprintln!("rust-lint: sorted dependency tables in {}", path.display());
                src = fixed;
            }
        }
    }
    out.extend(toml_lint::check(&src).into_iter().map(|d| (path.to_path_buf(), d)));
}

fn collect_files(paths: &[PathBuf], ext: &str) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for p in paths {
        if p.is_file() {
            if matches!(p.extension(), Some(e) if e == ext) {
                files.push(p.clone());
            }
            continue;
        }
        for entry in walkdir::WalkDir::new(p)
            .into_iter()
            .filter_entry(|e| !is_ignored(e.path()))
        {
            let Ok(entry) = entry else { continue };
            let path = entry.path();
            if path.is_file() && matches!(path.extension(), Some(e) if e == ext) {
                files.push(path.to_path_buf());
            }
        }
    }
    files.sort();
    files.dedup();
    files
}

fn is_ignored(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|s| s.to_str()),
        Some("target" | ".git" | "node_modules" | ".cargo")
    )
}

fn print_help(registry: &[Box<dyn Analyzer>]) {
    println!(
        "rust-lint {} — block low-signal bloat patterns in Rust\n",
        env!("CARGO_PKG_VERSION")
    );
    println!("USAGE:\n    rust-lint [--<analyzer>...] [--fix] [PATH...]\n");
    println!("With no --<analyzer> flags, all analyzers run over PATH (default: .),");
    println!("including the .toml manifest checks. Passing --<analyzer> flags runs only");
    println!("those (and skips the .toml checks).\n");
    println!("OPTIONS:");
    println!("    --fix            sort dependency tables in .toml files in place");
    println!("    --list           list all analyzers");
    println!("    -h, --help       show this help");
    println!("    -V, --version    show version\n");
    println!("ANALYZERS:");
    for a in registry {
        println!("    --{:<20} {}", a.name(), a.doc());
    }
}
