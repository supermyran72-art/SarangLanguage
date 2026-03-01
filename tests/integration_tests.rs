use sarang::common::span::{SourceFile, Span};
use sarang::diagnostics::report::{Diagnostic, DiagnosticBag};
use sarang::ir;
use sarang::lexer::{tokenize, TokenKind};
use sarang::parser::{self, AgentBlock, Value};
use sarang::validator;
use sarang::{compile, CompileError};

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

// ── Lowering integration tests ──────────────────────────────────

fn lower_file(path: &str) -> ir::PolicyIr {
    let source = std::fs::read_to_string(path).unwrap_or_else(|_| panic!("{path} should exist"));
    let program = parser::parse(&source).unwrap_or_else(|diags| {
        let sf = SourceFile::new(path, &source);
        panic!("parse failed for {path}:\n{}", diags.render_to_string(&sf));
    });
    let diags = validator::validate(&program);
    assert!(
        !diags.has_errors(),
        "validation failed for {path}: {:?}",
        diags.iter().collect::<Vec<_>>()
    );
    ir::lower(&program).unwrap_or_else(|e| panic!("lowering failed for {path}: {e}"))
}

#[test]
fn lower_example_basic_agent() {
    let ir = lower_file("examples/basic_agent.sarang");
    assert_eq!(ir.version, "0.1.0");
    assert_eq!(ir.agent.name, "BasicAssistant");
    assert_eq!(ir.agent.models.len(), 1);
    assert_eq!(ir.agent.models[0].provider, "openai");
    assert!(ir.agent.safety.is_some());
    assert!(ir.agent.output.is_some());
}

#[test]
fn lower_example_coding_assistant() {
    let ir = lower_file("examples/coding_assistant.sarang");
    assert_eq!(ir.agent.name, "CodingAssistant");
    assert_eq!(ir.agent.models.len(), 1);
    assert_eq!(ir.agent.models[0].provider, "anthropic");
    assert_eq!(ir.agent.models[0].temperature, Some(0.3));
    assert_eq!(ir.agent.fallbacks.len(), 1);
    assert_eq!(ir.agent.tools.len(), 3);
    assert_eq!(ir.agent.tools[0].action, ir::ToolAction::Allow);
    assert_eq!(ir.agent.tools[1].action, ir::ToolAction::RequireApproval);
    assert_eq!(ir.agent.tools[2].action, ir::ToolAction::Deny);

    let ev = ir.agent.evidence.as_ref().unwrap();
    assert_eq!(ev.require, vec!["source_file", "line_number"]);
    assert_eq!(ev.min_sources, Some(1));

    let vf = ir.agent.verify.as_ref().unwrap();
    assert_eq!(vf.rules.len(), 2);

    let mem = ir.agent.memory.as_ref().unwrap();
    assert_eq!(mem.read, Some(ir::MemoryAccess::Allow));
    assert_eq!(mem.write, Some(ir::MemoryWriteAccess::Scoped));
    assert_eq!(mem.max_entries, Some(100));

    let bud = ir.agent.budget.as_ref().unwrap();
    assert_eq!(bud.max_tokens, Some(100000));
    assert_eq!(bud.max_cost_usd, Some(1.0));
    assert_eq!(bud.max_time_seconds, Some(120));
}

#[test]
fn lower_example_research_agent() {
    let ir = lower_file("examples/research_agent.sarang");
    assert_eq!(ir.agent.name, "ResearchAgent");
    assert_eq!(ir.agent.tools.len(), 3);
    assert!(ir.agent.tools.iter().all(|t| t.action == ir::ToolAction::Allow));
    assert!(ir.agent.tools.iter().all(|t| t.require_evidence == Some(true)));

    let ev = ir.agent.evidence.as_ref().unwrap();
    assert_eq!(ev.require, vec!["url", "title", "retrieved_at"]);
    assert_eq!(ev.min_sources, Some(3));

    let vf = ir.agent.verify.as_ref().unwrap();
    assert_eq!(vf.rules.len(), 3);

    let mem = ir.agent.memory.as_ref().unwrap();
    assert_eq!(mem.write, Some(ir::MemoryWriteAccess::AppendOnly));
}

