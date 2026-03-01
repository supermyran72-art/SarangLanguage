use crate::common::span::Span;
use crate::diagnostics::report::{Diagnostic, DiagnosticBag};
use crate::parser::ast::*;

/// Validates a parsed Sarang AST for semantic correctness.
///
/// Catches: missing required blocks/fields, duplicate singleton blocks,
/// duplicate named blocks, unknown fields, wrong value types, invalid
/// field values, and cross-block contradictions.
pub struct Validator {
    pub diagnostics: DiagnosticBag,
}

impl Default for Validator {
    fn default() -> Self {
        Self::new()
    }
}

impl Validator {
    pub fn new() -> Self {
        Self {
            diagnostics: DiagnosticBag::new(),
        }
    }

    /// Validate a program and return the diagnostic bag.
    pub fn validate(mut self, program: &Program) -> DiagnosticBag {
        self.validate_agent(&program.agent);
        self.diagnostics
    }

    // ── Agent-level checks ──────────────────────────────────

    fn validate_agent(&mut self, agent: &AgentDef) {
        self.check_required_model(agent);
        self.check_duplicate_singletons(agent);
        self.check_duplicate_named_blocks(agent);

        for block in &agent.blocks {
            self.validate_block(block);
        }

        self.check_cross_block_rules(agent);
    }

    /// Rule: at least one `model` block is required.
    fn check_required_model(&mut self, agent: &AgentDef) {
        let has_model = agent.blocks.iter().any(|b| matches!(b, AgentBlock::Model(_)));
        if !has_model {
            self.error_with_hint(
                "agent is missing a required `model` block",
                agent.name.span,
                "every agent must define at least one model",
            );
        }
    }

    /// Rule: singleton blocks (evidence, verify, safety, memory, output, budget)
    /// must appear at most once.
    fn check_duplicate_singletons(&mut self, agent: &AgentDef) {
        let singleton_kinds = ["evidence", "verify", "safety", "memory", "output", "budget"];

        for kind in &singleton_kinds {
            let occurrences: Vec<_> = agent
                .blocks
                .iter()
                .filter(|b| b.keyword() == *kind)
                .collect();

            if occurrences.len() > 1 {
                for dup in &occurrences[1..] {
                    self.error_with_hint(
                        format!("duplicate `{kind}` block"),
                        dup.span(),
                        format!("`{kind}` can only appear once per agent"),
                    );
                }
            }
        }
    }

    /// Rule: named blocks (model, fallback, tool) must have unique names
    /// within their type.
    fn check_duplicate_named_blocks(&mut self, agent: &AgentDef) {
        self.check_named_duplicates_for(agent, "model");
        self.check_named_duplicates_for(agent, "fallback");
        self.check_named_duplicates_for(agent, "tool");
    }

    fn check_named_duplicates_for(&mut self, agent: &AgentDef, kind: &str) {
        let named: Vec<_> = agent
            .blocks
            .iter()
            .filter(|b| b.keyword() == kind)
            .filter_map(|b| match b {
                AgentBlock::Model(nb) | AgentBlock::Fallback(nb) | AgentBlock::Tool(nb) => {
                    Some(&nb.name)
                }
                _ => None,
            })
            .collect();

        let mut seen = std::collections::HashSet::new();
        for name in &named {
            if !seen.insert(&name.value) {
                self.error_with_hint(
                    format!("duplicate {kind} name `{}`", name.value),
                    name.span,
                    format!("each {kind} block must have a unique name"),
                );
            }
        }
    }

    // ── Block validation ────────────────────────────────────

    fn validate_block(&mut self, block: &AgentBlock) {
        match block {
            AgentBlock::Model(nb) => self.validate_model(nb),
            AgentBlock::Fallback(nb) => self.validate_fallback(nb),
            AgentBlock::Tool(nb) => self.validate_tool(nb),
            AgentBlock::Evidence(ub) => self.validate_evidence(ub),
            AgentBlock::Verify(ub) => self.validate_verify(ub),
            AgentBlock::Safety(ub) => self.validate_safety(ub),
            AgentBlock::Memory(ub) => self.validate_memory(ub),
            AgentBlock::Output(ub) => self.validate_output(ub),
            AgentBlock::Budget(ub) => self.validate_budget(ub),
        }
    }

    // ── Model block ─────────────────────────────────────────

