use sarang::common::span::{SourceFile, Span};
use sarang::diagnostics::report::{Diagnostic, DiagnosticBag};
use sarang::lexer::token::{Token, TokenKind};

// ── Span integration tests ──────────────────────────────────────

#[test]
fn span_roundtrip_with_source() {
    let source = "agent MyAgent {}";
    let span = Span::new(6, 13); // "MyAgent"
    assert_eq!(&source[span.start..span.end], "MyAgent");
}

// ── Token integration tests ─────────────────────────────────────

#[test]
fn keyword_from_str_covers_all_block_keywords() {
    let block_keywords = [
        "agent", "model", "fallback", "tool", "evidence",
        "verify", "safety", "memory", "output", "budget",
    ];
    for kw in &block_keywords {
        let kind = TokenKind::keyword_from_str(kw).expect(kw);
        assert!(
            kind.is_block_keyword(),
            "{kw} should be a block keyword"
        );
    }
}

#[test]
fn token_spans_match_source_text() {
    let source = "agent Foo { model bar {} }";
    // Simulate tokens that a lexer would produce
    let tokens = vec![
        Token::new(TokenKind::Agent, Span::new(0, 5)),
        Token::new(TokenKind::Ident("Foo".into()), Span::new(6, 9)),
        Token::new(TokenKind::LBrace, Span::new(10, 11)),
        Token::new(TokenKind::Model, Span::new(12, 17)),
        Token::new(TokenKind::Ident("bar".into()), Span::new(18, 21)),
        Token::new(TokenKind::LBrace, Span::new(22, 23)),
        Token::new(TokenKind::RBrace, Span::new(23, 24)),
        Token::new(TokenKind::RBrace, Span::new(25, 26)),
        Token::new(TokenKind::Eof, Span::new(26, 26)),
    ];

    // Verify each token's span extracts the right text
    for tok in &tokens {
        if tok.kind == TokenKind::Eof {
            continue;
        }
        let text = &source[tok.span.start..tok.span.end];
        match &tok.kind {
            TokenKind::Agent => assert_eq!(text, "agent"),
            TokenKind::Model => assert_eq!(text, "model"),
            TokenKind::Ident(name) => assert_eq!(text, name.as_str()),
            TokenKind::LBrace => assert_eq!(text, "{"),
            TokenKind::RBrace => assert_eq!(text, "}"),
            _ => panic!("unexpected token kind"),
        }
    }
}

// ── Diagnostic rendering integration tests ──────────────────────

#[test]
fn diagnostic_rendering_shows_file_name() {
    let sf = SourceFile::new("my_policy.sarang", "agent X {}");
    let mut bag = DiagnosticBag::new();
    bag.push(Diagnostic::error("bad agent", Span::new(0, 5)));

    let output = bag.render_to_string(&sf);
    assert!(
        output.contains("my_policy.sarang"),
        "should include file name in output: {output}"
    );
}

#[test]
fn diagnostic_rendering_shows_line_and_column() {
    let source = "// comment\nagent Broken {}";
    let sf = SourceFile::new("test.sarang", source);
    let mut bag = DiagnosticBag::new();
    // "Broken" starts at byte 17 (after "// comment\nagent ")
    bag.push(Diagnostic::error("invalid name", Span::new(17, 23)));

    let output = bag.render_to_string(&sf);
    // codespan-reporting should show line 2
    assert!(
        output.contains(":2:"),
        "should show line number: {output}"
    );
}

#[test]
fn multiple_errors_all_rendered() {
    let source = "agent A {}\nagent B {}";
    let sf = SourceFile::new("multi.sarang", source);
    let mut bag = DiagnosticBag::new();
    bag.push(Diagnostic::error("error in A", Span::new(0, 5)));
    bag.push(Diagnostic::warning("warn in B", Span::new(11, 16)));

    let output = bag.render_to_string(&sf);
    assert!(output.contains("error in A"), "missing first diagnostic: {output}");
    assert!(output.contains("warn in B"), "missing second diagnostic: {output}");
    assert_eq!(bag.error_count(), 1);
    assert_eq!(bag.warning_count(), 1);
}