#[test]
fn lower_testdata_valid_minimal() {
    let ir = lower_file("testdata/valid/minimal.sarang");
    assert_eq!(ir.agent.name, "Minimal");
    assert_eq!(ir.agent.models.len(), 1);
    assert!(ir.agent.fallbacks.is_empty());
    assert!(ir.agent.tools.is_empty());
    assert!(ir.agent.evidence.is_none());
}

#[test]
fn lower_roundtrip_coding_assistant_json() {
    let ir = lower_file("examples/coding_assistant.sarang");
    let json = serde_json::to_string_pretty(&ir).unwrap();
    let roundtrip: ir::PolicyIr = serde_json::from_str(&json).unwrap();
    assert_eq!(ir, roundtrip);
}

// ── Emit integration tests ─────────────────────────────────────

fn compile_file(path: &str) -> String {
    let ir = lower_file(path);
    sarang::emit::emit_json_pretty(&ir).expect("emit should succeed")
}

#[test]
fn emit_basic_agent_json() {
    let json = compile_file("examples/basic_agent.sarang");
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["version"], "0.1.0");
    assert_eq!(parsed["agent"]["name"], "BasicAssistant");
    assert!(parsed["agent"]["models"].is_array());
}

#[test]
fn emit_coding_assistant_json_has_all_sections() {
    let json = compile_file("examples/coding_assistant.sarang");
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    let agent = &parsed["agent"];
    assert!(agent["models"].is_array());
    assert!(agent["fallbacks"].is_array());
    assert!(agent["tools"].is_array());
    assert!(agent["evidence"].is_object());
    assert!(agent["verify"].is_object());
    assert!(agent["safety"].is_object());
    assert!(agent["memory"].is_object());
    assert!(agent["output"].is_object());
    assert!(agent["budget"].is_object());
}

#[test]
fn emit_minimal_json_omits_optional_sections() {
    let json = compile_file("testdata/valid/minimal.sarang");
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    let agent = &parsed["agent"];
    assert!(agent.get("fallbacks").is_none());
    assert!(agent.get("tools").is_none());
    assert!(agent.get("evidence").is_none());
    assert!(agent.get("verify").is_none());
    assert!(agent.get("safety").is_none());
    assert!(agent.get("memory").is_none());
    assert!(agent.get("output").is_none());
    assert!(agent.get("budget").is_none());
}

#[test]
fn emit_research_agent_roundtrip() {
    let ir = lower_file("examples/research_agent.sarang");
    let json = sarang::emit::emit_json(&ir).unwrap();
    let roundtrip: ir::PolicyIr = serde_json::from_str(&json).unwrap();
    assert_eq!(ir, roundtrip);
}

// ── CLI integration tests ──────────────────────────────────────

use std::process::Command as CliCommand;

fn cargo_bin() -> CliCommand {
    CliCommand::new(env!("CARGO_BIN_EXE_sarang"))
}

#[test]
fn cli_version() {
    let output = cargo_bin().arg("version").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.starts_with("sarang "));
}

