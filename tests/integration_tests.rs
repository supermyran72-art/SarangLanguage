use sarang::common::span::{SourceFile, Span};
use sarang::diagnostics::report::{Diagnostic, DiagnosticBag};
use sarang::lexer::{tokenize, TokenKind};
use sarang::parser::{self, AgentBlock, Value};
use sarang::validator;

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
    bag.push(Diagnostic::error("invalid name", Span::new(17, 23)));

    let output = bag.render_to_string(&sf);
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

// ── Lexer integration tests ─────────────────────────────────────

#[test]
fn lex_real_file_basic_agent() {
    let source = std::fs::read_to_string("examples/basic_agent.sarang")
        .expect("examples/basic_agent.sarang should exist");
    let tokens = tokenize(&source);

    // Must end with Eof
    assert_eq!(tokens.last().unwrap().kind, TokenKind::Eof);

    // Must not contain any error tokens
    for tok in &tokens {
        assert!(
            !matches!(tok.kind, TokenKind::Error(_)),
            "unexpected error token: {:?} at {:?}",
            tok.kind,
            tok.span
        );
    }

    // First real token should be "agent"
    assert_eq!(tokens[0].kind, TokenKind::Agent);
}

#[test]
fn lex_real_file_coding_assistant() {
    let source = std::fs::read_to_string("examples/coding_assistant.sarang")
        .expect("examples/coding_assistant.sarang should exist");
    let tokens = tokenize(&source);

    assert_eq!(tokens.last().unwrap().kind, TokenKind::Eof);

    for tok in &tokens {
        assert!(
            !matches!(tok.kind, TokenKind::Error(_)),
            "unexpected error token: {:?} at {:?}",
            tok.kind,
            tok.span
        );
    }

    // Verify some key tokens are present
    let kinds: Vec<_> = tokens.iter().map(|t| &t.kind).collect();
    assert!(kinds.contains(&&TokenKind::Agent));
    assert!(kinds.contains(&&TokenKind::Model));
    assert!(kinds.contains(&&TokenKind::Fallback));
    assert!(kinds.contains(&&TokenKind::Tool));
    assert!(kinds.contains(&&TokenKind::Evidence));
    assert!(kinds.contains(&&TokenKind::Verify));
    assert!(kinds.contains(&&TokenKind::Safety));
    assert!(kinds.contains(&&TokenKind::Memory));
    assert!(kinds.contains(&&TokenKind::Output));
    assert!(kinds.contains(&&TokenKind::Budget));
    assert!(kinds.contains(&&TokenKind::True));
}

#[test]
fn lex_real_file_research_agent() {
    let source = std::fs::read_to_string("examples/research_agent.sarang")
        .expect("examples/research_agent.sarang should exist");
    let tokens = tokenize(&source);

    assert_eq!(tokens.last().unwrap().kind, TokenKind::Eof);

    for tok in &tokens {
        assert!(
            !matches!(tok.kind, TokenKind::Error(_)),
            "unexpected error token: {:?} at {:?}",
            tok.kind,
            tok.span
        );
    }
}

#[test]
fn lex_all_token_spans_are_valid() {
    let source = std::fs::read_to_string("examples/coding_assistant.sarang").unwrap();
    let tokens = tokenize(&source);

    for tok in &tokens {
        assert!(tok.span.start <= tok.span.end, "invalid span: {:?}", tok);
        assert!(
            tok.span.end <= source.len(),
            "span exceeds source length: {:?}",
            tok
        );
        if tok.kind != TokenKind::Eof {
            // Non-EOF tokens must have non-empty spans
            assert!(
                !tok.span.is_empty(),
                "non-EOF token has empty span: {:?}",
                tok
            );
        }
    }
}

#[test]
fn lexer_matches_manual_tokens_for_simple_source() {
    let source = "agent Foo { model bar {} }";
    let tokens = tokenize(source);

    // Verify the lexer produces the same tokens we manually
    // constructed in Phase 2 integration tests
    assert_eq!(tokens[0].kind, TokenKind::Agent);
    assert_eq!(tokens[0].span, Span::new(0, 5));
    assert_eq!(tokens[1].kind, TokenKind::Ident("Foo".into()));
    assert_eq!(tokens[1].span, Span::new(6, 9));
    assert_eq!(tokens[2].kind, TokenKind::LBrace);
    assert_eq!(tokens[2].span, Span::new(10, 11));
    assert_eq!(tokens[3].kind, TokenKind::Model);
    assert_eq!(tokens[4].kind, TokenKind::Ident("bar".into()));
    assert_eq!(tokens[5].kind, TokenKind::LBrace);
    assert_eq!(tokens[6].kind, TokenKind::RBrace);
    assert_eq!(tokens[7].kind, TokenKind::RBrace);
    assert_eq!(tokens[8].kind, TokenKind::Eof);
}

// ── Parser integration tests ────────────────────────────────────

fn parse_file_ok(path: &str) -> sarang::parser::Program {
    let source = std::fs::read_to_string(path).unwrap_or_else(|_| panic!("{path} should exist"));
    match parser::parse(&source) {
        Ok(prog) => prog,
        Err(diags) => {
            let sf = SourceFile::new(path, &source);
            panic!("parse failed for {path}:\n{}", diags.render_to_string(&sf));
        }
    }
}

#[test]
fn parse_example_basic_agent() {
    let prog = parse_file_ok("examples/basic_agent.sarang");
    assert_eq!(prog.agent.name.value, "BasicAssistant");
    assert_eq!(prog.agent.blocks.len(), 3);
    assert_eq!(prog.agent.blocks[0].keyword(), "model");
    assert_eq!(prog.agent.blocks[1].keyword(), "safety");
    assert_eq!(prog.agent.blocks[2].keyword(), "output");
}

