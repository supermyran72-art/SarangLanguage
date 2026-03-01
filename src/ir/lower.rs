use crate::ir::policy_ir::*;
use crate::parser::ast::*;
use std::fmt;

/// Errors that can occur during lowering.
///
/// These indicate compiler bugs (the validator should have caught user errors),
/// but we return `Result` instead of panicking for robustness.
#[derive(Debug, Clone, PartialEq)]
pub struct LoweringError {
    pub message: String,
}

impl LoweringError {
    fn new(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
        }
    }
}

impl fmt::Display for LoweringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "lowering error: {}", self.message)
    }
}

impl std::error::Error for LoweringError {}

type Result<T> = std::result::Result<T, LoweringError>;

/// Lower a validated AST into the typed Policy IR.
///
/// The AST must have passed validation. If the AST contains structures
/// the validator would have rejected, this function returns a `LoweringError`.
pub fn lower(program: &Program) -> Result<PolicyIr> {
    let agent = lower_agent(&program.agent)?;
    Ok(PolicyIr::new(agent))
}

fn lower_agent(agent: &AgentDef) -> Result<AgentPolicy> {
    let mut models = Vec::new();
    let mut fallbacks = Vec::new();
    let mut tools = Vec::new();
    let mut evidence = None;
    let mut verify = None;
    let mut safety = None;
    let mut memory = None;
    let mut output = None;
    let mut budget = None;

    for block in &agent.blocks {
        match block {
            AgentBlock::Model(nb) => models.push(lower_model(nb)?),
            AgentBlock::Fallback(nb) => fallbacks.push(lower_fallback(nb)?),
            AgentBlock::Tool(nb) => tools.push(lower_tool(nb)?),
            AgentBlock::Evidence(ub) => evidence = Some(lower_evidence(ub)?),
            AgentBlock::Verify(ub) => verify = Some(lower_verify(ub)),
            AgentBlock::Safety(ub) => safety = Some(lower_safety(ub)?),
            AgentBlock::Memory(ub) => memory = Some(lower_memory(ub)?),
            AgentBlock::Output(ub) => output = Some(lower_output(ub)?),
            AgentBlock::Budget(ub) => budget = Some(lower_budget(ub)?),
        }
    }

    Ok(AgentPolicy {
        name: agent.name.value.clone(),
        models,
        fallbacks,
        tools,
        evidence,
        verify,
        safety,
        memory,
        output,
        budget,
    })
}

// ── Block lowering helpers ────────────────────────────────────

fn lower_model(block: &NamedBlock) -> Result<ModelConfig> {
    let provider = require_string(&block.fields, "provider", "model")?;
    let model_name = require_string(&block.fields, "name", "model")?;
    let temperature = optional_float(&block.fields, "temperature")?;

    Ok(ModelConfig {
        name: block.name.value.clone(),
        provider,
        model_name,
        temperature,
    })
}

fn lower_fallback(block: &NamedBlock) -> Result<FallbackConfig> {
    let model = require_string(&block.fields, "model", "fallback")?;
    let provider = require_string(&block.fields, "provider", "fallback")?;
    let condition = optional_string(&block.fields, "condition");

    Ok(FallbackConfig {
        name: block.name.value.clone(),
        model,
        provider,
        condition,
    })
}

fn lower_tool(block: &NamedBlock) -> Result<ToolPolicy> {
    let action_str = require_string(&block.fields, "action", "tool")?;
    let action = ToolAction::from_str(&action_str).ok_or_else(|| {
        LoweringError::new(format!("invalid tool action `{action_str}`"))
    })?;
    let require_evidence = optional_bool(&block.fields, "require_evidence");

    Ok(ToolPolicy {
        name: block.name.value.clone(),
        action,
        require_evidence,
    })
}

fn lower_evidence(block: &UnnamedBlock) -> Result<EvidencePolicy> {
    let require = optional_string_list(&block.fields, "require")?;
    let min_sources = optional_int(&block.fields, "min_sources")?;

    Ok(EvidencePolicy {
        require,
        min_sources,
    })
}

fn lower_verify(block: &UnnamedBlock) -> VerifyPolicy {
    // `verify` uses repeated `rule:` fields, not a single list
    let rules = block
        .fields
        .iter()
        .filter(|f| f.name.value == "rule")
        .filter_map(|f| match &f.value {
            Value::String(s) => Some(s.value.clone()),
            _ => None,
        })
        .collect();

    VerifyPolicy { rules }
}

fn lower_safety(block: &UnnamedBlock) -> Result<SafetyPolicy> {
    let items = optional_string_list(&block.fields, "block")?;
    Ok(SafetyPolicy { block: items })
}

