use syn::spanned::Spanned;
use syn::visit::{self, Visit};

use crate::lint::{path_segments, span_start, Analyzer, Diagnostic, SourceFile};

pub struct NoDeadGuard;

const VARIANT_CTORS: &[&str] = &["Some", "None", "Ok", "Err"];
const VARIANT_PREDICATES: &[&str] = &["is_some", "is_none", "is_ok", "is_err"];

fn known_variant(expr: &syn::Expr) -> Option<String> {
    match expr {
        syn::Expr::Call(c) => {
            if let syn::Expr::Path(p) = &*c.func {
                let segs = path_segments(&p.path);
                let last = segs.last()?.clone();
                return VARIANT_CTORS.contains(&last.as_str()).then_some(last);
            }
            None
        }
        syn::Expr::Path(p) => {
            let segs = path_segments(&p.path);
            let last = segs.last()?.clone();
            (last == "None").then_some(last)
        }
        _ => None,
    }
}

impl Analyzer for NoDeadGuard {
    fn name(&self) -> &'static str {
        "no_dead_guard"
    }
    fn doc(&self) -> &'static str {
        "forbids guarding a statically-known Option/Result variant — dead defensive guard"
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
    fn report(&mut self, span: proc_macro2::Span, variant: &str) {
        let (line, col) = span_start(span);
        self.out.push(Diagnostic::new(
            self.name,
            line,
            col,
            format!("guard on a statically-known {variant}(…) value — dead defensive guard"),
        ));
    }
}

impl<'ast, 'a> Visit<'ast> for Collector<'a> {
    fn visit_expr_let(&mut self, node: &'ast syn::ExprLet) {
        if let Some(variant) = known_variant(&node.expr) {
            self.report(node.expr.span(), &variant);
        }
        visit::visit_expr_let(self, node);
    }

    fn visit_expr_match(&mut self, node: &'ast syn::ExprMatch) {
        if let Some(variant) = known_variant(&node.expr) {
            self.report(node.expr.span(), &variant);
        }
        visit::visit_expr_match(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        if VARIANT_PREDICATES.contains(&node.method.to_string().as_str()) {
            if let Some(variant) = known_variant(&node.receiver) {
                self.report(node.receiver.span(), &variant);
            }
        }
        visit::visit_expr_method_call(self, node);
    }
}