    fn validate_model(&mut self, block: &NamedBlock) {
        let known = &["provider", "name", "temperature"];
        self.check_unknown_fields(&block.fields, known, "model");
        self.require_string_field(&block.fields, "provider", "model", block.span);
        self.require_string_field(&block.fields, "name", "model", block.span);

        if let Some(field) = find_field(&block.fields, "temperature") {
            match &field.value {
                Value::Float(f) => {
                    if f.value < 0.0 || f.value > 2.0 {
                        self.error_with_hint(
                            format!("temperature {} is out of range", f.value),
                            f.span,
                            "temperature must be between 0.0 and 2.0",
                        );
                    }
                }
                other => {
                    self.error_with_hint(
                        format!("expected float for `temperature`, found {}", other.type_name()),
                        field.name.span,
                        "temperature should be a decimal number like 0.7",
                    );
                }
            }
        }
    }

    // ── Fallback block ──────────────────────────────────────

    fn validate_fallback(&mut self, block: &NamedBlock) {
        let known = &["model", "provider", "condition"];
        self.check_unknown_fields(&block.fields, known, "fallback");
        self.require_string_field(&block.fields, "model", "fallback", block.span);
        self.require_string_field(&block.fields, "provider", "fallback", block.span);

        self.check_optional_string(&block.fields, "condition", "fallback");
    }

    // ── Tool block ──────────────────────────────────────────

    fn validate_tool(&mut self, block: &NamedBlock) {
        let known = &["action", "require_evidence"];
        self.check_unknown_fields(&block.fields, known, "tool");
        self.require_string_field(&block.fields, "action", "tool", block.span);

        if let Some(field) = find_field(&block.fields, "action") {
            if let Value::String(s) = &field.value {
                let valid = ["allow", "deny", "require_approval"];
                if !valid.contains(&s.value.as_str()) {
                    self.error_with_hint(
                        format!("invalid tool action `{}`", s.value),
                        s.span,
                        "action must be one of: \"allow\", \"deny\", \"require_approval\"",
                    );
                }
            }
        }

        self.check_optional_bool(&block.fields, "require_evidence", "tool");
    }

    // ── Evidence block ──────────────────────────────────────

    fn validate_evidence(&mut self, block: &UnnamedBlock) {
        let known = &["require", "min_sources"];
        self.check_unknown_fields(&block.fields, known, "evidence");

        self.check_optional_string_list(&block.fields, "require", "evidence");

        if let Some(field) = find_field(&block.fields, "min_sources") {
            match &field.value {
                Value::Int(n) => {
                    if n.value < 0 {
                        self.error(
                            "min_sources must be >= 0",
                            n.span,
                        );
                    }
                }
                other => {
                    self.type_error("min_sources", "integer", other.type_name(), field.name.span);
                }
            }
        }
    }

    // ── Verify block ────────────────────────────────────────

    fn validate_verify(&mut self, block: &UnnamedBlock) {
        let known = &["rule"];
        self.check_unknown_fields(&block.fields, known, "verify");

        for field in &block.fields {
            if field.name.value == "rule"
                && !matches!(field.value, Value::String(_))
            {
                self.type_error("rule", "string", field.value.type_name(), field.name.span);
            }
        }
    }

    // ── Safety block ────────────────────────────────────────

    fn validate_safety(&mut self, block: &UnnamedBlock) {
        let known = &["block"];
        self.check_unknown_fields(&block.fields, known, "safety");

        self.check_optional_string_list(&block.fields, "block", "safety");
    }

    // ── Memory block ────────────────────────────────────────

    fn validate_memory(&mut self, block: &UnnamedBlock) {
        let known = &["read", "write", "max_entries"];
        self.check_unknown_fields(&block.fields, known, "memory");

        if let Some(field) = find_field(&block.fields, "read") {
            if let Value::String(s) = &field.value {
                let valid = ["allow", "deny"];
                if !valid.contains(&s.value.as_str()) {
                    self.error_with_hint(
                        format!("invalid memory read policy `{}`", s.value),
                        s.span,
                        "read must be one of: \"allow\", \"deny\"",
                    );
                }
            } else {
                self.type_error("read", "string", field.value.type_name(), field.name.span);
            }
        }

        if let Some(field) = find_field(&block.fields, "write") {
            if let Value::String(s) = &field.value {
                let valid = ["allow", "deny", "scoped", "append_only"];
                if !valid.contains(&s.value.as_str()) {
                    self.error_with_hint(
                        format!("invalid memory write policy `{}`", s.value),
                        s.span,
                        "write must be one of: \"allow\", \"deny\", \"scoped\", \"append_only\"",
                    );
                }
            } else {
                self.type_error("write", "string", field.value.type_name(), field.name.span);
            }
        }

        self.check_positive_int(&block.fields, "max_entries", "memory");
    }

