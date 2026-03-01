use crate::common::span::Span;
use crate::diagnostics::report::{Diagnostic, DiagnosticBag};
use crate::lexer::token::{Token, TokenKind};
use crate::parser::ast::*;

/// The Sarang parser. Converts a token stream into an AST.
///
/// Uses recursive descent with simple error recovery: on a parse
/// error, the parser emits a diagnostic and synchronizes to the
/// next block keyword or closing brace.
pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    pub diagnostics: DiagnosticBag,
}

/// The result of parsing: either a valid program or `None` if
/// the errors were too severe to produce an AST.
pub type ParseResult = Result<Program, DiagnosticBag>;

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            pos: 0,
            diagnostics: DiagnosticBag::new(),
        }
    }

    /// Parse the entire token stream into a `Program`.
    pub fn parse(mut self) -> ParseResult {
        match self.parse_program() {
            Some(program) => {
                if self.diagnostics.has_errors() {
                    Err(self.diagnostics)
                } else {
                    Ok(program)
                }
            }
            None => Err(self.diagnostics),
        }
    }

    // ── Top-level parsing ───────────────────────────────────

    /// program → agent_def EOF
    fn parse_program(&mut self) -> Option<Program> {
        let agent = self.parse_agent_def()?;

        if !self.check(&TokenKind::Eof) {
            let tok = self.current();
            self.error(
                format!("expected end of file, found {}", tok.kind.describe()),
                tok.span,
            );
        }

        Some(Program { agent })
    }

    /// agent_def → "agent" IDENT "{" block* "}"
    fn parse_agent_def(&mut self) -> Option<AgentDef> {
        let start_span = self.current().span;

        if !self.expect(&TokenKind::Agent) {
            let tok = self.current();
            self.error(
                format!(
                    "expected keyword `agent`, found {}",
                    tok.kind.describe()
                ),
                tok.span,
            );
            return None;
        }

        let name = self.expect_ident("agent name")?;

        if !self.expect(&TokenKind::LBrace) {
            let tok = self.current();
            self.error_with_hint(
                format!("expected `{{` after agent name, found {}", tok.kind.describe()),
                tok.span,
                format!("agent {} should be followed by a block: `agent {} {{ ... }}`", name.value, name.value),
            );
            return None;
        }

        let mut blocks = Vec::new();

        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            if let Some(block) = self.parse_block() {
                blocks.push(block);
            } else {
                // Error recovery: skip to next block keyword or closing brace
                self.synchronize_to_block();
            }
        }

        let end_span = self.current().span;
        if !self.expect(&TokenKind::RBrace) {
            self.error("expected `}` to close agent definition", end_span);
        }

        let span = start_span.merge(end_span);

        Some(AgentDef {
            name,
            blocks,
            span,
        })
    }

    // ── Block parsing ───────────────────────────────────────

    /// block → named_block | unnamed_block
    fn parse_block(&mut self) -> Option<AgentBlock> {
        let tok = self.current();
        match &tok.kind {
            // Named blocks: keyword <name> { fields }
            TokenKind::Model => self.parse_named_block(BlockTag::Model),
            TokenKind::Fallback => self.parse_named_block(BlockTag::Fallback),
            TokenKind::Tool => self.parse_named_block(BlockTag::Tool),

            // Unnamed blocks: keyword { fields }
            TokenKind::Evidence => self.parse_unnamed_block(BlockTag::Evidence),
            TokenKind::Verify => self.parse_unnamed_block(BlockTag::Verify),
            TokenKind::Safety => self.parse_unnamed_block(BlockTag::Safety),
            TokenKind::Memory => self.parse_unnamed_block(BlockTag::Memory),
            TokenKind::Output => self.parse_unnamed_block(BlockTag::Output),
            TokenKind::Budget => self.parse_unnamed_block(BlockTag::Budget),

            _ => {
                self.error(
                    format!(
                        "expected a block keyword (model, tool, safety, ...), found {}",
                        tok.kind.describe()
                    ),
                    tok.span,
                );
                None
            }
        }
    }

    /// named_block → KEYWORD IDENT "{" field* "}"
    fn parse_named_block(&mut self, tag: BlockTag) -> Option<AgentBlock> {
        let start_span = self.current().span;
        self.advance(); // consume keyword

        let name = self.expect_ident(&format!("{} block name", tag.as_str()))?;

        if !self.expect(&TokenKind::LBrace) {
            let tok = self.current();
            self.error(
                format!(
                    "expected `{{` after {} block name, found {}",
                    tag.as_str(),
                    tok.kind.describe()
                ),
                tok.span,
            );
            return None;
        }

        let fields = self.parse_fields();

        let end_span = self.current().span;
        if !self.expect(&TokenKind::RBrace) {
            self.error(
                format!("expected `}}` to close {} block", tag.as_str()),
                end_span,
            );
        }

        let span = start_span.merge(end_span);
        let block = NamedBlock {
            name,
            fields,
            span,
        };

        Some(match tag {
            BlockTag::Model => AgentBlock::Model(block),
            BlockTag::Fallback => AgentBlock::Fallback(block),
            BlockTag::Tool => AgentBlock::Tool(block),
            _ => unreachable!(),
        })
    }

    /// unnamed_block → KEYWORD "{" field* "}"
    fn parse_unnamed_block(&mut self, tag: BlockTag) -> Option<AgentBlock> {
        let start_span = self.current().span;
        self.advance(); // consume keyword

        if !self.expect(&TokenKind::LBrace) {
            let tok = self.current();
            self.error(
                format!(
                    "expected `{{` after `{}`, found {}",
                    tag.as_str(),
                    tok.kind.describe()
                ),
                tok.span,
            );
            return None;
        }

        let fields = self.parse_fields();

        let end_span = self.current().span;
        if !self.expect(&TokenKind::RBrace) {
            self.error(
                format!("expected `}}` to close {} block", tag.as_str()),
                end_span,
            );
        }

        let span = start_span.merge(end_span);
        let block = UnnamedBlock { fields, span };

        Some(match tag {
            BlockTag::Evidence => AgentBlock::Evidence(block),
            BlockTag::Verify => AgentBlock::Verify(block),
            BlockTag::Safety => AgentBlock::Safety(block),
            BlockTag::Memory => AgentBlock::Memory(block),
            BlockTag::Output => AgentBlock::Output(block),
            BlockTag::Budget => AgentBlock::Budget(block),
            _ => unreachable!(),
        })
    }

    // ── Field parsing ───────────────────────────────────────

    /// Parse zero or more fields until `}` or EOF.
    fn parse_fields(&mut self) -> Vec<Field> {
        let mut fields = Vec::new();

        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            if let Some(field) = self.parse_field() {
                fields.push(field);
            } else {
                // Error recovery: skip current token and try again
                self.advance();
            }
        }

        fields
    }

    /// field → (IDENT | KEYWORD) ":" value
    ///
    /// Inside a block body, keywords are valid field names (e.g.,
    /// `model: "gpt-4"` inside a fallback block).
    fn parse_field(&mut self) -> Option<Field> {
        let name = self.expect_field_name()?;

        if !self.expect(&TokenKind::Colon) {
            let tok = self.current();
            self.error_with_hint(
                format!("expected `:` after field name, found {}", tok.kind.describe()),
                tok.span,
                format!("fields use the syntax: {}: <value>", name.value),
            );
            return None;
        }

        let value = self.parse_value()?;
        let span = name.span.merge(value.span());

        Some(Field { name, value, span })
    }

    // ── Value parsing ───────────────────────────────────────

    /// value → STRING | INT | FLOAT | "true" | "false" | list
    fn parse_value(&mut self) -> Option<Value> {
        let tok = self.current();
        match &tok.kind {
            TokenKind::StringLiteral(_) => {
                let tok = self.advance_and_return();
                if let TokenKind::StringLiteral(s) = tok.kind {
                    Some(Value::String(Spanned::new(s, tok.span)))
                } else {
                    unreachable!()
                }
            }
            TokenKind::IntLiteral(_) => {
                let tok = self.advance_and_return();
                if let TokenKind::IntLiteral(n) = tok.kind {
                    Some(Value::Int(Spanned::new(n, tok.span)))
                } else {
                    unreachable!()
                }
            }
            TokenKind::FloatLiteral(_) => {
                let tok = self.advance_and_return();
                if let TokenKind::FloatLiteral(f) = tok.kind {
                    Some(Value::Float(Spanned::new(f, tok.span)))
                } else {
                    unreachable!()
                }
            }
            TokenKind::True => {
                let tok = self.advance_and_return();
                Some(Value::Bool(Spanned::new(true, tok.span)))
            }
            TokenKind::False => {
                let tok = self.advance_and_return();
                Some(Value::Bool(Spanned::new(false, tok.span)))
            }
            TokenKind::LBracket => self.parse_list(),
            _ => {
                self.error(
                    format!(
                        "expected a value (string, number, boolean, or list), found {}",
                        tok.kind.describe()
                    ),
                    tok.span,
                );
                None
            }
        }
    }

    /// list → "[" (value ("," value)* ","?)? "]"
    fn parse_list(&mut self) -> Option<Value> {
        let start_span = self.current().span;
        self.advance(); // consume '['

        let mut elements = Vec::new();

        if !self.check(&TokenKind::RBracket) {
            // Parse first element
            let val = self.parse_value()?;
            elements.push(val);

            // Parse remaining elements
            while self.check(&TokenKind::Comma) {
                self.advance(); // consume ','

                // Allow trailing comma: if we see ']' after comma, stop
                if self.check(&TokenKind::RBracket) {
                    break;
                }

                let val = self.parse_value()?;
                elements.push(val);
            }
        }

        let end_span = self.current().span;
        if !self.expect(&TokenKind::RBracket) {
            self.error("expected `]` to close list", end_span);
            return None;
        }

        let span = start_span.merge(end_span);
        Some(Value::List(ListValue { elements, span }))
    }

    // ── Token cursor ────────────────────────────────────────

    /// Returns the current token without advancing.
    fn current(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or_else(|| {
            self.tokens
                .last()
                .expect("token stream must contain at least EOF")
        })
    }

    /// Advance the cursor by one and return the consumed token.
    fn advance_and_return(&mut self) -> Token {
        let tok = self.tokens[self.pos].clone();
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        tok
    }

    /// Advance the cursor by one.
    fn advance(&mut self) {
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
    }

    /// Check if the current token matches the given kind (without consuming it).
    fn check(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(&self.current().kind) == std::mem::discriminant(kind)
    }

    /// If the current token matches the given kind, consume it and return `true`.
    fn expect(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    /// Accept an identifier or keyword as a field name.
    ///
    /// Inside block bodies, keywords like `model`, `output`, etc. are valid
    /// field names (e.g., `model: "gpt-4"` inside a fallback block).
    fn expect_field_name(&mut self) -> Option<Spanned<String>> {
        let tok = self.current().clone();
        match &tok.kind {
            TokenKind::Ident(name) => {
                let spanned = Spanned::new(name.clone(), tok.span);
                self.advance();
                Some(spanned)
            }
            kind if kind.is_block_keyword() => {
                let name = match kind {
                    TokenKind::Agent => "agent",
                    TokenKind::Model => "model",
                    TokenKind::Fallback => "fallback",
                    TokenKind::Tool => "tool",
                    TokenKind::Evidence => "evidence",
                    TokenKind::Verify => "verify",
                    TokenKind::Safety => "safety",
                    TokenKind::Memory => "memory",
                    TokenKind::Output => "output",
                    TokenKind::Budget => "budget",
                    _ => unreachable!(),
                };
                let spanned = Spanned::new(name.to_owned(), tok.span);
                self.advance();
                Some(spanned)
            }
            // Not a field name — return None so parse_fields can handle it
            _ => None,
        }
    }

    /// Expect an identifier token, consume it, and return the name as Spanned.
    fn expect_ident(&mut self, context: &str) -> Option<Spanned<String>> {
        let tok = self.current().clone();
        if let TokenKind::Ident(name) = &tok.kind {
            let spanned = Spanned::new(name.clone(), tok.span);
            self.advance();
            Some(spanned)
        } else {
            self.error(
                format!("expected {context}, found {}", tok.kind.describe()),
                tok.span,
            );
            None
        }
    }

    // ── Error recovery ──────────────────────────────────────

    /// Skip tokens until we find a block keyword or closing `}`.
    /// This allows the parser to recover and continue parsing
    /// after encountering an error inside a block.
    fn synchronize_to_block(&mut self) {
        loop {
            let tok = self.current();
            match &tok.kind {
                // Block keywords — we can try parsing a new block
                TokenKind::Model
                | TokenKind::Fallback
                | TokenKind::Tool
                | TokenKind::Evidence
                | TokenKind::Verify
                | TokenKind::Safety
                | TokenKind::Memory
                | TokenKind::Output
                | TokenKind::Budget => return,
                // Closing brace — the enclosing block parser will handle it
                TokenKind::RBrace => return,
                // End of file — stop
                TokenKind::Eof => return,
                // Skip everything else
                _ => self.advance(),
            }
        }
    }

    // ── Diagnostics ─────────────────────────────────────────

    fn error(&mut self, message: impl Into<String>, span: Span) {
        self.diagnostics
            .push(Diagnostic::error(message.into(), span));
    }

    fn error_with_hint(
        &mut self,
        message: impl Into<String>,
        span: Span,
        hint: impl Into<String>,
    ) {
        self.diagnostics
            .push(Diagnostic::error(message.into(), span).with_hint(hint.into()));
    }
}

