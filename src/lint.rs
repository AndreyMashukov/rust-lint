use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub analyzer: &'static str,
    pub line: usize,
    pub col: usize,
    pub message: String,
}

impl Diagnostic {
    pub fn new(analyzer: &'static str, line: usize, col: usize, message: impl Into<String>) -> Self {
        Self {
            analyzer,
            line,
            col,
            message: message.into(),
        }
    }
}

pub trait Analyzer: Sync {
    fn name(&self) -> &'static str;
    fn doc(&self) -> &'static str;
    fn check(&self, file: &SourceFile, out: &mut Vec<Diagnostic>);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentKind {
    Line,
    Block,
    DocLine,
    DocBlock,
}

impl CommentKind {
    pub fn is_doc(self) -> bool {
        matches!(self, CommentKind::DocLine | CommentKind::DocBlock)
    }
}

#[derive(Debug, Clone)]
pub struct Comment {
    pub kind: CommentKind,
    pub start_line: usize,
    pub start_col: usize,
    pub body: String,
}

pub struct SourceFile {
    pub path: PathBuf,
    pub ast: syn::File,
    pub comments: Vec<Comment>,
}

impl SourceFile {
    pub fn parse(path: PathBuf, src: String) -> Result<Self, syn::Error> {
        let ast = syn::parse_file(&src)?;
        let comments = scan_comments(&src);
        Ok(Self { path, ast, comments })
    }

    pub fn is_test_file(&self) -> bool {
        let p = self.path.to_string_lossy().replace('\\', "/");
        p.contains("/tests/")
            || p.starts_with("tests/")
            || self.file_stem().ends_with("_test")
            || self.file_stem().starts_with("test_")
    }

    pub fn is_config_module(&self) -> bool {
        let p = self.path.to_string_lossy().replace('\\', "/");
        p.contains("/config/") || self.file_stem() == "config" || self.file_stem() == "settings"
    }

    pub fn is_clock_module(&self) -> bool {
        matches!(self.file_stem(), "clock" | "time")
    }