#[test]
fn parse_example_coding_assistant() {
    let prog = parse_file_ok("examples/coding_assistant.sarang");
    assert_eq!(prog.agent.name.value, "CodingAssistant");

    let keywords: Vec<_> = prog.agent.blocks.iter().map(|b| b.keyword()).collect();
    assert_eq!(
        keywords,
        vec![
            "model", "fallback", "tool", "tool", "tool",
            "evidence", "verify", "safety", "memory", "output", "budget"
        ]
    );

    // Verify a specific tool block
    if let AgentBlock::Tool(nb) = &prog.agent.blocks[4] {
        assert_eq!(nb.name.value, "shell_exec");
        assert_eq!(nb.fields.len(), 1);
        assert_eq!(nb.fields[0].name.value, "action");
        if let Value::String(s) = &nb.fields[0].value {
            assert_eq!(s.value, "deny");
        }
    } else {
        panic!("expected Tool block at index 4");
    }
}

#[test]
fn parse_example_research_agent() {
    let prog = parse_file_ok("examples/research_agent.sarang");
    assert_eq!(prog.agent.name.value, "ResearchAgent");

    // Should have: model, fallback, 3 tools, evidence, verify, safety, memory, output, budget
    assert_eq!(prog.agent.blocks.len(), 11);

    // Verify evidence requires 3 min sources
    if let AgentBlock::Evidence(ub) = &prog.agent.blocks[5] {
        let min_sources = &ub.fields[1];
        assert_eq!(min_sources.name.value, "min_sources");
        if let Value::Int(n) = &min_sources.value {
            assert_eq!(n.value, 3);
        }
    } else {
        panic!("expected Evidence at index 5");
    }
}

#[test]
fn parse_testdata_valid_minimal() {
    let prog = parse_file_ok("testdata/valid/minimal.sarang");
    assert_eq!(prog.agent.name.value, "Minimal");
    assert_eq!(prog.agent.blocks.len(), 1);
}

#[test]
fn parse_testdata_invalid_missing_model() {
    let source =
        std::fs::read_to_string("testdata/invalid/missing_model.sarang").unwrap();
    // This file is syntactically valid (no parse errors), but
    // semantically invalid (no model block) — that's the validator's job.
    // The parser should succeed here.
    let result = parser::parse(&source);
    assert!(result.is_ok(), "syntactically valid file should parse");
}

// ── Validator integration tests ─────────────────────────────────

fn validate_file(path: &str) -> DiagnosticBag {
    let source = std::fs::read_to_string(path).unwrap_or_else(|_| panic!("{path} should exist"));
    let program = parser::parse(&source).unwrap_or_else(|diags| {
        let sf = SourceFile::new(path, &source);
        panic!("parse failed for {path}:\n{}", diags.render_to_string(&sf));
    });
    validator::validate(&program)
}

#[test]
fn validate_example_basic_agent_passes() {
    let diags = validate_file("examples/basic_agent.sarang");
    assert!(
        !diags.has_errors(),
        "basic_agent.sarang should be valid: {:?}",
        diags.iter().collect::<Vec<_>>()
    );
}

#[test]
fn validate_example_coding_assistant_passes() {
    let diags = validate_file("examples/coding_assistant.sarang");
    assert!(
        !diags.has_errors(),
        "coding_assistant.sarang should be valid: {:?}",
        diags.iter().collect::<Vec<_>>()
    );
}

#[test]
fn validate_example_research_agent_passes() {
    let diags = validate_file("examples/research_agent.sarang");
    assert!(
        !diags.has_errors(),
        "research_agent.sarang should be valid: {:?}",
        diags.iter().collect::<Vec<_>>()
    );
}

#[test]
fn validate_testdata_valid_minimal_passes() {
    let diags = validate_file("testdata/valid/minimal.sarang");
    assert!(
        !diags.has_errors(),
        "minimal.sarang should be valid: {:?}",
        diags.iter().collect::<Vec<_>>()
    );
}

#[test]
fn validate_testdata_missing_model_fails() {
    let diags = validate_file("testdata/invalid/missing_model.sarang");
    assert!(diags.has_errors(), "missing_model.sarang should fail validation");
    let msgs: Vec<_> = diags.iter().map(|d| d.message.as_str()).collect();
    assert!(
        msgs.iter().any(|m| m.contains("missing a required `model` block")),
        "expected missing model error: {msgs:?}"
    );
}

#[test]
fn validate_testdata_duplicate_safety_fails() {
    let diags = validate_file("testdata/invalid/duplicate_safety.sarang");
    assert!(diags.has_errors());
    let msgs: Vec<_> = diags.iter().map(|d| d.message.as_str()).collect();
    assert!(
        msgs.iter().any(|m| m.contains("duplicate `safety` block")),
        "expected duplicate safety error: {msgs:?}"
    );
}

#[test]
fn validate_testdata_bad_tool_action_fails() {
    let diags = validate_file("testdata/invalid/bad_tool_action.sarang");
    assert!(diags.has_errors());
    let msgs: Vec<_> = diags.iter().map(|d| d.message.as_str()).collect();
    assert!(
        msgs.iter().any(|m| m.contains("invalid tool action")),
        "expected invalid action error: {msgs:?}"
    );
}

#[test]
fn validate_testdata_unknown_field_fails() {
    let diags = validate_file("testdata/invalid/unknown_field.sarang");
    assert!(diags.has_errors());
    let msgs: Vec<_> = diags.iter().map(|d| d.message.as_str()).collect();
    assert!(
        msgs.iter().any(|m| m.contains("unknown field `flavor`")),
        "expected unknown field error: {msgs:?}"
    );
}
