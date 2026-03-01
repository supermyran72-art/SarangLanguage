use sarang::common::span::Span;
use sarang::diagnostics::report::{Diagnostic, DiagnosticBag, Severity};

#[test]
fn span_new_and_len() {
    let span = Span::new(0, 10);
    assert_eq!(span.len(), 10);
    assert!(!span.is_empty());
}

#[test]
fn span_empty() {
    let span = Span::new(5, 5);
    assert!(span.is_empty());
    assert_eq!(span.len(), 0);
}

#[test]
fn span_merge() {
    let a = Span::new(2, 5);
    let b = Span::new(8, 12);
    let merged = a.merge(b);
    assert_eq!(merged.start, 2);
    assert_eq!(merged.end, 12);
}

#[test]
fn diagnostic_error_creation() {
    let diag = Diagnostic::error("test error", Span::new(0, 5));
    assert_eq!(diag.severity, Severity::Error);
    assert!(diag.is_error());
    assert_eq!(diag.message, "test error");
    assert!(diag.hint.is_none());
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
    assert!(!bag.has_errors());

    bag.push(Diagnostic::warning("warn", Span::new(0, 1)));
    assert!(!bag.has_errors());
    assert_eq!(bag.error_count(), 0);

    bag.push(Diagnostic::error("err", Span::new(2, 3)));
    assert!(bag.has_errors());
    assert_eq!(bag.error_count(), 1);
}
