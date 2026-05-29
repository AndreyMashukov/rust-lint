use std::path::PathBuf;
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_rust-lint");

fn write_temp(slot: &str, content: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rustlint_cli_{}_{slot}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("input.rs");
    std::fs::write(&path, content).expect("write fixture");
    path
}

fn invoke(args: &[&str]) -> (i32, String) {
    let out = Command::new(BIN).args(args).output().expect("run rust-lint");
    let code = out.status.code().expect("exit code");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    (code, stdout)
}

#[test]
fn clean_file_exits_zero_with_no_output() {
    let path = write_temp("clean", "pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n");
    let (code, stdout) = invoke(&[path.to_str().unwrap()]);
    assert_eq!(code, 0);
    assert_eq!(stdout.trim(), "");
}

#[test]
fn violation_exits_one_and_names_the_analyzer() {
    let path = write_temp("violation", "#[allow(dead_code)]\npub fn a() {}\n");
    let (code, stdout) = invoke(&[path.to_str().unwrap()]);
    assert_eq!(code, 1);
    assert!(stdout.contains("no_allow_attr"), "stdout was: {stdout}");
}

#[test]
fn selecting_one_analyzer_silences_the_others() {
    let path = write_temp("selection", "#[allow(dead_code)]\npub fn a() {}\n// TODO unowned\n");
    let (code, stdout) = invoke(&["--no_allow_attr", path.to_str().unwrap()]);
    assert_eq!(code, 1);
    assert!(stdout.contains("no_allow_attr"), "stdout was: {stdout}");
    assert!(!stdout.contains("no_todo"), "no_todo should be filtered out: {stdout}");
}

#[test]
fn list_exits_zero_and_shows_every_analyzer() {
    let (code, stdout) = invoke(&["--list"]);
    assert_eq!(code, 0);
    assert_eq!(stdout.lines().count(), 14);
    assert!(stdout.contains("no_inline_comment"));
    assert!(stdout.contains("no_dead_guard"));
}

#[test]
fn unknown_analyzer_flag_exits_two() {
    let path = write_temp("unknown", "pub fn a() {}\n");
    let (code, _) = invoke(&["--no_such_analyzer", path.to_str().unwrap()]);
    assert_eq!(code, 2);
}
