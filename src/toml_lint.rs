use toml_edit::{DocumentMut, Item, Table};

use crate::lint::Diagnostic;

const DEP_TABLES: &[&str] = &["dependencies", "dev-dependencies", "build-dependencies"];

pub fn check(src: &str) -> Vec<Diagnostic> {
    let doc = match src.parse::<DocumentMut>() {
        Ok(d) => d,
        Err(e) => {
            let (line, col) = match e.span() {
                Some(s) => line_col(src, s.start),
                None => (1, 1),
            };
            return vec![Diagnostic::new(
                "toml_parse",
                line,
                col,
                "manifest does not parse as TOML",
            )];
        }
    };
    let mut out = Vec::new();
    visit_dep_tables(doc.as_table(), &mut |table| inspect(table, src, &mut out));
    out
}

pub fn fix(src: &str) -> Option<String> {
    let mut doc = src.parse::<DocumentMut>().ok()?;
    sort_dep_tables(doc.as_table_mut());
    let fixed = doc.to_string();
    (fixed != src).then_some(fixed)
}

fn inspect(table: &Table, src: &str, out: &mut Vec<Diagnostic>) {
    let keys: Vec<String> = table.iter().map(|(k, _)| k.to_string()).collect();
    let mut sorted = keys.clone();
    sorted.sort_by_key(|k| k.to_lowercase());
    if keys != sorted {
        let (line, col) = match table.span() {
            Some(s) => line_col(src, s.start),
            None => (1, 1),
        };
        out.push(Diagnostic::new(
            "toml_unsorted_deps",
            line,
            col,
            "dependency table is not sorted alphabetically",
        ));
    }
    for (key, item) in table.iter() {
        if is_wildcard(item) {
            let (line, col) = match item.span() {
                Some(s) => line_col(src, s.start),
                None => (1, 1),
            };
            out.push(Diagnostic::new(
                "toml_wildcard_dep",
                line,
                col,
                format!("dependency `{key}` is pinned to \"*\" — require an explicit version"),
            ));
        }
    }
}

fn is_wildcard(item: &Item) -> bool {
    if item.as_str() == Some("*") {
        return true;
    }
    if let Some(inline) = item.as_value().and_then(|v| v.as_inline_table()) {
        return inline.get("version").and_then(|v| v.as_str()) == Some("*");
    }
    if let Some(t) = item.as_table_like() {
        return t.get("version").and_then(|v| v.as_str()) == Some("*");
    }
    false
}

fn visit_dep_tables(root: &Table, f: &mut impl FnMut(&Table)) {
    for (key, item) in root.iter() {
        if DEP_TABLES.contains(&key) {
            if let Some(t) = item.as_table() {
                f(t);
            }
        } else if key == "target" {
            if let Some(targets) = item.as_table() {
                for (_, sub) in targets.iter() {
                    if let Some(subt) = sub.as_table() {
                        visit_dep_tables(subt, f);
                    }
                }
            }
        }
    }
}

fn sort_dep_tables(root: &mut Table) {
    let target_keys: Vec<String> = root
        .iter()
        .filter(|(k, _)| DEP_TABLES.contains(k) || *k == "target")
        .map(|(k, _)| k.to_string())
        .collect();
    for key in target_keys {
        if DEP_TABLES.contains(&key.as_str()) {
            if let Some(t) = root.get_mut(&key).and_then(Item::as_table_mut) {
                t.sort_values();
            }
        } else if let Some(targets) = root.get_mut(&key).and_then(Item::as_table_mut) {
            let subs: Vec<String> = targets.iter().map(|(k, _)| k.to_string()).collect();
            for sub in subs {
                if let Some(subt) = targets.get_mut(&sub).and_then(Item::as_table_mut) {
                    sort_dep_tables(subt);
                }
            }
        }
    }
}

fn line_col(src: &str, offset: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut col = 1usize;
    for (idx, ch) in src.char_indices() {
        if idx >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}
