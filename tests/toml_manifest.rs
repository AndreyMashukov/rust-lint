use rust_lint::toml_lint;

fn names(src: &str) -> Vec<&'static str> {
    toml_lint::check(src).into_iter().map(|d| d.analyzer).collect()
}

#[test]
fn flags_unsorted_and_wildcard() {
    let src = "[dependencies]\nzzz = \"1\"\naaa = \"*\"\nmmm = \"2\"\n";
    let mut found = names(src);
    found.sort_unstable();
    assert_eq!(found, vec!["toml_unsorted_deps", "toml_wildcard_dep"]);
}

#[test]
fn clean_manifest_has_no_findings() {
    let src = "[dependencies]\naaa = \"1\"\nmmm = \"2\"\nzzz = \"3\"\n";
    assert_eq!(toml_lint::check(src).len(), 0);
}

#[test]
fn flags_wildcard_in_inline_table() {
    let src = "[dependencies]\nserde = { version = \"*\", features = [\"derive\"] }\n";
    assert_eq!(names(src), vec!["toml_wildcard_dep"]);
}

#[test]
fn checks_dev_and_build_dependency_tables() {
    let src = "[dev-dependencies]\nb = \"1\"\na = \"1\"\n\n[build-dependencies]\nz = \"*\"\n";
    let mut found = names(src);
    found.sort_unstable();
    assert_eq!(found, vec!["toml_unsorted_deps", "toml_wildcard_dep"]);
}

#[test]
fn fix_sorts_and_preserves_comments() {
    let src = "[dependencies]\n# pin carefully\nzzz = \"1\"\naaa = \"2\"\n";
    let fixed = toml_lint::fix(src).expect("should rewrite unsorted manifest");
    let aaa = fixed.find("aaa").expect("aaa present");
    let zzz = fixed.find("zzz").expect("zzz present");
    assert!(aaa < zzz, "aaa must sort before zzz:\n{fixed}");
    assert!(fixed.contains("# pin carefully"), "comment must survive:\n{fixed}");
}

#[test]
fn fix_is_idempotent_on_sorted_input() {
    let src = "[dependencies]\naaa = \"1\"\nzzz = \"2\"\n";
    assert_eq!(toml_lint::fix(src), None);
}
