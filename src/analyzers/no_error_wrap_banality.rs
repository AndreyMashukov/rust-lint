use syn::spanned::Spanned;
use syn::visit::{self, Visit};

use crate::lint::{path_segments, span_start, Analyzer, Diagnostic, SourceFile};

pub struct NoErrorWrapBanality;

const LEAD_INS: &[&str] = &["failed to", "unable to", "could not", "couldn't", "cannot", "error"];

fn is_banal(msg: &str) -> bool {
    let lower = msg.trim().to_lowercase();
    let Some(rest) = LEAD_INS.iter().find_map(|lead| strip_lead(&lower, lead)) else {
        return false;
    };
    let mut tail = rest.trim();
    if let Some(open) = tail.rfind('{') {
        if tail.ends_with('}') {
            tail = tail[..open].trim();
        }
    }
    let tail = tail.trim_end_matches([':', ' ']).trim();
    if tail.is_empty() {
        return true;
    }
    let words: Vec<&str> = tail.split_whitespace().collect();
    words.len() <= 2 && words.iter().all(|w| w.chars().all(|c| c.is_alphanumeric()))
}

fn strip_lead<'a>(s: &'a str, lead: &str) -> Option<&'a str> {
    let rest = s.strip_prefix(lead)?;
    match rest.chars().next() {
        None => Some(rest),
        Some(c) if !c.is_alphanumeric() => Some(rest),
        Some(_) => None,
    }
}

impl Analyzer for NoErrorWrapBanality {
    fn name(&self) -> &'static str {
        "no_error_wrap_banality"
    }
    fn doc(&self) -> &'static str {
        "forbids error wrappers (.context/anyhow!/bail!) that add no context — return err or add real context"
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
    fn report(&mut self, span: proc_macro2::Span) {
        let (line, col) = span_start(span);
        self.out.push(Diagnostic::new(
            self.name,
            line,
            col,
            "error wrapper adds no context — return the error directly or add real context (id, path, entity)",
        ));
    }
}

fn lit_str(expr: &syn::Expr) -> Option<String> {
    if let syn::Expr::Lit(syn::ExprLit {
        lit: syn::Lit::Str(s), ..
    }) = expr
    {
        Some(s.value())
    } else {
        None
    }
}

fn closure_lit(expr: &syn::Expr) -> Option<String> {
    if let syn::Expr::Closure(c) = expr {
        return match &*c.body {
            syn::Expr::Block(b) => b.block.stmts.iter().find_map(|s| match s {
                syn::Stmt::Expr(e, _) => lit_str(e),
                _ => None,
            }),
            other => lit_str(other),
        };
    }
    None
}

impl<'ast, 'a> Visit<'ast> for Collector<'a> {
    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        let m = node.method.to_string();
        if m == "context" {
            if let Some(arg) = node.args.first() {
                if lit_str(arg).is_some_and(|s| is_banal(&s)) {
                    self.report(node.method.span());
                }
            }
        } else if m == "with_context" {
            if let Some(arg) = node.args.first() {
                if closure_lit(arg).is_some_and(|s| is_banal(&s)) {
                    self.report(node.method.span());
                }
            }
        }
        visit::visit_expr_method_call(self, node);
    }

    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        let segs = path_segments(&node.path);
        let Some(last_seg) = segs.last() else {
            visit::visit_macro(self, node);
            return;
        };
        let last = last_seg.clone();
        if last == "anyhow" || last == "bail" {
            if let Ok(args) =
                node.parse_body_with(syn::punctuated::Punctuated::<syn::Expr, syn::token::Comma>::parse_terminated)
            {
                if let Some(first) = args.first() {
                    if lit_str(first).is_some_and(|s| is_banal(&s)) {
                        self.report(node.path.span());
                    }
                }
            }
        }
        visit::visit_macro(self, node);
    }
}