/// Internal tag used to route block parsing to the correct AST variant.
#[derive(Debug, Clone, Copy)]
enum BlockTag {
    Model,
    Fallback,
    Tool,
    Evidence,
    Verify,
    Safety,
    Memory,
    Output,
    Budget,
}

impl BlockTag {
    fn as_str(self) -> &'static str {
        match self {
            BlockTag::Model => "model",
            BlockTag::Fallback => "fallback",
            BlockTag::Tool => "tool",
            BlockTag::Evidence => "evidence",
            BlockTag::Verify => "verify",
            BlockTag::Safety => "safety",
            BlockTag::Memory => "memory",
            BlockTag::Output => "output",
            BlockTag::Budget => "budget",
        }
    }
}

/// Convenience function: parse a source string in one call.
pub fn parse(source: &str) -> ParseResult {
    let tokens = crate::lexer::tokenize(source);
    Parser::new(tokens).parse()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: parse source and assert success, returning the Program.
    fn parse_ok(source: &str) -> Program {
        match parse(source) {
            Ok(program) => program,
            Err(diags) => {
                let sf = crate::common::span::SourceFile::new("<test>", source);
                panic!(
                    "parse failed:\n{}",
                    diags.render_to_string(&sf)
                );
            }
        }
    }

    /// Helper: parse source and assert failure, returning the diagnostics.
    fn parse_err(source: &str) -> DiagnosticBag {
        match parse(source) {
            Ok(_) => panic!("expected parse error, but parsing succeeded"),
            Err(diags) => diags,
        }
    }

    // ── Minimal programs ────────────────────────────────────

    #[test]
    fn parse_minimal_agent() {
        let prog = parse_ok(
            r#"agent Minimal {
                model default {
                    provider: "openai"
                    name: "gpt-4"
                }
            }"#,
        );
        assert_eq!(prog.agent.name.value, "Minimal");
        assert_eq!(prog.agent.blocks.len(), 1);
        assert_eq!(prog.agent.blocks[0].keyword(), "model");
    }

    #[test]
    fn parse_empty_agent() {
        let prog = parse_ok("agent Empty {}");
        assert_eq!(prog.agent.name.value, "Empty");
        assert_eq!(prog.agent.blocks.len(), 0);
    }

    // ── Named blocks ────────────────────────────────────────

    #[test]
    fn parse_model_block() {
        let prog = parse_ok(
            r#"agent A {
                model primary {
                    provider: "anthropic"
                    name: "claude-sonnet-4-20250514"
                    temperature: 0.3
                }
            }"#,
        );
        let block = &prog.agent.blocks[0];
        assert_eq!(block.keyword(), "model");
        assert!(block.is_named());
        if let AgentBlock::Model(nb) = block {
            assert_eq!(nb.name.value, "primary");
            assert_eq!(nb.fields.len(), 3);
            assert_eq!(nb.fields[0].name.value, "provider");
            assert_eq!(nb.fields[1].name.value, "name");
            assert_eq!(nb.fields[2].name.value, "temperature");
        } else {
            panic!("expected Model block");
        }
    }

    #[test]
    fn parse_fallback_block() {
        let prog = parse_ok(
            r#"agent A {
                fallback backup {
                    model: "gpt-4"
                    provider: "openai"
                    condition: "primary_unavailable"
                }
            }"#,
        );
        assert_eq!(prog.agent.blocks[0].keyword(), "fallback");
        if let AgentBlock::Fallback(nb) = &prog.agent.blocks[0] {
            assert_eq!(nb.name.value, "backup");
            assert_eq!(nb.fields.len(), 3);
        } else {
            panic!("expected Fallback block");
        }
    }

    #[test]
    fn parse_tool_block() {
        let prog = parse_ok(
            r#"agent A {
                tool file_read {
                    action: "allow"
                    require_evidence: true
                }
            }"#,
        );
        if let AgentBlock::Tool(nb) = &prog.agent.blocks[0] {
            assert_eq!(nb.name.value, "file_read");
            assert_eq!(nb.fields.len(), 2);
            assert_eq!(nb.fields[1].name.value, "require_evidence");
            assert!(matches!(nb.fields[1].value, Value::Bool(ref b) if b.value));
        } else {
            panic!("expected Tool block");
        }
    }

    // ── Unnamed blocks ──────────────────────────────────────

    #[test]
    fn parse_safety_block() {
        let prog = parse_ok(
            r#"agent A {
                safety {
                    block: ["harmful_content", "pii_leak"]
                }
            }"#,
        );
        if let AgentBlock::Safety(ub) = &prog.agent.blocks[0] {
            assert_eq!(ub.fields.len(), 1);
            assert_eq!(ub.fields[0].name.value, "block");
            if let Value::List(lv) = &ub.fields[0].value {
                assert_eq!(lv.elements.len(), 2);
            } else {
                panic!("expected list value");
            }
        } else {
            panic!("expected Safety block");
        }
    }

    #[test]
    fn parse_evidence_block() {
        let prog = parse_ok(
            r#"agent A {
                evidence {
                    require: ["source_file", "line_number"]
                    min_sources: 1
                }
            }"#,
        );
        if let AgentBlock::Evidence(ub) = &prog.agent.blocks[0] {
            assert_eq!(ub.fields.len(), 2);
        } else {
            panic!("expected Evidence block");
        }
    }

    #[test]
    fn parse_verify_block_repeated_fields() {
        let prog = parse_ok(
            r#"agent A {
                verify {
                    rule: "no_hallucinated_imports"
                    rule: "no_undefined_variables"
                }
            }"#,
        );
        if let AgentBlock::Verify(ub) = &prog.agent.blocks[0] {
            assert_eq!(ub.fields.len(), 2);
            assert_eq!(ub.fields[0].name.value, "rule");
            assert_eq!(ub.fields[1].name.value, "rule");
        } else {
            panic!("expected Verify block");
        }
    }

    #[test]
    fn parse_memory_block() {
        let prog = parse_ok(
            r#"agent A {
                memory {
                    read: "allow"
                    write: "scoped"
                    max_entries: 100
                }
            }"#,
        );
        if let AgentBlock::Memory(ub) = &prog.agent.blocks[0] {
            assert_eq!(ub.fields.len(), 3);
        } else {
            panic!("expected Memory block");
        }
    }

    #[test]
    fn parse_output_block() {
        let prog = parse_ok(
            r#"agent A {
                output {
                    format: "markdown"
                    max_tokens: 8192
                }
            }"#,
        );
        assert_eq!(prog.agent.blocks[0].keyword(), "output");
    }

    #[test]
    fn parse_budget_block() {
        let prog = parse_ok(
            r#"agent A {
                budget {
                    max_tokens: 100000
                    max_cost_usd: 1.00
                    max_time_seconds: 120
                }
            }"#,
        );
        if let AgentBlock::Budget(ub) = &prog.agent.blocks[0] {
            assert_eq!(ub.fields.len(), 3);
            // Check float value
            if let Value::Float(f) = &ub.fields[1].value {
                assert!((f.value - 1.0).abs() < f64::EPSILON);
            } else {
                panic!("expected float for max_cost_usd");
            }
        } else {
            panic!("expected Budget block");
        }
    }

    // ── Value types ─────────────────────────────────────────

    #[test]
    fn parse_all_value_types() {
        let prog = parse_ok(
            r#"agent A {
                model m {
                    str_val: "hello"
                    int_val: 42
                    float_val: 3.14
                    bool_val: true
                    list_val: ["a", "b"]
                }
            }"#,
        );
        if let AgentBlock::Model(nb) = &prog.agent.blocks[0] {
            assert!(matches!(nb.fields[0].value, Value::String(_)));
            assert!(matches!(nb.fields[1].value, Value::Int(_)));
            assert!(matches!(nb.fields[2].value, Value::Float(_)));
            assert!(matches!(nb.fields[3].value, Value::Bool(_)));
            assert!(matches!(nb.fields[4].value, Value::List(_)));
        } else {
            panic!("expected Model block");
        }
    }

    #[test]
    fn parse_empty_list() {
        let prog = parse_ok(
            r#"agent A {
                safety {
                    block: []
                }
            }"#,
        );
        if let AgentBlock::Safety(ub) = &prog.agent.blocks[0] {
            if let Value::List(lv) = &ub.fields[0].value {
                assert!(lv.elements.is_empty());
            } else {
                panic!("expected list");
            }
        } else {
            panic!("expected Safety");
        }
    }

    #[test]
    fn parse_list_trailing_comma() {
        let prog = parse_ok(
            r#"agent A {
                safety {
                    block: ["a", "b",]
                }
            }"#,
        );
        if let AgentBlock::Safety(ub) = &prog.agent.blocks[0] {
            if let Value::List(lv) = &ub.fields[0].value {
                assert_eq!(lv.elements.len(), 2);
            } else {
                panic!("expected list");
            }
        } else {
            panic!("expected Safety");
        }
    }

    // ── Multiple blocks ─────────────────────────────────────

    #[test]
    fn parse_multiple_blocks() {
        let prog = parse_ok(
            r#"agent A {
                model primary {
                    provider: "openai"
                }
                tool read {
                    action: "allow"
                }
                safety {
                    block: ["x"]
                }
            }"#,
        );
        assert_eq!(prog.agent.blocks.len(), 3);
        assert_eq!(prog.agent.blocks[0].keyword(), "model");
        assert_eq!(prog.agent.blocks[1].keyword(), "tool");
        assert_eq!(prog.agent.blocks[2].keyword(), "safety");
    }

    // ── Span accuracy ───────────────────────────────────────

    #[test]
    fn agent_span_covers_full_definition() {
        let source = "agent Foo {}";
        let prog = parse_ok(source);
        assert_eq!(prog.agent.span.start, 0);
        assert_eq!(prog.agent.span.end, source.len());
    }

    #[test]
    fn field_name_span_matches_source() {
        let source = r#"agent A { model m { provider: "x" } }"#;
        let prog = parse_ok(source);
        if let AgentBlock::Model(nb) = &prog.agent.blocks[0] {
            let field = &nb.fields[0];
            assert_eq!(&source[field.name.span.start..field.name.span.end], "provider");
        }
    }

    // ── Error cases ─────────────────────────────────────────

    #[test]
    fn error_missing_agent_keyword() {
        let diags = parse_err("model foo {}");
        assert!(diags.has_errors());
    }

    #[test]
    fn error_missing_agent_name() {
        let diags = parse_err("agent {}");
        assert!(diags.has_errors());
    }

    #[test]
    fn error_missing_opening_brace() {
        let diags = parse_err("agent Foo model m {}");
        assert!(diags.has_errors());
    }

    #[test]
    fn error_missing_colon_in_field() {
        let diags = parse_err(r#"agent A { model m { provider "openai" } }"#);
        assert!(diags.has_errors());
    }

    #[test]
    fn error_invalid_value() {
        let diags = parse_err(r#"agent A { model m { provider: { } } }"#);
        assert!(diags.has_errors());
    }

    #[test]
    fn error_unexpected_token_in_agent() {
        let diags = parse_err(r#"agent A { "stray_string" }"#);
        assert!(diags.has_errors());
    }

    #[test]
    fn error_unterminated_list() {
        let diags = parse_err(r#"agent A { safety { block: ["a", "b" } }"#);
        assert!(diags.has_errors());
    }

    #[test]
    fn error_recovery_continues_after_bad_block() {
        // The parser should recover after the bad block and still parse "safety"
        let source = r#"agent A {
            model {}
            safety {
                block: ["x"]
            }
        }"#;
        let diags = parse_err(source);
        // Should have at least one error (missing model name)
        assert!(diags.has_errors());
    }
}
