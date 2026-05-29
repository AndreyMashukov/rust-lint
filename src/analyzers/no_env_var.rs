use syn::spanned::Spanned;
use syn::visit::{self, Visit};

use crate::lint::{path_segments, span_start, Analyzer, Diagnostic, SourceFile};

pub struct NoEnvVar;

impl Analyzer for NoEnvVar {
    fn name(&self) -> &'static str {
        "no_env_var"
    }
    fn doc(&self) -> &'static str {
        "forbids std::env::var / env::var outside config modules — inject a typed config struct"
    }
    fn check(&self, file: &SourceFile, out: &mut Vec<Diagnostic>) {
        if file.is_config_module() {
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

const ENV_FNS: &[&str] = &["var", "var_os", "vars", "vars_os"];

impl<'ast, 'a> Visit<'ast> for Collector<'a> {
    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        if let syn::Expr::Path(p) = &*node.func {
            let segs = path_segments(&p.path);
            let n = segs.len();
            if n >= 2 && segs[n - 2] == "env" && ENV_FNS.contains(&segs[n - 1].as_str()) {
                let (line, col) = span_start(node.func.span());
                self.out.push(Diagnostic::new(
                    self.name,
                    line,
                    col,
                    format!(
                        "env::{} outside a config module — inject a typed config struct",
                        segs[n - 1]
                    ),
                ));
            }
        }
        visit::visit_expr_call(self, node);
    }
}
