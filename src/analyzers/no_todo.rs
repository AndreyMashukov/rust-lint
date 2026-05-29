use crate::lint::{Analyzer, Diagnostic, SourceFile};

pub struct NoTodo;

const MARKERS: &[&str] = &["TODO", "FIXME", "XXX", "HACK"];

impl Analyzer for NoTodo {
    fn name(&self) -> &'static str {
        "no_todo"
    }
    fn doc(&self) -> &'static str {
        "forbids TODO/FIXME/XXX/HACK markers outright — implement it now or track it elsewhere"
    }
    fn check(&self, file: &SourceFile, out: &mut Vec<Diagnostic>) {
        for c in &file.comments {
            if let Some(marker) = opening_marker(c.body.trim()) {
                out.push(Diagnostic::new(
                    self.name(),
                    c.start_line,
                    c.start_col,
                    format!("{marker} marker is forbidden — implement it now, do not leave a stub"),
                ));
            }
        }
    }
}

fn opening_marker(text: &str) -> Option<&'static str> {
    MARKERS.iter().copied().find(|m| match text.strip_prefix(m) {
        Some(rest) => match rest.chars().next() {
            Some(c) => !c.is_alphanumeric(),
            None => true,
        },
        None => false,
    })
}
