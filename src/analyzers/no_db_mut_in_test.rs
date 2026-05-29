use syn::visit::{self, Visit};

use crate::lint::{attrs_mark_test, span_start, Analyzer, Diagnostic, SourceFile};

pub struct NoDbMutInTest;

const MUTATING: &[&str] = &["INSERT ", "UPDATE ", "DELETE ", "TRUNCATE ", "DROP ", "ALTER "];
const SQL_COMPANIONS: &[&str] = &[" FROM ", " INTO ", " SET ", " TABLE ", " VALUES", " WHERE "];

fn is_mutating_sql(s: &str) -> bool {
    let upper = format!("{} ", s.trim().to_uppercase());
    let starts = MUTATING.iter().any(|kw| upper.starts_with(kw));
    starts && SQL_COMPANIONS.iter().any(|c| upper.contains(c))
}

impl Analyzer for NoDbMutInTest {
    fn name(&self) -> &'static str {
        "no_db_mut_in_test"
    }
    fn doc(&self) -> &'static str {
        "forbids raw INSERT/UPDATE/DELETE/etc SQL in tests — drive state through production code"
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

    fn visit_lit_str(&mut self, node: &'ast syn::LitStr) {
        if self.in_test > 0 && is_mutating_sql(&node.value()) {
            let (line, col) = span_start(node.span());
            self.out.push(Diagnostic::new(
                self.name,
                line,
                col,
                "raw mutating SQL in test — drive state through production code paths",
            ));
        }
        visit::visit_lit_str(self, node);
    }
}