    // ── Output block ────────────────────────────────────────

    fn validate_output(&mut self, block: &UnnamedBlock) {
        let known = &["format", "max_tokens"];
        self.check_unknown_fields(&block.fields, known, "output");

        self.check_optional_string(&block.fields, "format", "output");
        self.check_positive_int(&block.fields, "max_tokens", "output");
    }

    // ── Budget block ────────────────────────────────────────

    fn validate_budget(&mut self, block: &UnnamedBlock) {
        let known = &["max_tokens", "max_cost_usd", "max_time_seconds"];
        self.check_unknown_fields(&block.fields, known, "budget");

        self.check_positive_int(&block.fields, "max_tokens", "budget");
        self.check_positive_int(&block.fields, "max_time_seconds", "budget");

        if let Some(field) = find_field(&block.fields, "max_cost_usd") {
            match &field.value {
                Value::Float(f) => {
                    if f.value <= 0.0 {
                        self.error("max_cost_usd must be > 0", f.span);
                    }
                }
                Value::Int(n) => {
                    if n.value <= 0 {
                        self.error("max_cost_usd must be > 0", n.span);
                    }
                }
                other => {
                    self.type_error(
                        "max_cost_usd",
                        "float or integer",
                        other.type_name(),
                        field.name.span,
                    );
                }
            }
        }
    }

    // ── Cross-block checks ──────────────────────────────────

    fn check_cross_block_rules(&mut self, agent: &AgentDef) {
        // If any tool has require_evidence: true, warn if no evidence block exists
        let any_requires_evidence = agent.blocks.iter().any(|b| {
            if let AgentBlock::Tool(nb) = b {
                nb.fields.iter().any(|f| {
                    f.name.value == "require_evidence"
                        && matches!(f.value, Value::Bool(ref bv) if bv.value)
                })
            } else {
                false
            }
        });

        let has_evidence = agent
            .blocks
            .iter()
            .any(|b| matches!(b, AgentBlock::Evidence(_)));

        if any_requires_evidence && !has_evidence {
            self.warning(
                "tool requires evidence but no `evidence` block is defined",
                agent.name.span,
            );
        }
    }

    // ── Field helpers ───────────────────────────────────────

    fn check_unknown_fields(&mut self, fields: &[Field], known: &[&str], block_name: &str) {
        // For blocks with repeated fields (like "rule" in verify), we just
        // check the field name is in the known list
        for field in fields {
            if !known.contains(&field.name.value.as_str()) {
                self.error_with_hint(
                    format!("unknown field `{}` in {} block", field.name.value, block_name),
                    field.name.span,
                    format!(
                        "known fields for {}: {}",
                        block_name,
                        known.join(", ")
                    ),
                );
            }
        }
    }

    fn require_string_field(
        &mut self,
        fields: &[Field],
        name: &str,
        block_name: &str,
        block_span: Span,
    ) {
        match find_field(fields, name) {
            Some(field) => {
                if !matches!(field.value, Value::String(_)) {
                    self.type_error(name, "string", field.value.type_name(), field.name.span);
                }
            }
            None => {
                self.error_with_hint(
                    format!("missing required field `{name}` in {block_name} block"),
                    block_span,
                    format!("{block_name} must include `{name}: \"...\"`"),
                );
            }
        }
    }

    fn check_optional_string(&mut self, fields: &[Field], name: &str, block_name: &str) {
        if let Some(field) = find_field(fields, name) {
            if !matches!(field.value, Value::String(_)) {
                self.type_error(name, "string", field.value.type_name(), field.name.span);
            }
        }
        let _ = block_name;
    }

    fn check_optional_bool(&mut self, fields: &[Field], name: &str, block_name: &str) {
        if let Some(field) = find_field(fields, name) {
            if !matches!(field.value, Value::Bool(_)) {
                self.type_error(name, "boolean", field.value.type_name(), field.name.span);
            }
        }
        let _ = block_name;
    }

