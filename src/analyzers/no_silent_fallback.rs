use syn::visit::{self, Visit};

use crate::lint::{attrs_mark_test, span_start, Analyzer, Diagnostic, SourceFile};

pub struct NoSilentFallback;

const FALLBACK_METHODS: &[&str] = &[
    "unwrap_or",
    "unwrap_or_else",
    "unwrap_or_default",
    "ok_or",
    "ok_or_else",
    "map_or",
    "map_or_else",
    "get_or_insert",
    "get_or_insert_with",
];

impl Analyzer for NoSilentFallback {
    fn name(&self) -> &'static str {
        "no_silent_fallback"
    }
    fn doc(&self) -> &'static str {
        "forbids silent default methods on Option/Result: .unwrap_or, .unwrap_or_else, .unwrap_or_default, .ok_or, .ok_or_else, .map_or, .map_or_else, .get_or_insert, .get_or_insert_with"
    }
    fn check(&self, file: &SourceFile, out: &mut Vec<Diagnostic>) {
        if file.is_test_file() {
            return;
        }
        let mut v = Collector { out, name: self.name() };
        v.visit_file(&file.ast);
    }
}

struct Collector<'a> {
    out: &'a mut Vec<Diagnostic>,
    name: &'static str,
}

impl<'a> Collector<'a> {
    fn report(&mut self, span: proc_macro2::Span, method: &str) {
        let (line, col) = span_start(span);
        self.out.push(Diagnostic::new(
            self.name,
            line,
            col,
            format!(
                ".{method}() is a silent fallback for a missing value — validate at the boundary, propagate the error with ?, or let the call site read the value directly"
            ),
        ));
    }
}

impl<'ast, 'a> Visit<'ast> for Collector<'a> {
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        if attrs_mark_test(&node.attrs) {
            return;
        }
        visit::visit_item_fn(self, node);
    }
    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        if attrs_mark_test(&node.attrs) {
            return;
        }
        visit::visit_impl_item_fn(self, node);
    }
    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        if attrs_mark_test(&node.attrs) {
            return;
        }
        visit::visit_item_mod(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        let m = node.method.to_string();
        if FALLBACK_METHODS.contains(&m.as_str()) {
            self.report(node.method.span(), &m);
        }
        visit::visit_expr_method_call(self, node);
    }
}