#[test]
fn cli_check_valid_file() {
    let output = cargo_bin()
        .args(["check", "examples/basic_agent.sarang"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("ok:"));
}

#[test]
fn cli_check_invalid_file_exits_nonzero() {
    let output = cargo_bin()
        .args(["check", "testdata/invalid/missing_model.sarang"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("missing a required `model` block"));
}

#[test]
fn cli_compile_to_stdout() {
    let output = cargo_bin()
        .args(["compile", "examples/basic_agent.sarang"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed["version"], "0.1.0");
    assert_eq!(parsed["agent"]["name"], "BasicAssistant");
}

#[test]
fn cli_compile_to_file() {
    let dir = tempfile::tempdir().unwrap();
    let out_path = dir.path().join("output.json");
    let output = cargo_bin()
        .args([
            "compile",
            "examples/coding_assistant.sarang",
            "-o",
            out_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let json = std::fs::read_to_string(&out_path).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["agent"]["name"], "CodingAssistant");
}

#[test]
fn cli_compile_invalid_file_exits_nonzero() {
    let output = cargo_bin()
        .args(["compile", "testdata/invalid/bad_tool_action.sarang"])
        .output()
        .unwrap();
    assert!(!output.status.success());
}

#[test]
fn cli_check_nonexistent_file() {
    let output = cargo_bin()
        .args(["check", "nonexistent.sarang"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("file not found"));
}

#[test]
fn cli_inspect_prints_ast() {
    let output = cargo_bin()
        .args(["inspect", "testdata/valid/minimal.sarang"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Program"));
    assert!(stdout.contains("AgentDef"));
}

// ── compile() convenience function tests ────────────────────────

#[test]
fn compile_fn_basic_agent() {
    let source = std::fs::read_to_string("examples/basic_agent.sarang").unwrap();
    let policy = compile(&source).unwrap();
    assert_eq!(policy.agent.name, "BasicAssistant");
    assert_eq!(policy.version, "0.1.0");
}

#[test]
fn compile_fn_coding_assistant() {
    let source = std::fs::read_to_string("examples/coding_assistant.sarang").unwrap();
    let policy = compile(&source).unwrap();
    assert_eq!(policy.agent.name, "CodingAssistant");
    assert_eq!(policy.agent.tools.len(), 3);
}

#[test]
fn compile_fn_parse_error() {
    let result = compile("this is not valid sarang");
    assert!(matches!(result, Err(CompileError::Parse(_))));
}

#[test]
fn compile_fn_validation_error() {
    let source = std::fs::read_to_string("testdata/invalid/missing_model.sarang").unwrap();
    let result = compile(&source);
    assert!(matches!(result, Err(CompileError::Validation(_))));
}

#[test]
fn compile_error_display() {
    let err = compile("not sarang").unwrap_err();
    let msg = format!("{err}");
    assert!(!msg.is_empty());
}

// ── Edge-case tests ────────────────────────────────────────────

#[test]
fn empty_source_is_parse_error() {
    let result = compile("");
    assert!(matches!(result, Err(CompileError::Parse(_))));
}

#[test]
fn whitespace_only_is_parse_error() {
    let result = compile("   \n\n\t  \n");
    assert!(matches!(result, Err(CompileError::Parse(_))));
}

#[test]
fn comment_only_is_parse_error() {
    let result = compile("// just a comment\n// and another\n");
    assert!(matches!(result, Err(CompileError::Parse(_))));
}

#[test]
fn lex_unterminated_string() {
    let tokens = tokenize("agent Foo { model bar { provider \"open");
    let has_error = tokens.iter().any(|t| matches!(t.kind, TokenKind::Error(_)));
    assert!(has_error, "unterminated string should produce an error token");
}

#[test]
fn lex_unknown_character() {
    let tokens = tokenize("agent Foo { model bar { # } }");
    let has_error = tokens.iter().any(|t| matches!(t.kind, TokenKind::Error(_)));
    assert!(has_error, "unknown character # should produce an error token");
}

#[test]
fn lex_empty_input() {
    let tokens = tokenize("");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].kind, TokenKind::Eof);
}

#[test]
fn parse_duplicate_fields_in_block() {
    let source = r#"agent Test {
        model primary {
            provider "openai"
            provider "anthropic"
        }
    }"#;
    // Duplicate fields should either parse successfully (validator catches them)
    // or produce a parse error — either way, we ensure no panic.
    let _ = parser::parse(source);
}

#[test]
fn validate_agent_with_no_blocks() {
    let source = "agent Empty {}";
    let prog = parser::parse(source).unwrap();
    let diags = validator::validate(&prog);
    assert!(diags.has_errors(), "agent with no model should fail validation");
}

#[test]
fn validate_unknown_block_keyword_is_parse_error() {
    let source = "agent Foo { widget bar {} }";
    let result = parser::parse(source);
    assert!(result.is_err(), "unknown block keyword should fail parsing");
}

#[test]
fn cli_wrong_extension() {
    let output = cargo_bin()
        .args(["check", "Cargo.toml"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("expected a .sarang file"));
}

#[test]
fn compile_fn_roundtrip_json() {
    let source = std::fs::read_to_string("examples/research_agent.sarang").unwrap();
    let policy = compile(&source).unwrap();
    let json = sarang::emit::emit_json_pretty(&policy).unwrap();
    let roundtrip: ir::PolicyIr = serde_json::from_str(&json).unwrap();
    assert_eq!(policy, roundtrip);
}

#[test]
fn diagnostic_re_exports_accessible() {
    // Verify the diagnostic re-exports are accessible from the short path
    let _diag = sarang::diagnostics::Diagnostic::error_no_span("test");
    let _bag = sarang::diagnostics::DiagnosticBag::new();
    let _sev = sarang::diagnostics::Severity::Error;
}
