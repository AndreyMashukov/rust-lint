use syn::spanned::Spanned;
use syn::visit::{self, Visit};

use crate::lint::{path_segments, span_start, Analyzer, Diagnostic, SourceFile};

pub struct NoAllowAttr;

impl Analyzer for NoAllowAttr {
    fn name(&self) -> &'static str {
        "no_allow_attr"
    }
    fn doc(&self) -> &'static str {
        "forbids #[allow(...)] / #[expect(...)] lint suppression — fix the underlying issue"
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
    fn check_attr(&mut self, attr: &syn::Attribute) {
        let segs = path_segments(attr.path());
        let last = match segs.last() {
            Some(s) => s.clone(),
            None => return,
        };
        if last == "allow" || last == "expect" {
            let (line, col) = span_start(attr.span());
            self.out.push(Diagnostic::new(
                self.name,
                line,
                col,
                format!("#[{last}(...)] suppresses a lint — fix the underlying issue instead"),
            ));
        }
    }
}

impl<'ast, 'a> Visit<'ast> for Collector<'a> {
    fn visit_attribute(&mut self, attr: &'ast syn::Attribute) {
        self.check_attr(attr);
        visit::visit_attribute(self, attr);
    }
}
