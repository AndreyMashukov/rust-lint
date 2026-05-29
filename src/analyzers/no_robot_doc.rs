use syn::visit::{self, Visit};

use crate::lint::{span_start, Analyzer, Diagnostic, SourceFile};

pub struct NoRobotDoc;

const STOP_WORDS: &[&str] = &[
    "a", "an", "the", "of", "to", "for", "with", "from", "and", "or", "is", "returns", "return", "gets", "get", "sets",
    "set", "this", "function", "it", "new", "that", "given", "based", "using", "into", "as", "by", "on", "in", "value",
    "instance",
];

impl Analyzer for NoRobotDoc {
    fn name(&self) -> &'static str {
        "no_robot_doc"
    }
    fn doc(&self) -> &'static str {
        "forbids doc comments that tautologically restate the function name — describe behavior or omit"
    }
    fn check(&self, file: &SourceFile, out: &mut Vec<Diagnostic>) {
        let mut v = Collector { out, name: self.name() };
        v.visit_file(&file.ast);
    }
}

struct Collector<'a> {
    out: &'a mut Vec<Diagnostic>,
    name: &'static str,
}

impl<'a> Collector<'a> {
    fn check_fn(&mut self, ident: &syn::Ident, attrs: &[syn::Attribute]) {
        let Some(doc) = extract_doc(attrs) else { return };
        if is_tautological(&ident.to_string(), &doc) {
            let (line, col) = span_start(ident.span());
            self.out.push(Diagnostic::new(
                self.name,
                line,
                col,
                "doc comment tautologically restates the function name — describe behavior or omit",
            ));
        }
    }
}

impl<'ast, 'a> Visit<'ast> for Collector<'a> {
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        self.check_fn(&node.sig.ident, &node.attrs);
        visit::visit_item_fn(self, node);
    }
    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        self.check_fn(&node.sig.ident, &node.attrs);
        visit::visit_impl_item_fn(self, node);
    }
    fn visit_trait_item_fn(&mut self, node: &'ast syn::TraitItemFn) {
        self.check_fn(&node.sig.ident, &node.attrs);
        visit::visit_trait_item_fn(self, node);
    }
}

fn extract_doc(attrs: &[syn::Attribute]) -> Option<String> {
    let mut parts = Vec::new();
    for attr in attrs {
        if !attr.path().is_ident("doc") {
            continue;
        }
        if let syn::Meta::NameValue(nv) = &attr.meta {
            if let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(s), ..
            }) = &nv.value
            {
                parts.push(s.value());
            }
        }
    }
    if parts.is_empty() {
        return None;
    }
    let joined = parts.join(" ").trim().to_string();
    if joined.is_empty() {
        None
    } else {
        Some(joined)
    }
}

fn words(s: &str) -> Vec<String> {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(|w| w.to_lowercase())
        .collect()
}

fn ident_tokens(name: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    for part in name.split('_').filter(|p| !p.is_empty()) {
        let mut current = String::new();
        for (i, ch) in part.char_indices() {
            if i > 0 && ch.is_uppercase() && !current.is_empty() {
                tokens.push(std::mem::take(&mut current).to_lowercase());
            }
            current.push(ch);
        }
        if !current.is_empty() {
            tokens.push(current.to_lowercase());
        }
    }
    tokens
}

fn is_inflection(base: &str, w: &str) -> bool {
    if base == w {
        return true;
    }
    let forms = [
        format!("{base}s"),
        format!("{base}es"),
        format!("{base}ed"),
        format!("{base}d"),
        format!("{base}ing"),
    ];
    if forms.iter().any(|f| f == w) {
        return true;
    }
    if let Some(stem) = base.strip_suffix('e') {
        if format!("{stem}ing") == w {
            return true;
        }
    }
    if let Some(stem) = base.strip_suffix('y') {
        if format!("{stem}ies") == w {
            return true;
        }
    }
    false
}

fn is_tautological(name: &str, doc: &str) -> bool {
    let doc_words = words(doc);
    let Some(first) = doc_words.first() else { return false };
    let tokens = ident_tokens(name);
    let Some(verb) = tokens.first() else { return false };

    let echoes_name = first == &name.to_lowercase() || is_inflection(verb, first);
    if !echoes_name {
        return false;
    }

    let meaningful: Vec<&String> = doc_words[1..]
        .iter()
        .filter(|w| !STOP_WORDS.contains(&w.as_str()))
        .filter(|w| !tokens.iter().any(|t| t == *w))
        .filter(|w| !tokens.iter().any(|t| is_inflection(t, w)))
        .collect();

    meaningful.len() <= 2
}
