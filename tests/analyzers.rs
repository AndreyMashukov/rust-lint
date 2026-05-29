use std::path::PathBuf;

use rust_lint::analyzers;
use rust_lint::lint::{Diagnostic, SourceFile};

fn run(name: &str, path: &str, src: &str) -> Vec<Diagnostic> {
    let file = SourceFile::parse(PathBuf::from(path), src.to_string())
        .unwrap_or_else(|e| panic!("fixture for {name} must parse: {e}"));
    let registry = analyzers::all();
    let analyzer = registry
        .iter()
        .find(|a| a.name() == name)
        .unwrap_or_else(|| panic!("no analyzer named {name}"));
    let mut out = Vec::new();
    analyzer.check(&file, &mut out);
    out.sort_by_key(|d| d.line);
    out
}

fn lines(name: &str, path: &str, src: &str) -> Vec<usize> {
    run(name, path, src).into_iter().map(|d| d.line).collect()
}

fn count(name: &str, path: &str, src: &str) -> usize {
    run(name, path, src).len()
}

#[test]
fn no_inline_comment_flags_body_prose_but_not_doc_or_todo() {
    let src = r#"
/// Doc comment is fine.
pub fn f() {
    // narrating the next line
    let x = 1;
    // TODO handle the edge case
    let _ = x;
}
"#;
    assert_eq!(lines("no_inline_comment", "src/lib.rs", src), vec![4]);
}

#[test]
fn no_inline_comment_clean() {
    let src = "/// doc\npub fn f() {\n    let x = 1;\n    let _ = x;\n}\n";
    assert_eq!(count("no_inline_comment", "src/lib.rs", src), 0);
}

#[test]
fn no_allow_attr_flags_allow_and_expect() {
    let src = "#[allow(dead_code)]\npub fn a() {}\n#[expect(unused)]\npub fn b() {}\n";
    assert_eq!(lines("no_allow_attr", "src/lib.rs", src), vec![1, 3]);
}

#[test]
fn no_allow_attr_clean() {
    let src = "#[derive(Debug)]\npub struct S;\n#[inline]\npub fn a() {}\n";
    assert_eq!(count("no_allow_attr", "src/lib.rs", src), 0);
}

#[test]
fn no_env_var_flags_outside_config_only() {
    let src = "pub fn dsn() -> String {\n    std::env::var(\"DSN\").unwrap()\n}\n";
    assert_eq!(lines("no_env_var", "src/db.rs", src), vec![2]);
    assert_eq!(count("no_env_var", "src/config/mod.rs", src), 0);
}

#[test]
fn no_env_branch_flags_eq_and_match() {
    let src = r#"
pub fn pick(env: &str) -> u8 {
    if env == "prod" {
        return 1;
    }
    match env {
        "dev" => 2,
        _ => 3,
    }
}
"#;
    assert_eq!(lines("no_env_branch", "src/lib.rs", src), vec![3, 7]);
}

#[test]
fn no_env_branch_ignores_non_env_comparisons() {
    let src = "pub fn f(attr_name: &str) -> bool {\n    attr_name == \"test\"\n}\n";
    assert_eq!(count("no_env_branch", "src/lib.rs", src), 0);
}

#[test]
fn no_panic_src_flags_macros_and_unwrap_outside_tests() {
    let src = r#"
pub fn f(o: Option<u8>) -> u8 {
    let x = o.unwrap();
    if x == 0 {
        panic!("zero");
    }
    todo!()
}
"#;
    assert_eq!(lines("no_panic_src", "src/lib.rs", src), vec![3, 5, 7]);
}

#[test]
fn no_panic_src_exempts_test_code() {
    let src = r#"
#[cfg(test)]
mod tests {
    #[test]
    fn t() {
        let v: Option<u8> = Some(1);
        assert_eq!(v.unwrap(), 1);
    }
}
"#;
    assert_eq!(count("no_panic_src", "src/lib.rs", src), 0);
}

#[test]
fn no_time_now_flags_clock_reads_outside_clock_module() {
    let src = "pub fn t() -> std::time::Instant {\n    std::time::Instant::now()\n}\n";
    assert_eq!(lines("no_time_now", "src/handler.rs", src), vec![2]);
    assert_eq!(count("no_time_now", "src/clock.rs", src), 0);
}

#[test]
fn no_type_only_assert_flags_weak_assertions_in_tests() {
    let src = r#"
#[cfg(test)]
mod tests {
    #[test]
    fn t() {
        let r: Option<u8> = Some(1);
        assert!(r.is_some());
        assert!(!r.is_none());
        assert_eq!(r, Some(1));
    }
}
"#;
    assert_eq!(lines("no_type_only_assert", "src/lib.rs", src), vec![7, 8]);
}

#[test]
fn no_db_mut_in_test_flags_mutating_sql_in_tests_only() {
    let bad = r#"
#[cfg(test)]
mod tests {
    #[test]
    fn t() {
        let _ = "INSERT INTO users (id) VALUES (1)";
        let _ = "update the user profile please";
    }
}
"#;
    assert_eq!(lines("no_db_mut_in_test", "src/lib.rs", bad), vec![6]);
    let prod = "pub fn q() -> &'static str {\n    \"DELETE FROM users WHERE id = 1\"\n}\n";
    assert_eq!(count("no_db_mut_in_test", "src/repo.rs", prod), 0);
}

#[test]
fn no_robot_doc_flags_tautology_for_public_and_private() {
    let taut = "/// Creates a new user.\npub fn create_user() {}\n";
    assert_eq!(lines("no_robot_doc", "src/lib.rs", taut), vec![2]);

    let real = "/// Creates a user and emails the welcome packet to their inbox.\npub fn create_user() {}\n";
    assert_eq!(count("no_robot_doc", "src/lib.rs", real), 0);

    let private = "/// Creates a new user.\nfn create_user() {}\n";
    assert_eq!(lines("no_robot_doc", "src/lib.rs", private), vec![2]);
}