    fn file_stem(&self) -> &str {
        let stem = match self.path.file_stem() {
            Some(s) => s,
            None => return "",
        };
        match stem.to_str() {
            Some(s) => s,
            None => "",
        }
    }
}

pub fn span_start(span: proc_macro2::Span) -> (usize, usize) {
    let lc = span.start();
    (lc.line, lc.column + 1)
}

pub fn scan_comments(src: &str) -> Vec<Comment> {
    let bytes = src.as_bytes();
    let n = bytes.len();
    let mut out = Vec::new();
    let mut i = 0;
    let mut line = 1usize;
    let mut col = 1usize;

    macro_rules! bump {
        () => {{
            if bytes[i] == b'\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
            i += 1;
        }};
    }

    while i < n {
        let b = bytes[i];

        if b == b'/' && i + 1 < n && bytes[i + 1] == b'/' {
            let start_line = line;
            let start_col = col;
            let kind = line_comment_kind(bytes, i);
            let body_start = i + 2;
            while i < n && bytes[i] != b'\n' {
                bump!();
            }
            let body = strip_line_body(&src[body_start..line_end_byte(bytes, body_start)]);
            out.push(Comment {
                kind,
                start_line,
                start_col,
                body,
            });
            continue;
        }

        if b == b'/' && i + 1 < n && bytes[i + 1] == b'*' {
            let start_line = line;
            let start_col = col;
            let kind = block_comment_kind(bytes, i);
            let body_start = i + 2;
            let mut depth = 0usize;
            let mut body_end = body_start;
            bump!();
            bump!();
            depth += 1;
            while i < n && depth > 0 {
                if bytes[i] == b'/' && i + 1 < n && bytes[i + 1] == b'*' {
                    bump!();
                    bump!();
                    depth += 1;
                } else if bytes[i] == b'*' && i + 1 < n && bytes[i + 1] == b'/' {
                    body_end = i;
                    bump!();
                    bump!();
                    depth -= 1;
                } else {
                    bump!();
                }
            }
            if depth > 0 {
                body_end = i;
            }
            let body = strip_block_body(&src[body_start..body_end.max(body_start)]);
            out.push(Comment {
                kind,
                start_line,
                start_col,
                body,
            });
            continue;
        }

        if b == b'"' {
            bump!();
            while i < n {
                if bytes[i] == b'\\' {
                    bump!();
                    if i < n {
                        bump!();
                    }
                } else if bytes[i] == b'"' {
                    bump!();
                    break;
                } else {
                    bump!();
                }
            }
            continue;
        }

        if (b == b'r' || b == b'b') && raw_or_byte_string_ahead(bytes, i) {
            skip_raw_or_byte_string(bytes, &mut i, &mut line, &mut col);
            continue;
        }

        if b == b'\'' {
            if is_lifetime_at(bytes, i) {
                bump!();
                continue;
            }
            bump!();
            while i < n {
                if bytes[i] == b'\\' {
                    bump!();
                    if i < n {
                        bump!();
                    }
                } else if bytes[i] == b'\'' {
                    bump!();
                    break;
                } else {
                    bump!();
                }
            }
            continue;
        }

        bump!();
    }

    out
}

fn line_comment_kind(bytes: &[u8], i: usize) -> CommentKind {
    match bytes.get(i + 2).copied() {
        Some(b'/') if bytes.get(i + 3) != Some(&b'/') => CommentKind::DocLine,
        Some(b'!') => CommentKind::DocLine,
        _ => CommentKind::Line,
    }
}

fn block_comment_kind(bytes: &[u8], i: usize) -> CommentKind {
    match bytes.get(i + 2).copied() {
        Some(b'*') if bytes.get(i + 3) != Some(&b'/') => CommentKind::DocBlock,
        Some(b'!') => CommentKind::DocBlock,
        _ => CommentKind::Block,
    }
}

fn line_end_byte(bytes: &[u8], from: usize) -> usize {
    let mut j = from;
    while j < bytes.len() && bytes[j] != b'\n' {
        j += 1;
    }
    j
}

fn strip_line_body(s: &str) -> String {
    s.trim_start_matches(['/', '!']).trim().to_string()
}

fn strip_block_body(s: &str) -> String {
    s.trim_start_matches(['*', '!']).trim().to_string()
}

fn raw_or_byte_string_ahead(bytes: &[u8], i: usize) -> bool {
    let mut j = i;
    if bytes[j] == b'b' {
        j += 1;
        if j < bytes.len() && bytes[j] == b'"' {
            return true;
        }
    }
    if j < bytes.len() && bytes[j] == b'r' {
        j += 1;
        return j < bytes.len() && (bytes[j] == b'"' || bytes[j] == b'#');
    }
    false
}

fn skip_raw_or_byte_string(bytes: &[u8], i: &mut usize, line: &mut usize, col: &mut usize) {
    let n = bytes.len();
    macro_rules! bump {
        () => {{
            if bytes[*i] == b'\n' {
                *line += 1;
                *col = 1;
            } else {
                *col += 1;
            }
            *i += 1;
        }};
    }
    if bytes[*i] == b'b' {
        bump!();
    }
    if *i < n && bytes[*i] == b'"' {
        bump!();
        while *i < n {
            if bytes[*i] == b'\\' {
                bump!();
                if *i < n {
                    bump!();
                }
            } else if bytes[*i] == b'"' {
                bump!();
                break;
            } else {
                bump!();
            }
        }
        return;
    }
    bump!();
    let mut hashes = 0usize;
    while *i < n && bytes[*i] == b'#' {
        hashes += 1;
        bump!();
    }
    if *i < n && bytes[*i] == b'"' {
        bump!();
    }
    while *i < n {
        if bytes[*i] == b'"' {
            let mut k = *i + 1;
            let mut matched = 0;
            while k < n && bytes[k] == b'#' && matched < hashes {
                k += 1;
                matched += 1;
            }
            if matched == hashes {
                while *i < k {
                    bump!();
                }
                break;
            }
        }
        bump!();
    }
}

fn is_lifetime_at(bytes: &[u8], i: usize) -> bool {
    let n = bytes.len();
    let Some(&c1) = bytes.get(i + 1) else { return false };
    if !(c1.is_ascii_alphabetic() || c1 == b'_') {
        return false;
    }
    let mut j = i + 1;
    while j < n && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_') {
        j += 1;
    }
    bytes.get(j) != Some(&b'\'')
}

pub fn path_segments(path: &syn::Path) -> Vec<String> {
    path.segments.iter().map(|s| s.ident.to_string()).collect()
}

pub fn attr_is_test(attr: &syn::Attribute) -> bool {
    let segs = path_segments(attr.path());
    let last = match segs.last() {
        Some(s) => s.as_str(),
        None => return false,
    };
    if last == "test" || last == "rstest" {
        return true;
    }
    if last == "cfg" || last == "cfg_attr" {
        return cfg_mentions_test(attr);
    }
    false
}

fn cfg_mentions_test(attr: &syn::Attribute) -> bool {
    let mut found = false;
    let _ = attr.parse_nested_meta(|m| {
        if m.path.is_ident("test") {
            found = true;
        }
        Ok(())
    });
    found
}

pub fn attrs_mark_test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(attr_is_test)
}