    fn check_optional_string_list(&mut self, fields: &[Field], name: &str, block_name: &str) {
        if let Some(field) = find_field(fields, name) {
            match &field.value {
                Value::List(lv) => {
                    for (i, elem) in lv.elements.iter().enumerate() {
                        if !matches!(elem, Value::String(_)) {
                            self.error(
                                format!(
                                    "element {} in `{name}` list must be a string, found {}",
                                    i, elem.type_name()
                                ),
                                elem.span(),
                            );
                        }
                    }
                }
                other => {
                    self.type_error(name, "list of strings", other.type_name(), field.name.span);
                }
            }
        }
        let _ = block_name;
    }

    fn check_positive_int(&mut self, fields: &[Field], name: &str, block_name: &str) {
        if let Some(field) = find_field(fields, name) {
            match &field.value {
                Value::Int(n) => {
                    if n.value <= 0 {
                        self.error(
                            format!("`{name}` must be a positive integer"),
                            n.span,
                        );
                    }
                }
                other => {
                    self.type_error(name, "integer", other.type_name(), field.name.span);
                }
            }
        }
        let _ = block_name;
    }

    // ── Diagnostics ─────────────────────────────────────────

    fn error(&mut self, message: impl Into<String>, span: Span) {
        self.diagnostics.push(Diagnostic::error(message, span));
    }

    fn error_with_hint(&mut self, message: impl Into<String>, span: Span, hint: impl Into<String>) {
        self.diagnostics
            .push(Diagnostic::error(message, span).with_hint(hint));
    }

    fn warning(&mut self, message: impl Into<String>, span: Span) {
        self.diagnostics.push(Diagnostic::warning(message, span));
    }

    fn type_error(&mut self, field_name: &str, expected: &str, got: &str, span: Span) {
        self.error(
            format!("expected {expected} for `{field_name}`, found {got}"),
            span,
        );
    }
}

/// Find the first field with the given name.
fn find_field<'a>(fields: &'a [Field], name: &str) -> Option<&'a Field> {
    fields.iter().find(|f| f.name.value == name)
}

