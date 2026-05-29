use syn::visit::{self, Visit};

use crate::lint::{span_start, Analyzer, CommentKind, Diagnostic, SourceFile};

pub struct NoInlineComment;

impl Analyzer for NoInlineComment {
    fn name(&self) -> &'static str {
        "no_inline_comment"
    }
    fn doc(&self) -> &'static str {
        "forbids // and /* */ comments inside function bodies — explain via clear naming, not prose"
    }
    fn check(&self, file: &SourceFile, out: &mut Vec<Diagnostic>) {
        let mut v = BodyRanges::default();
        v.visit_file(&file.ast);

        for c in &file.comments {
            if c.kind.is_doc() {
                continue;
            }
            if is_allowed_body(&c.body) {
                continue;
            }
            if v.contains(c.start_line, c.start_col) {
                out.push(Diagnostic::new(
                    self.name(),
                    c.start_line,
                    c.start_col,
                    match c.kind {
                        CommentKind::Block => {
                            "block comment inside function body — explain via clear naming, not prose"
                        }
                        _ => "inline comment inside function body — explain via clear naming, not prose",
                    },
                ));
            }
        }
    }
}

fn is_allowed_body(body: &str) -> bool {
    let upper = body.to_uppercase();
    if upper.starts_with("TODO") || upper.starts_with("FIXME") || upper.starts_with("XXX") || upper.starts_with("HACK")
    {
        return true;
    }
    let lower = body.to_lowercase();
    lower.starts_with("rust-lint") || lower.starts_with("want")
}

#[derive(Default)]
struct BodyRanges {
    ranges: Vec<(usize, usize, usize, usize)>,
}

impl BodyRanges {
    fn push_block(&mut self, block: &syn::Block) {
        let (ol, oc) = span_start(block.brace_token.span.open());
        let (cl, cc) = span_start(block.brace_token.span.close());
        self.ranges.push((ol, oc, cl, cc));
    }

    fn contains(&self, line: usize, col: usize) -> bool {
        self.ranges.iter().any(|&(ol, oc, cl, cc)| {
            let after_open = line > ol || (line == ol && col > oc);
            let before_close = line < cl || (line == cl && col < cc);
            after_open && before_close
        })
    }
}

impl<'ast> Visit<'ast> for BodyRanges {
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        self.push_block(&node.block);
        visit::visit_item_fn(self, node);
    }
    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        self.push_block(&node.block);
        visit::visit_impl_item_fn(self, node);
    }
    fn visit_trait_item_fn(&mut self, node: &'ast syn::TraitItemFn) {
        if let Some(block) = &node.default {
            self.push_block(block);
        }
        visit::visit_trait_item_fn(self, node);
    }
    fn visit_expr_closure(&mut self, node: &'ast syn::ExprClosure) {
        if let syn::Expr::Block(b) = &*node.body {
            self.push_block(&b.block);
        }
        visit::visit_expr_closure(self, node);
    }
}