fn lower_memory(block: &UnnamedBlock) -> Result<MemoryPolicy> {
    let read = match optional_string(&block.fields, "read") {
        Some(s) => Some(MemoryAccess::from_str(&s).ok_or_else(|| {
            LoweringError::new(format!("invalid memory read policy `{s}`"))
        })?),
        None => None,
    };

    let write = match optional_string(&block.fields, "write") {
        Some(s) => Some(MemoryWriteAccess::from_str(&s).ok_or_else(|| {
            LoweringError::new(format!("invalid memory write policy `{s}`"))
        })?),
        None => None,
    };

    let max_entries = optional_int(&block.fields, "max_entries")?;

    Ok(MemoryPolicy {
        read,
        write,
        max_entries,
    })
}

fn lower_output(block: &UnnamedBlock) -> Result<OutputPolicy> {
    let format = optional_string(&block.fields, "format");
    let max_tokens = optional_int(&block.fields, "max_tokens")?;

    Ok(OutputPolicy { format, max_tokens })
}

fn lower_budget(block: &UnnamedBlock) -> Result<BudgetPolicy> {
    let max_tokens = optional_int(&block.fields, "max_tokens")?;
    let max_cost_usd = optional_numeric_as_f64(&block.fields, "max_cost_usd")?;
    let max_time_seconds = optional_int(&block.fields, "max_time_seconds")?;

    Ok(BudgetPolicy {
        max_tokens,
        max_cost_usd,
        max_time_seconds,
    })
}

// ── Field extraction helpers ──────────────────────────────────

fn find_field<'a>(fields: &'a [Field], name: &str) -> Option<&'a Field> {
    fields.iter().find(|f| f.name.value == name)
}

fn require_string(fields: &[Field], name: &str, block: &str) -> Result<String> {
    let field = find_field(fields, name).ok_or_else(|| {
        LoweringError::new(format!("missing required field `{name}` in {block} block"))
    })?;
    match &field.value {
        Value::String(s) => Ok(s.value.clone()),
        other => Err(LoweringError::new(format!(
            "expected string for `{name}`, found {}",
            other.type_name()
        ))),
    }
}

fn optional_string(fields: &[Field], name: &str) -> Option<String> {
    find_field(fields, name).and_then(|f| match &f.value {
        Value::String(s) => Some(s.value.clone()),
        _ => None,
    })
}

fn optional_bool(fields: &[Field], name: &str) -> Option<bool> {
    find_field(fields, name).and_then(|f| match &f.value {
        Value::Bool(b) => Some(b.value),
        _ => None,
    })
}

fn optional_int(fields: &[Field], name: &str) -> Result<Option<i64>> {
    match find_field(fields, name) {
        Some(f) => match &f.value {
            Value::Int(n) => Ok(Some(n.value)),
            other => Err(LoweringError::new(format!(
                "expected integer for `{name}`, found {}",
                other.type_name()
            ))),
        },
        None => Ok(None),
    }
}

fn optional_float(fields: &[Field], name: &str) -> Result<Option<f64>> {
    match find_field(fields, name) {
        Some(f) => match &f.value {
            Value::Float(n) => Ok(Some(n.value)),
            other => Err(LoweringError::new(format!(
                "expected float for `{name}`, found {}",
                other.type_name()
            ))),
        },
        None => Ok(None),
    }
}

fn optional_numeric_as_f64(fields: &[Field], name: &str) -> Result<Option<f64>> {
    match find_field(fields, name) {
        Some(f) => match &f.value {
            Value::Float(n) => Ok(Some(n.value)),
            Value::Int(n) => Ok(Some(n.value as f64)),
            other => Err(LoweringError::new(format!(
                "expected number for `{name}`, found {}",
                other.type_name()
            ))),
        },
        None => Ok(None),
    }
}