/// Convenience function: validate a parsed program.
pub fn validate(program: &Program) -> DiagnosticBag {
    Validator::new().validate(program)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    /// Parse and validate, returning diagnostics.
    fn validate_source(source: &str) -> DiagnosticBag {
        let program = parse(source).expect("source should parse");
        validate(&program)
    }

    /// Assert the source is valid (no errors).
    fn assert_valid(source: &str) {
        let diags = validate_source(source);
        assert!(
            !diags.has_errors(),
            "expected no errors:\n{:?}",
            diags.iter().collect::<Vec<_>>()
        );
    }

    /// Assert the source has at least one error containing the given substring.
    fn assert_error(source: &str, expected_msg: &str) {
        let diags = validate_source(source);
        assert!(
            diags.has_errors(),
            "expected errors but got none"
        );
        let msgs: Vec<_> = diags.iter().map(|d| d.message.as_str()).collect();
        assert!(
            msgs.iter().any(|m| m.contains(expected_msg)),
            "expected error containing {expected_msg:?}, got: {msgs:?}"
        );
    }

    /// Assert the source has a warning containing the given substring.
    fn assert_warning(source: &str, expected_msg: &str) {
        let diags = validate_source(source);
        let warnings: Vec<_> = diags
            .iter()
            .filter(|d| d.severity == crate::diagnostics::report::Severity::Warning)
            .map(|d| d.message.as_str())
            .collect();
        assert!(
            warnings.iter().any(|m| m.contains(expected_msg)),
            "expected warning containing {expected_msg:?}, got: {warnings:?}"
        );
    }

    // ── Valid programs ──────────────────────────────────────

    #[test]
    fn valid_minimal_agent() {
        assert_valid(
            r#"agent A {
                model primary {
                    provider: "openai"
                    name: "gpt-4"
                }
            }"#,
        );
    }

    #[test]
    fn valid_full_agent() {
        assert_valid(
            r#"agent Full {
                model primary {
                    provider: "anthropic"
                    name: "claude-sonnet-4-20250514"
                    temperature: 0.3
                }
                fallback backup {
                    model: "gpt-4"
                    provider: "openai"
                    condition: "primary_unavailable"
                }
                tool file_read {
                    action: "allow"
                    require_evidence: true
                }
                tool shell_exec {
                    action: "deny"
                }
                evidence {
                    require: ["source_file"]
                    min_sources: 1
                }
                verify {
                    rule: "no_hallucinated_imports"
                    rule: "no_undefined_variables"
                }
                safety {
                    block: ["harmful_content"]
                }
                memory {
                    read: "allow"
                    write: "scoped"
                    max_entries: 100
                }
                output {
                    format: "markdown"
                    max_tokens: 8192
                }
                budget {
                    max_tokens: 100000
                    max_cost_usd: 1.00
                    max_time_seconds: 120
                }
            }"#,
        );
    }

    // ── Missing required blocks / fields ────────────────────

    #[test]
    fn error_missing_model_block() {
        assert_error(
            r#"agent A { safety { block: ["x"] } }"#,
            "missing a required `model` block",
        );
    }

    #[test]
    fn error_missing_model_provider() {
        assert_error(
            r#"agent A {
                model primary {
                    name: "gpt-4"
                }
            }"#,
            "missing required field `provider`",
        );
    }

    #[test]
    fn error_missing_model_name() {
        assert_error(
            r#"agent A {
                model primary {
                    provider: "openai"
                }
            }"#,
            "missing required field `name`",
        );
    }

    #[test]
    fn error_missing_tool_action() {
        assert_error(
            r#"agent A {
                model m { provider: "x" name: "y" }
                tool t { require_evidence: true }
            }"#,
            "missing required field `action`",
        );
    }

    #[test]
    fn error_missing_fallback_model() {
        assert_error(
            r#"agent A {
                model m { provider: "x" name: "y" }
                fallback f { provider: "x" }
            }"#,
            "missing required field `model`",
        );
    }

    #[test]
    fn error_missing_fallback_provider() {
        assert_error(
            r#"agent A {
                model m { provider: "x" name: "y" }
                fallback f { model: "x" }
            }"#,
            "missing required field `provider`",
        );
    }

    // ── Duplicate blocks ────────────────────────────────────

    #[test]
    fn error_duplicate_safety_block() {
        assert_error(
            r#"agent A {
                model m { provider: "x" name: "y" }
                safety { block: ["a"] }
                safety { block: ["b"] }
            }"#,
            "duplicate `safety` block",
        );
    }

    #[test]
    fn error_duplicate_output_block() {
        assert_error(
            r#"agent A {
                model m { provider: "x" name: "y" }
                output { format: "json" }
                output { format: "markdown" }
            }"#,
            "duplicate `output` block",
        );
    }

    #[test]
    fn error_duplicate_model_names() {
        assert_error(
            r#"agent A {
                model primary { provider: "x" name: "y" }
                model primary { provider: "z" name: "w" }
            }"#,
            "duplicate model name `primary`",
        );
    }

    #[test]
    fn error_duplicate_tool_names() {
        assert_error(
            r#"agent A {
                model m { provider: "x" name: "y" }
                tool read { action: "allow" }
                tool read { action: "deny" }
            }"#,
            "duplicate tool name `read`",
        );
    }

    // ── Unknown fields ──────────────────────────────────────

    #[test]
    fn error_unknown_field_in_model() {
        assert_error(
            r#"agent A {
                model m {
                    provider: "x"
                    name: "y"
                    flavor: "spicy"
                }
            }"#,
            "unknown field `flavor` in model block",
        );
    }

    #[test]
    fn error_unknown_field_in_tool() {
        assert_error(
            r#"agent A {
                model m { provider: "x" name: "y" }
                tool t { action: "allow" timeout: 30 }
            }"#,
            "unknown field `timeout` in tool block",
        );
    }

    // ── Type errors ─────────────────────────────────────────

    #[test]
    fn error_wrong_type_provider() {
        assert_error(
            r#"agent A {
                model m { provider: 42 name: "y" }
            }"#,
            "expected string for `provider`, found integer",
        );
    }

    #[test]
    fn error_wrong_type_require_evidence() {
        assert_error(
            r#"agent A {
                model m { provider: "x" name: "y" }
                tool t { action: "allow" require_evidence: "yes" }
            }"#,
            "expected boolean for `require_evidence`, found string",
        );
    }

    #[test]
    fn error_wrong_type_max_tokens() {
        assert_error(
            r#"agent A {
                model m { provider: "x" name: "y" }
                output { max_tokens: "many" }
            }"#,
            "expected integer for `max_tokens`, found string",
        );
    }

    #[test]
    fn error_wrong_type_verify_rule() {
        assert_error(
            r#"agent A {
                model m { provider: "x" name: "y" }
                verify { rule: 42 }
            }"#,
            "expected string for `rule`, found integer",
        );
    }

    #[test]
    fn error_safety_block_not_list() {
        assert_error(
            r#"agent A {
                model m { provider: "x" name: "y" }
                safety { block: "harmful" }
            }"#,
            "expected list of strings for `block`, found string",
        );
    }

    #[test]
    fn error_list_element_wrong_type() {
        assert_error(
            r#"agent A {
                model m { provider: "x" name: "y" }
                safety { block: ["ok", 42] }
            }"#,
            "element 1 in `block` list must be a string",
        );
    }

    // ── Value range errors ──────────────────────────────────

    #[test]
    fn error_temperature_too_high() {
        assert_error(
            r#"agent A {
                model m { provider: "x" name: "y" temperature: 3.0 }
            }"#,
            "temperature 3 is out of range",
        );
    }

    #[test]
    fn error_temperature_wrong_type() {
        // Negative floats aren't supported by the lexer (no unary minus),
        // so test that a non-float type is caught.
        assert_error(
            r#"agent A {
                model m { provider: "x" name: "y" temperature: "hot" }
            }"#,
            "expected float for `temperature`, found string",
        );
    }

    #[test]
    fn error_invalid_tool_action() {
        assert_error(
            r#"agent A {
                model m { provider: "x" name: "y" }
                tool t { action: "maybe" }
            }"#,
            "invalid tool action `maybe`",
        );
    }

    #[test]
    fn error_invalid_memory_read() {
        assert_error(
            r#"agent A {
                model m { provider: "x" name: "y" }
                memory { read: "sometimes" }
            }"#,
            "invalid memory read policy",
        );
    }

    #[test]
    fn error_invalid_memory_write() {
        assert_error(
            r#"agent A {
                model m { provider: "x" name: "y" }
                memory { write: "whenever" }
            }"#,
            "invalid memory write policy",
        );
    }

    #[test]
    fn error_max_entries_zero() {
        assert_error(
            r#"agent A {
                model m { provider: "x" name: "y" }
                memory { max_entries: 0 }
            }"#,
            "must be a positive integer",
        );
    }

    #[test]
    fn error_budget_max_tokens_negative() {
        assert_error(
            r#"agent A {
                model m { provider: "x" name: "y" }
                budget { max_tokens: 0 }
            }"#,
            "must be a positive integer",
        );
    }

    #[test]
    fn error_budget_cost_zero() {
        assert_error(
            r#"agent A {
                model m { provider: "x" name: "y" }
                budget { max_cost_usd: 0.0 }
            }"#,
            "max_cost_usd must be > 0",
        );
    }

    // ── Cross-block warnings ────────────────────────────────

    #[test]
    fn warning_require_evidence_without_evidence_block() {
        assert_warning(
            r#"agent A {
                model m { provider: "x" name: "y" }
                tool t { action: "allow" require_evidence: true }
            }"#,
            "tool requires evidence but no `evidence` block",
        );
    }

    #[test]
    fn no_warning_when_evidence_block_present() {
        let diags = validate_source(
            r#"agent A {
                model m { provider: "x" name: "y" }
                tool t { action: "allow" require_evidence: true }
                evidence { require: ["url"] }
            }"#,
        );
        let warnings: Vec<_> = diags
            .iter()
            .filter(|d| d.severity == crate::diagnostics::report::Severity::Warning)
            .collect();
        assert!(warnings.is_empty(), "expected no warnings: {warnings:?}");
    }

    // ── Boundary cases ──────────────────────────────────────

    #[test]
    fn valid_temperature_at_boundaries() {
        assert_valid(
            r#"agent A {
                model m { provider: "x" name: "y" temperature: 0.0 }
            }"#,
        );
        assert_valid(
            r#"agent A {
                model m { provider: "x" name: "y" temperature: 2.0 }
            }"#,
        );
    }

    #[test]
    fn valid_empty_safety_list() {
        assert_valid(
            r#"agent A {
                model m { provider: "x" name: "y" }
                safety { block: [] }
            }"#,
        );
    }

    #[test]
    fn valid_multiple_tools_different_names() {
        assert_valid(
            r#"agent A {
                model m { provider: "x" name: "y" }
                tool read { action: "allow" }
                tool write { action: "deny" }
            }"#,
        );
    }

    #[test]
    fn valid_multiple_models_different_names() {
        assert_valid(
            r#"agent A {
                model primary { provider: "x" name: "y" }
                model secondary { provider: "z" name: "w" }
            }"#,
        );
    }
}
