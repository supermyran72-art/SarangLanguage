use crate::common::span::{SourceFile, Span};
use codespan_reporting::diagnostic as cs_diag;
use codespan_reporting::term;
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream, Buffer};

/// Severity level of a diagnostic message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// A single diagnostic message with source location.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub span: Option<Span>,
    pub hint: Option<String>,
}

impl Diagnostic {
    pub fn error(message: impl Into<String>, span: Span) -> Self {
        Self {
            severity: Severity::Error,
            message: message.into(),
            span: Some(span),
            hint: None,
        }
    }

    pub fn error_no_span(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            message: message.into(),
            span: None,
            hint: None,
        }
    }

    pub fn warning(message: impl Into<String>, span: Span) -> Self {
        Self {
            severity: Severity::Warning,
            message: message.into(),
            span: Some(span),
            hint: None,
        }
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }

    /// Convert to a `codespan_reporting` diagnostic for rendering.
    fn to_codespan(&self) -> cs_diag::Diagnostic<()> {
        let severity = match self.severity {
            Severity::Error => cs_diag::Severity::Error,
            Severity::Warning => cs_diag::Severity::Warning,
            Severity::Info => cs_diag::Severity::Note,
        };

        let mut diag = cs_diag::Diagnostic::new(severity).with_message(&self.message);

        if let Some(span) = self.span {
            let label = cs_diag::Label::primary((), span.start..span.end);
            diag = diag.with_labels(vec![label]);
        }

        if let Some(hint) = &self.hint {
            diag = diag.with_notes(vec![hint.clone()]);
        }

        diag
    }
}

/// Collects diagnostics during compilation.
#[derive(Debug, Default)]
pub struct DiagnosticBag {
    diagnostics: Vec<Diagnostic>,
}

impl DiagnosticBag {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, diag: Diagnostic) {
        self.diagnostics.push(diag);
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(|d| d.is_error())
    }

    pub fn error_count(&self) -> usize {
        self.diagnostics.iter().filter(|d| d.is_error()).count()
    }

    pub fn warning_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .count()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }

    pub fn len(&self) -> usize {
        self.diagnostics.len()
    }

    /// Print all diagnostics to stderr with colors and source context.
    pub fn emit(&self, source_file: &SourceFile) {
        let writer = StandardStream::stderr(ColorChoice::Auto);
        let config = term::Config::default();
        let file = source_file.as_simple_file();

        for diag in &self.diagnostics {
            let cs = diag.to_codespan();
            let _ = term::emit(&mut writer.lock(), &config, &file, &cs);
        }
    }

    /// Render all diagnostics to a string (no colors). Useful for testing.
    pub fn render_to_string(&self, source_file: &SourceFile) -> String {
        let mut buffer = Buffer::no_color();
        let config = term::Config::default();
        let file = source_file.as_simple_file();

        for diag in &self.diagnostics {
            let cs = diag.to_codespan();
            let _ = term::emit(&mut buffer, &config, &file, &cs);
        }

        String::from_utf8_lossy(buffer.as_slice()).into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostic_error_creation() {
        let diag = Diagnostic::error("test error", Span::new(0, 5));
        assert_eq!(diag.severity, Severity::Error);
        assert!(diag.is_error());
        assert_eq!(diag.message, "test error");
        assert!(diag.hint.is_none());
    }

    #[test]
    fn diagnostic_error_no_span() {
        let diag = Diagnostic::error_no_span("global error");
        assert!(diag.is_error());
        assert!(diag.span.is_none());
    }

    #[test]
    fn diagnostic_with_hint() {
        let diag = Diagnostic::warning("test warning", Span::new(0, 3))
            .with_hint("try this instead");
        assert_eq!(diag.severity, Severity::Warning);
        assert!(!diag.is_error());
        assert_eq!(diag.hint.as_deref(), Some("try this instead"));
    }

    #[test]
    fn diagnostic_bag_collects_and_counts() {
        let mut bag = DiagnosticBag::new();
        assert!(bag.is_empty());
        assert_eq!(bag.len(), 0);
        assert!(!bag.has_errors());

        bag.push(Diagnostic::warning("warn", Span::new(0, 1)));
        assert!(!bag.has_errors());
        assert_eq!(bag.error_count(), 0);
        assert_eq!(bag.warning_count(), 1);

        bag.push(Diagnostic::error("err", Span::new(2, 3)));
        assert!(bag.has_errors());
        assert_eq!(bag.error_count(), 1);
        assert_eq!(bag.len(), 2);
    }

    #[test]
    fn render_error_to_string() {
        let source = SourceFile::new("test.sarang", "agent Foo {}\n");
        let mut bag = DiagnosticBag::new();
        bag.push(Diagnostic::error("unexpected token", Span::new(6, 9)));

        let output = bag.render_to_string(&source);
        assert!(output.contains("error"), "should contain 'error': {output}");
        assert!(
            output.contains("unexpected token"),
            "should contain message: {output}"
        );
        assert!(output.contains("Foo"), "should show source context: {output}");
    }

    #[test]
    fn render_warning_with_hint() {
        let source = SourceFile::new("test.sarang", "agent Bar {}");
        let mut bag = DiagnosticBag::new();
        bag.push(
            Diagnostic::warning("unused agent", Span::new(0, 5))
                .with_hint("consider removing this agent"),
        );

        let output = bag.render_to_string(&source);
        assert!(
            output.contains("warning"),
            "should contain 'warning': {output}"
        );
        assert!(
            output.contains("consider removing"),
            "should contain hint: {output}"
        );
    }

    #[test]
    fn render_multiple_diagnostics() {
        let source = SourceFile::new("test.sarang", "agent X { model y {} }");
        let mut bag = DiagnosticBag::new();
        bag.push(Diagnostic::error("first error", Span::new(0, 5)));
        bag.push(Diagnostic::error("second error", Span::new(10, 15)));

        let output = bag.render_to_string(&source);
        assert!(
            output.contains("first error"),
            "should contain first: {output}"
        );
        assert!(
            output.contains("second error"),
            "should contain second: {output}"
        );
    }

    #[test]
    fn render_no_span_diagnostic() {
        let source = SourceFile::new("test.sarang", "");
        let mut bag = DiagnosticBag::new();
        bag.push(Diagnostic::error_no_span("file is empty"));

        let output = bag.render_to_string(&source);
        assert!(
            output.contains("file is empty"),
            "should contain message: {output}"
        );
    }
}
