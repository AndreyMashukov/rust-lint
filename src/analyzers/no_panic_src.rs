use syn::spanned::Spanned;
use syn::visit::{self, Visit};

use crate::lint::{attrs_mark_test, path_segments, span_start, Analyzer, Diagnostic, SourceFile};

pub struct NoPanicSrc;

const PANIC_MACROS: &[&str] = &["panic", "todo", "unimplemented", "unreachable"];

impl Analyzer for NoPanicSrc {
    fn name(&self) -> &'static str {
        "no_panic_src"
    }
    fn doc(&self) -> &'static str {
        "forbids panic!/todo!/unimplemented!/unreachable! and .unwrap()/.expect() in production code"
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
    fn report(&mut self, span: proc_macro2::Span, msg: &str) {
        let (line, col) = span_start(span);
        self.out.push(Diagnostic::new(self.name, line, col, msg));
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

    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        if let Some(last) = path_segments(&node.path).last() {
            if PANIC_MACROS.contains(&last.as_str()) {
                self.report(
                    node.path.span(),
                    &format!("{last}! in production code — return an error instead"),
                );
            }
        }
        visit::visit_macro(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        let m = node.method.to_string();
        if m == "unwrap" || m == "expect" {
            self.report(
                node.method.span(),
                &format!(".{m}() in production code — propagate the error with ? instead of crashing"),
            );
        }
        visit::visit_expr_method_call(self, node);
    }
}
