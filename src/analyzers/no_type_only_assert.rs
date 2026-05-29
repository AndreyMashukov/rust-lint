use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Comma;
use syn::visit::{self, Visit};

use crate::lint::{attrs_mark_test, path_segments, span_start, Analyzer, Diagnostic, SourceFile};

pub struct NoTypeOnlyAssert;

const WEAK_PREDICATES: &[&str] = &["is_some", "is_none", "is_ok", "is_err", "is_empty"];

impl Analyzer for NoTypeOnlyAssert {
    fn name(&self) -> &'static str {
        "no_type_only_assert"
    }
    fn doc(&self) -> &'static str {
        "forbids type-/existence-only test assertions like assert!(x.is_some()) — assert the value"
    }
    fn check(&self, file: &SourceFile, out: &mut Vec<Diagnostic>) {
        let mut v = Collector {
            out,
            name: self.name(),
            in_test: file.is_test_file() as usize,
        };
        v.visit_file(&file.ast);
    }
}

struct Collector<'a> {
    out: &'a mut Vec<Diagnostic>,
    name: &'static str,
    in_test: usize,
}

fn weak_predicate(expr: &syn::Expr) -> Option<String> {
    match expr {
        syn::Expr::MethodCall(mc) => {
            let m = mc.method.to_string();
            WEAK_PREDICATES.contains(&m.as_str()).then_some(m)
        }
        syn::Expr::Unary(u) if matches!(u.op, syn::UnOp::Not(_)) => weak_predicate(&u.expr),
        syn::Expr::Paren(p) => weak_predicate(&p.expr),
        _ => None,
    }
}

impl<'ast, 'a> Visit<'ast> for Collector<'a> {
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        let test = attrs_mark_test(&node.attrs);
        self.in_test += test as usize;
        visit::visit_item_fn(self, node);
        self.in_test -= test as usize;
    }
    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        let test = attrs_mark_test(&node.attrs);
        self.in_test += test as usize;
        visit::visit_impl_item_fn(self, node);
        self.in_test -= test as usize;
    }
    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        let test = attrs_mark_test(&node.attrs);
        self.in_test += test as usize;
        visit::visit_item_mod(self, node);
        self.in_test -= test as usize;
    }

    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        if self.in_test > 0 {
            let segs = path_segments(&node.path);
            if let Some(last) = segs.last() {
                let name = last.clone();
                if name == "assert" || name == "debug_assert" {
                    if let Ok(args) = node.parse_body_with(Punctuated::<syn::Expr, Comma>::parse_terminated) {
                        if let Some(cond) = args.first() {
                            if let Some(pred) = weak_predicate(cond) {
                                let (line, col) = span_start(cond.span());
                                self.out.push(Diagnostic::new(
                                    self.name,
                                    line,
                                    col,
                                    format!("assert!(… .{pred}()) is type-/existence-only — assert the exact value"),
                                ));
                            }
                        }
                    }
                }
            }
        }
        visit::visit_macro(self, node);
    }
}