#[test]
fn no_error_wrap_banality_flags_contentless_wrappers() {
    let src = r#"
pub fn f() -> anyhow::Result<()> {
    do_a().context("failed to read")?;
    do_b().with_context(|| "cannot parse: {e}")?;
    do_c().context(format!("loading course {course_id} for coach {coach_id}"))?;
    Ok(())
}
"#;
    assert_eq!(lines("no_error_wrap_banality", "src/lib.rs", src), vec![3, 4]);
}

#[test]
fn no_error_wrap_banality_flags_anyhow_macro() {
    let src = "pub fn f() -> anyhow::Result<()> {\n    Err(anyhow::anyhow!(\"failed to load: {e}\"))\n}\n";
    assert_eq!(lines("no_error_wrap_banality", "src/lib.rs", src), vec![2]);
}

#[test]
fn no_todo_flags_every_marker_that_opens_a_comment() {
    let src = r#"
// TODO refactor this
// TODO(PROJ-12) refactor this
// FIXME @alice handle the retry
// HACK works for now
// see the TODO list in the wiki for the full backlog
"#;
    assert_eq!(lines("no_todo", "src/lib.rs", src), vec![2, 3, 4, 5]);
}

#[test]
fn no_redundant_if_flags_both_shapes() {
    let stmt = r#"
pub fn a(x: i32) -> bool {
    if x > 0 {
        return true;
    }
    return false;
}
"#;
    assert_eq!(lines("no_redundant_if", "src/lib.rs", stmt), vec![3]);

    let expr = "pub fn b(x: i32) -> bool {\n    if x > 0 { true } else { false }\n}\n";
    assert_eq!(lines("no_redundant_if", "src/lib.rs", expr), vec![2]);
}

#[test]
fn no_redundant_if_clean() {
    let src = "pub fn a(x: i32) -> bool {\n    x > 0\n}\n";
    assert_eq!(count("no_redundant_if", "src/lib.rs", src), 0);
}

#[test]
fn no_dead_guard_flags_known_variants() {
    let src = r#"
pub fn f() -> u8 {
    if let Some(v) = Some(compute()) {
        return v;
    }
    if Ok::<_, ()>(1).is_ok() {
        return 2;
    }
    match None::<u8> {
        Some(x) => x,
        None => 0,
    }
}
"#;
    assert_eq!(lines("no_dead_guard", "src/lib.rs", src), vec![3, 6, 9]);
}

#[test]
fn no_dead_guard_clean() {
    let src = r#"
pub fn f(o: Option<u8>) -> u8 {
    if let Some(v) = o {
        return v;
    }
    0
}
"#;
    assert_eq!(count("no_dead_guard", "src/lib.rs", src), 0);
}

#[test]
fn registry_has_fourteen_uniquely_named_analyzers() {
    let registry = analyzers::all();
    assert_eq!(registry.len(), 14);
    let mut names: Vec<_> = registry.iter().map(|a| a.name()).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), 14, "analyzer names must be unique");
}

#[test]
fn no_silent_fallback_flags_option_and_result_default_methods() {
    let src = r#"
pub fn f(o: Option<String>, r: Result<i32, ()>) -> String {
    let a = o.clone().unwrap_or(String::new());
    let b = o.clone().unwrap_or_else(|| String::new());
    let c = o.clone().unwrap_or_default();
    let d = r.ok_or(()).unwrap_or(0);
    let e = r.map_or(0, |v| v);
    let _ = (a, b, c, d, e);
    String::new()
}
"#;
    assert_eq!(lines("no_silent_fallback", "src/lib.rs", src), vec![3, 4, 5, 6, 6, 7]);
}

#[test]
fn no_silent_fallback_flags_get_or_insert() {
    let src = r#"
pub fn touch(mut o: Option<u8>) -> u8 {
    *o.get_or_insert(42)
}
"#;
    assert_eq!(count("no_silent_fallback", "src/lib.rs", src), 1);
}

#[test]
fn no_silent_fallback_flags_ok_or_else() {
    let src = r#"
pub fn parse(o: Option<i32>) -> Result<i32, String> {
    o.ok_or_else(|| String::from("missing"))
}
"#;
    assert_eq!(count("no_silent_fallback", "src/lib.rs", src), 1);
}

#[test]
fn no_silent_fallback_exempts_test_files() {
    let src = r#"
pub fn fixture() -> i32 {
    Some(1).unwrap_or(0)
}
"#;
    assert_eq!(count("no_silent_fallback", "tests/fallbacks.rs", src), 0);
}

#[test]
fn no_silent_fallback_exempts_cfg_test_modules() {
    let src = r#"
#[cfg(test)]
mod inner {
    pub fn fixture() -> i32 {
        Some(1).unwrap_or(0)
    }
}
"#;
    assert_eq!(count("no_silent_fallback", "src/lib.rs", src), 0);
}

#[test]
fn no_silent_fallback_exempts_test_fns() {
    let src = r#"
#[test]
fn t() {
    let _ = Some(1).unwrap_or(0);
}
"#;
    assert_eq!(count("no_silent_fallback", "src/lib.rs", src), 0);
}

#[test]
fn no_silent_fallback_does_not_flag_unrelated_methods() {
    let src = r#"
pub fn f(o: Option<u8>) -> Option<u8> {
    o.map(|x| x + 1).filter(|x| *x > 0)
}
"#;
    assert_eq!(count("no_silent_fallback", "src/lib.rs", src), 0);
}
