use syn::spanned::Spanned;
use syn::visit::{self, Visit};

use crate::lint::{path_segments, span_start, Analyzer, Diagnostic, SourceFile};

pub struct NoTimeNow;

const CLOCK_TYPES: &[&str] = &[
    "Instant",
    "SystemTime",
    "Utc",
    "Local",
    "OffsetDateTime",
    "DateTime",
    "Date",
    "Time",
];
const NOW_FNS: &[&str] = &["now", "now_utc", "now_local"];

impl Analyzer for NoTimeNow {
    fn name(&self) -> &'static str {
        "no_time_now"
    }
    fn doc(&self) -> &'static str {
        "forbids direct Instant/SystemTime/Utc/Local ::now() outside a clock module — inject a Clock"
    }
    fn check(&self, file: &SourceFile, out: &mut Vec<Diagnostic>) {
        if file.is_clock_module() {
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

impl<'ast, 'a> Visit<'ast> for Collector<'a> {
    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        if let syn::Expr::Path(p) = &*node.func {
            let segs = path_segments(&p.path);
            let n = segs.len();
            if n >= 2 && NOW_FNS.contains(&segs[n - 1].as_str()) && CLOCK_TYPES.contains(&segs[n - 2].as_str()) {
                let (line, col) = span_start(node.func.span());
                self.out.push(Diagnostic::new(
                    self.name,
                    line,
                    col,
                    format!(
                        "direct {}::{} — inject a Clock interface for testability",
                        segs[n - 2],
                        segs[n - 1]
                    ),
                ));
            }
        }
        visit::visit_expr_call(self, node);
    }
}
