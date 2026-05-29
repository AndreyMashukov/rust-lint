use syn::spanned::Spanned;
use syn::visit::{self, Visit};

use crate::lint::{span_start, Analyzer, Diagnostic, SourceFile};

pub struct NoRedundantIf;

impl Analyzer for NoRedundantIf {
    fn name(&self) -> &'static str {
        "no_redundant_if"
    }
    fn doc(&self) -> &'static str {
        "forbids redundant if returning a bool literal — return the condition (or its negation)"
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
            "redundant if — return the condition (or its negation) directly",
        ));
    }
}

fn bool_lit(expr: &syn::Expr) -> Option<bool> {
    if let syn::Expr::Lit(syn::ExprLit {
        lit: syn::Lit::Bool(b), ..
    }) = expr
    {
        Some(b.value)
    } else {
        None
    }
}

fn block_returns_bool(block: &syn::Block) -> Option<bool> {
    if block.stmts.len() != 1 {
        return None;
    }
    if let syn::Stmt::Expr(syn::Expr::Return(ret), _) = &block.stmts[0] {
        return ret.expr.as_deref().and_then(bool_lit);
    }
    None
}

fn block_tail_bool(block: &syn::Block) -> Option<bool> {
    if block.stmts.len() != 1 {
        return None;
    }
    if let syn::Stmt::Expr(expr, None) = &block.stmts[0] {
        return bool_lit(expr);
    }
    None
}

impl<'ast, 'a> Visit<'ast> for Collector<'a> {
    fn visit_block(&mut self, block: &'ast syn::Block) {
        for pair in block.stmts.windows(2) {
            let syn::Stmt::Expr(syn::Expr::If(if_expr), _) = &pair[0] else {
                continue;
            };
            if if_expr.else_branch.is_some() {
                continue;
            }
            let Some(then_b) = block_returns_bool(&if_expr.then_branch) else {
                continue;
            };
            let syn::Stmt::Expr(syn::Expr::Return(ret), _) = &pair[1] else {
                continue;
            };
            let Some(else_b) = ret.expr.as_deref().and_then(bool_lit) else {
                continue;
            };
            if then_b != else_b {
                self.report(if_expr.if_token.span());
            }
        }
        visit::visit_block(self, block);
    }

    fn visit_expr_if(&mut self, node: &'ast syn::ExprIf) {
        if let Some((_, else_expr)) = &node.else_branch {
            if let (Some(t), syn::Expr::Block(eb)) = (block_tail_bool(&node.then_branch), &**else_expr) {
                if let Some(e) = block_tail_bool(&eb.block) {
                    if t != e {
                        self.report(node.if_token.span());
                    }
                }
            }
        }
        visit::visit_expr_if(self, node);
    }
}