fn optional_string_list(fields: &[Field], name: &str) -> Result<Vec<String>> {
    match find_field(fields, name) {
        Some(f) => match &f.value {
            Value::List(lv) => {
                let mut result = Vec::with_capacity(lv.elements.len());
                for elem in &lv.elements {
                    match elem {
                        Value::String(s) => result.push(s.value.clone()),
                        other => {
                            return Err(LoweringError::new(format!(
                                "expected string in `{name}` list, found {}",
                                other.type_name()
                            )));
                        }
                    }
                }
                Ok(result)
            }
            other => Err(LoweringError::new(format!(
                "expected list for `{name}`, found {}",
                other.type_name()
            ))),
        },
        None => Ok(vec![]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    /// Parse, validate, and lower a source string.
    fn lower_source(source: &str) -> PolicyIr {
        let program = parse(source).expect("should parse");
        let diags = crate::validator::validate(&program);
        assert!(
            !diags.has_errors(),
            "validation errors: {:?}",
            diags.iter().collect::<Vec<_>>()
        );
        lower(&program).expect("should lower")
    }

    // ── Minimal agent ───────────────────────────────────────

    #[test]
    fn lower_minimal_agent() {
        let ir = lower_source(
            r#"agent Minimal {
                model primary {
                    provider: "openai"
                    name: "gpt-4"
                }
            }"#,
        );

        assert_eq!(ir.version, "0.1.0");
        assert_eq!(ir.agent.name, "Minimal");
        assert_eq!(ir.agent.models.len(), 1);
        assert_eq!(ir.agent.models[0].name, "primary");
        assert_eq!(ir.agent.models[0].provider, "openai");
        assert_eq!(ir.agent.models[0].model_name, "gpt-4");
        assert_eq!(ir.agent.models[0].temperature, None);
        assert!(ir.agent.fallbacks.is_empty());
        assert!(ir.agent.tools.is_empty());
        assert!(ir.agent.evidence.is_none());
        assert!(ir.agent.verify.is_none());
        assert!(ir.agent.safety.is_none());
        assert!(ir.agent.memory.is_none());
        assert!(ir.agent.output.is_none());
        assert!(ir.agent.budget.is_none());
    }

    // ── Full agent ──────────────────────────────────────────

    #[test]
    fn lower_full_agent() {
        let ir = lower_source(
            r#"agent CodingAssistant {
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
                    require: ["source_file", "line_number"]
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

        assert_eq!(ir.agent.name, "CodingAssistant");

        // Model
        assert_eq!(ir.agent.models[0].provider, "anthropic");
        assert_eq!(ir.agent.models[0].model_name, "claude-sonnet-4-20250514");
        assert_eq!(ir.agent.models[0].temperature, Some(0.3));

        // Fallback
        assert_eq!(ir.agent.fallbacks.len(), 1);
        assert_eq!(ir.agent.fallbacks[0].name, "backup");
        assert_eq!(ir.agent.fallbacks[0].model, "gpt-4");
        assert_eq!(ir.agent.fallbacks[0].provider, "openai");
        assert_eq!(
            ir.agent.fallbacks[0].condition,
            Some("primary_unavailable".into())
        );

        // Tools
        assert_eq!(ir.agent.tools.len(), 2);
        assert_eq!(ir.agent.tools[0].name, "file_read");
        assert_eq!(ir.agent.tools[0].action, ToolAction::Allow);
        assert_eq!(ir.agent.tools[0].require_evidence, Some(true));
        assert_eq!(ir.agent.tools[1].name, "shell_exec");
        assert_eq!(ir.agent.tools[1].action, ToolAction::Deny);
        assert_eq!(ir.agent.tools[1].require_evidence, None);

        // Evidence
        let ev = ir.agent.evidence.as_ref().unwrap();
        assert_eq!(ev.require, vec!["source_file", "line_number"]);
        assert_eq!(ev.min_sources, Some(1));

        // Verify
        let vf = ir.agent.verify.as_ref().unwrap();
        assert_eq!(
            vf.rules,
            vec!["no_hallucinated_imports", "no_undefined_variables"]
        );

        // Safety
        let sf = ir.agent.safety.as_ref().unwrap();
        assert_eq!(sf.block, vec!["harmful_content"]);

        // Memory
        let mem = ir.agent.memory.as_ref().unwrap();
        assert_eq!(mem.read, Some(MemoryAccess::Allow));
        assert_eq!(mem.write, Some(MemoryWriteAccess::Scoped));
        assert_eq!(mem.max_entries, Some(100));

        // Output
        let out = ir.agent.output.as_ref().unwrap();
        assert_eq!(out.format, Some("markdown".into()));
        assert_eq!(out.max_tokens, Some(8192));

        // Budget
        let bud = ir.agent.budget.as_ref().unwrap();
        assert_eq!(bud.max_tokens, Some(100000));
        assert_eq!(bud.max_cost_usd, Some(1.0));
        assert_eq!(bud.max_time_seconds, Some(120));
    }

    // ── Multiple models ─────────────────────────────────────

    #[test]
    fn lower_multiple_models() {
        let ir = lower_source(
            r#"agent Multi {
                model primary {
                    provider: "anthropic"
                    name: "claude-sonnet-4-20250514"
                }
                model secondary {
                    provider: "openai"
                    name: "gpt-4"
                    temperature: 0.7
                }
            }"#,
        );

        assert_eq!(ir.agent.models.len(), 2);
        assert_eq!(ir.agent.models[0].name, "primary");
        assert_eq!(ir.agent.models[0].temperature, None);
        assert_eq!(ir.agent.models[1].name, "secondary");
        assert_eq!(ir.agent.models[1].temperature, Some(0.7));
    }

    // ── Tool actions ────────────────────────────────────────

    #[test]
    fn lower_require_approval_action() {
        let ir = lower_source(
            r#"agent A {
                model m { provider: "x" name: "y" }
                tool write_file {
                    action: "require_approval"
                }
            }"#,
        );

        assert_eq!(ir.agent.tools[0].action, ToolAction::RequireApproval);
    }

    // ── Memory write modes ──────────────────────────────────

    #[test]
    fn lower_memory_append_only() {
        let ir = lower_source(
            r#"agent A {
                model m { provider: "x" name: "y" }
                memory {
                    write: "append_only"
                }
            }"#,
        );

        let mem = ir.agent.memory.unwrap();
        assert_eq!(mem.write, Some(MemoryWriteAccess::AppendOnly));
        assert_eq!(mem.read, None);
        assert_eq!(mem.max_entries, None);
    }

    #[test]
    fn lower_memory_deny_both() {
        let ir = lower_source(
            r#"agent A {
                model m { provider: "x" name: "y" }
                memory {
                    read: "deny"
                    write: "deny"
                }
            }"#,
        );

        let mem = ir.agent.memory.unwrap();
        assert_eq!(mem.read, Some(MemoryAccess::Deny));
        assert_eq!(mem.write, Some(MemoryWriteAccess::Deny));
    }

    // ── Empty optional blocks ───────────────────────────────

    #[test]
    fn lower_empty_safety_list() {
        let ir = lower_source(
            r#"agent A {
                model m { provider: "x" name: "y" }
                safety { block: [] }
            }"#,
        );

        let sf = ir.agent.safety.unwrap();
        assert!(sf.block.is_empty());
    }

    #[test]
    fn lower_verify_no_rules() {
        let ir = lower_source(
            r#"agent A {
                model m { provider: "x" name: "y" }
                verify { }
            }"#,
        );

        let vf = ir.agent.verify.unwrap();
        assert!(vf.rules.is_empty());
    }

    // ── Evidence without optional fields ────────────────────

    #[test]
    fn lower_evidence_require_only() {
        let ir = lower_source(
            r#"agent A {
                model m { provider: "x" name: "y" }
                evidence {
                    require: ["url"]
                }
            }"#,
        );

        let ev = ir.agent.evidence.unwrap();
        assert_eq!(ev.require, vec!["url"]);
        assert_eq!(ev.min_sources, None);
    }

    // ── Budget with int cost ────────────────────────────────

    #[test]
    fn lower_budget_int_cost() {
        let ir = lower_source(
            r#"agent A {
                model m { provider: "x" name: "y" }
                budget {
                    max_cost_usd: 5
                }
            }"#,
        );

        let bud = ir.agent.budget.unwrap();
        assert_eq!(bud.max_cost_usd, Some(5.0));
        assert_eq!(bud.max_tokens, None);
        assert_eq!(bud.max_time_seconds, None);
    }

    // ── Fallback without optional condition ─────────────────

    #[test]
    fn lower_fallback_no_condition() {
        let ir = lower_source(
            r#"agent A {
                model m { provider: "x" name: "y" }
                fallback f {
                    model: "gpt-4"
                    provider: "openai"
                }
            }"#,
        );

        assert_eq!(ir.agent.fallbacks[0].condition, None);
    }

    // ── JSON roundtrip from source ──────────────────────────

    #[test]
    fn lower_then_serialize_roundtrip() {
        let ir = lower_source(
            r#"agent Test {
                model primary {
                    provider: "openai"
                    name: "gpt-4"
                    temperature: 0.5
                }
                tool search {
                    action: "allow"
                    require_evidence: true
                }
                safety {
                    block: ["harmful"]
                }
            }"#,
        );

        let json = serde_json::to_string(&ir).unwrap();
        let roundtrip: PolicyIr = serde_json::from_str(&json).unwrap();
        assert_eq!(ir, roundtrip);
    }

    // ── Error cases (should not happen after validation) ────

    #[test]
    fn lower_error_display() {
        let err = LoweringError::new("test error");
        assert_eq!(err.to_string(), "lowering error: test error");
    }
}
