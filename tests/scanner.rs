use rust_lint::lint::{scan_comments, Comment, CommentKind};

fn kinds(src: &str) -> Vec<CommentKind> {
    scan_comments(src).iter().map(|c| c.kind).collect()
}

fn bodies(src: &str) -> Vec<String> {
    scan_comments(src).into_iter().map(|c| c.body).collect()
}

fn count(src: &str) -> usize {
    scan_comments(src).len()
}

fn only(src: &str) -> Comment {
    let mut found = scan_comments(src);
    assert_eq!(found.len(), 1, "expected exactly one comment in {src:?}");
    found.remove(0)
}

#[test]
fn classifies_line_and_block_doc_variants() {
    let src = "// line\n/// doc line\n//! inner doc\n/* block */\n/** doc block */\n/*! inner doc block */\n";
    assert_eq!(
        kinds(src),
        vec![
            CommentKind::Line,
            CommentKind::DocLine,
            CommentKind::DocLine,
            CommentKind::Block,
            CommentKind::DocBlock,
            CommentKind::DocBlock,
        ]
    );
}

#[test]
fn quadruple_slash_and_empty_block_are_not_doc() {
    assert_eq!(kinds("//// not a doc\n"), vec![CommentKind::Line]);
    assert_eq!(kinds("/**/\n"), vec![CommentKind::Block]);
}

#[test]
fn ignores_double_slash_inside_string_literal() {
    let src = "fn f() { let u = \"http://example.com\"; }\n";
    assert_eq!(count(src), 0);
}

#[test]
fn ignores_block_delims_inside_string_literal() {
    let src = "fn f() { let s = \"/* not a comment */\"; }\n";
    assert_eq!(count(src), 0);
}

#[test]
fn ignores_comment_markers_inside_raw_string() {
    let src = "fn f() { let s = r#\"a // b /* c */ d\"#; }\n";
    assert_eq!(count(src), 0);
}

#[test]
fn ignores_markers_inside_byte_and_raw_byte_strings() {
    assert_eq!(count("fn f() { let b = b\"// no\"; }\n"), 0);
    assert_eq!(count("fn f() { let b = br#\"// no /* no */\"#; }\n"), 0);
}

#[test]
fn finds_real_comment_after_a_string_containing_slashes() {
    let src = "fn f() { let s = \"// inside\"; } // outside\n";
    assert_eq!(bodies(src), vec!["outside".to_string()]);
}

#[test]
fn char_literal_holding_a_quote_does_not_break_scanning() {
    let src = "fn f() { let q = '\\''; let s = \"x\"; } // tail\n";
    assert_eq!(bodies(src), vec!["tail".to_string()]);
}

#[test]
fn double_quote_char_literal_is_not_a_string() {
    let src = "fn f() { let dq = '\"'; let n = 1; } // after\n";
    assert_eq!(bodies(src), vec!["after".to_string()]);
}

#[test]
fn lifetime_is_not_treated_as_char_literal() {
    let src = "fn f<'a>(x: &'a str) -> &'a str { x } // trailing\n";
    assert_eq!(bodies(src), vec!["trailing".to_string()]);
}

#[test]
fn nested_block_comment_balances_then_resumes() {
    let src = "/* outer /* inner */ still outer */ // after\n";
    assert_eq!(kinds(src), vec![CommentKind::Block, CommentKind::Line]);
    assert_eq!(bodies(src)[1], "after");
}

#[test]
fn records_one_based_position() {
    let c = only("fn f() {}\n    // indented\n");
    assert_eq!((c.start_line, c.start_col), (2, 5));
}

#[test]
fn strips_delimiters_and_doc_markers_from_body() {
    assert_eq!(only("/// the body\n").body, "the body");
    assert_eq!(only("/*  spaced  */\n").body, "spaced");
}
