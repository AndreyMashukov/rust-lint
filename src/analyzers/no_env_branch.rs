use syn::spanned::Spanned;
use syn::visit::{self, Visit};

use crate::lint::{span_start, Analyzer, Diagnostic, SourceFile};

pub struct NoEnvBranch;

const ENV_LITERALS: &[&str] = &[
    "prod",
    "production",
    "dev",
    "development",
    "test",
    "testing",
    "stage",
    "staging",
    "local",
];

const ENV_NAMES: &[&str] = &[
    "env",
    "environment",
    "appenv",
    "mode",
    "runmode",
    "stage",
    "staging",
    "profile",
    "tier",
    "deploy",
    "deployment",
];

fn env_literal_of(expr: &syn::Expr) -> Option<String> {
    if let syn::Expr::Lit(lit) = expr {
        if let syn::Lit::Str(s) = &lit.lit {
            let v = s.value().to_lowercase();
            return ENV_LITERALS.contains(&v.as_str()).then_some(v);
        }
    }
    None
}

fn pat_env_literal(pat: &syn::Pat) -> bool {
    if let syn::Pat::Lit(lit) = pat {
        if let syn::Lit::Str(s) = &lit.lit {
            return ENV_LITERALS.contains(&s.value().to_lowercase().as_str());
        }
    }
    false
}

fn names_in(expr: &syn::Expr, acc: &mut Vec<String>) {
    match expr {
        syn::Expr::Path(p) => acc.extend(p.path.segments.iter().map(|s| s.ident.to_string())),
        syn::Expr::Field(f) => {
            names_in(&f.base, acc);
            if let syn::Member::Named(id) = &f.member {
                acc.push(id.to_string());
            }
        }
        syn::Expr::MethodCall(m) => {
            names_in(&m.receiver, acc);
            acc.push(m.method.to_string());
        }
        syn::Expr::Call(c) => names_in(&c.func, acc),
        syn::Expr::Reference(r) => names_in(&r.expr, acc),
        syn::Expr::Paren(p) => names_in(&p.expr, acc),
        syn::Expr::Unary(u) => names_in(&u.expr, acc),
        syn::Expr::Try(t) => names_in(&t.expr, acc),
        syn::Expr::Cast(c) => names_in(&c.expr, acc),
        _ => {}
    }
}

fn looks_like_env_selector(expr: &syn::Expr) -> bool {
    let mut names = Vec::new();
    names_in(expr, &mut names);
    names.iter().any(|n| {
        let lower = n.to_lowercase();
        lower.contains("env") || ENV_NAMES.contains(&lower.as_str())
    })
}

impl Analyzer for NoEnvBranch {
    fn name(&self) -> &'static str {
        "no_env_branch"
    }
    fn doc(&self) -> &'static str {
        "forbids runtime branching on env strings ('prod'/'dev'/'test') — behave identically everywhere"
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
            "runtime environment branching — production code must behave identically in all environments",
        ));
    }
}

impl<'ast, 'a> Visit<'ast> for Collector<'a> {
    fn visit_expr_binary(&mut self, node: &'ast syn::ExprBinary) {
        if matches!(node.op, syn::BinOp::Eq(_) | syn::BinOp::Ne(_)) {
            let selector = if env_literal_of(&node.left).is_some() {
                Some(&node.right)
            } else if env_literal_of(&node.right).is_some() {
                Some(&node.left)
            } else {
                None
            };
            if let Some(sel) = selector {
                if looks_like_env_selector(sel) {
                    self.report(node.span());
                }
            }
        }
        visit::visit_expr_binary(self, node);
    }

    fn visit_expr_match(&mut self, node: &'ast syn::ExprMatch) {
        if looks_like_env_selector(&node.expr) {
            for arm in &node.arms {
                if pat_env_literal(&arm.pat) {
                    self.report(arm.pat.span());
                }
            }
        }
        visit::visit_expr_match(self, node);
    }
}
