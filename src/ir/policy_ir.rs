use serde::{Deserialize, Serialize};

/// The top-level Policy IR — a fully typed, serializable representation
/// of a validated Sarang agent policy.
///
/// This is the output of the lowering pass and the input to JSON emission.
/// It contains no source spans or syntax artifacts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PolicyIr {
    /// Schema version for forward compatibility.
    pub version: String,
    /// The agent policy.
    pub agent: AgentPolicy,
}

impl PolicyIr {
    pub fn new(agent: AgentPolicy) -> Self {
        Self {
            version: "0.1.0".to_owned(),
            agent,
        }
    }
}

/// A complete agent policy definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentPolicy {
    /// The agent's name.
    pub name: String,
    /// Model configurations (at least one).
    pub models: Vec<ModelConfig>,
    /// Fallback model chain (zero or more).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub fallbacks: Vec<FallbackConfig>,
    /// Tool access policies (zero or more).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub tools: Vec<ToolPolicy>,
    /// Evidence requirements (optional).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub evidence: Option<EvidencePolicy>,
    /// Verification rules (optional).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub verify: Option<VerifyPolicy>,
    /// Safety constraints (optional).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub safety: Option<SafetyPolicy>,
    /// Memory access rules (optional).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub memory: Option<MemoryPolicy>,
    /// Output format contract (optional).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub output: Option<OutputPolicy>,
    /// Cost/token/time budget (optional).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub budget: Option<BudgetPolicy>,
}

// ── Model ───────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelConfig {
    /// The model's local name (e.g., "primary").
    pub name: String,
    /// Provider identifier (e.g., "openai", "anthropic").
    pub provider: String,
    /// Model identifier (e.g., "gpt-4", "claude-sonnet-4-20250514").
    pub model_name: String,
    /// Sampling temperature (optional, 0.0–2.0).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub temperature: Option<f64>,
}

// ── Fallback ────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FallbackConfig {
    /// The fallback's local name (e.g., "backup").
    pub name: String,
    /// The model to fall back to.
    pub model: String,
    /// Provider for the fallback model.
    pub provider: String,
    /// When to trigger this fallback (optional).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub condition: Option<String>,
}

// ── Tool ────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolPolicy {
    /// The tool's name (e.g., "file_read").
    pub name: String,
    /// Access action for this tool.
    pub action: ToolAction,
    /// Whether this tool requires evidence.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub require_evidence: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolAction {
    Allow,
    Deny,
    RequireApproval,
}

impl ToolAction {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "allow" => Some(ToolAction::Allow),
            "deny" => Some(ToolAction::Deny),
            "require_approval" => Some(ToolAction::RequireApproval),
            _ => None,
        }
    }
}

// ── Evidence ────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidencePolicy {
    /// Required evidence fields (e.g., ["url", "title"]).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub require: Vec<String>,
    /// Minimum number of sources required.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub min_sources: Option<i64>,
}

// ── Verify ──────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VerifyPolicy {
    /// Verification rule names.
    pub rules: Vec<String>,
}

// ── Safety ──────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SafetyPolicy {
    /// Categories of content/behavior to block.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub block: Vec<String>,
}

// ── Memory ──────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryPolicy {
    /// Read access policy.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub read: Option<MemoryAccess>,
    /// Write access policy.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub write: Option<MemoryWriteAccess>,
    /// Maximum number of memory entries.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub max_entries: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryAccess {
    Allow,
    Deny,
}

impl MemoryAccess {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "allow" => Some(MemoryAccess::Allow),
            "deny" => Some(MemoryAccess::Deny),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryWriteAccess {
    Allow,
    Deny,
    Scoped,
    AppendOnly,
}

impl MemoryWriteAccess {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "allow" => Some(MemoryWriteAccess::Allow),
            "deny" => Some(MemoryWriteAccess::Deny),
            "scoped" => Some(MemoryWriteAccess::Scoped),
            "append_only" => Some(MemoryWriteAccess::AppendOnly),
            _ => None,
        }
    }
}

// ── Output ──────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutputPolicy {
    /// Required output format (e.g., "markdown", "json").
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub format: Option<String>,
    /// Maximum output tokens.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub max_tokens: Option<i64>,
}

// ── Budget ──────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BudgetPolicy {
    /// Maximum total tokens.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub max_tokens: Option<i64>,
    /// Maximum cost in USD.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub max_cost_usd: Option<f64>,
    /// Maximum execution time in seconds.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub max_time_seconds: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_policy() -> PolicyIr {
        PolicyIr::new(AgentPolicy {
            name: "TestAgent".into(),
            models: vec![ModelConfig {
                name: "primary".into(),
                provider: "openai".into(),
                model_name: "gpt-4".into(),
                temperature: None,
            }],
            fallbacks: vec![],
            tools: vec![],
            evidence: None,
            verify: None,
            safety: None,
            memory: None,
            output: None,
            budget: None,
        })
    }

    fn full_policy() -> PolicyIr {
        PolicyIr::new(AgentPolicy {
            name: "FullAgent".into(),
            models: vec![ModelConfig {
                name: "primary".into(),
                provider: "anthropic".into(),
                model_name: "claude-sonnet-4-20250514".into(),
                temperature: Some(0.3),
            }],
            fallbacks: vec![FallbackConfig {
                name: "backup".into(),
                model: "gpt-4".into(),
                provider: "openai".into(),
                condition: Some("primary_unavailable".into()),
            }],
            tools: vec![
                ToolPolicy {
                    name: "file_read".into(),
                    action: ToolAction::Allow,
                    require_evidence: Some(true),
                },
                ToolPolicy {
                    name: "shell_exec".into(),
                    action: ToolAction::Deny,
                    require_evidence: None,
                },
            ],
            evidence: Some(EvidencePolicy {
                require: vec!["url".into(), "title".into()],
                min_sources: Some(2),
            }),
            verify: Some(VerifyPolicy {
                rules: vec!["no_hallucinations".into()],
            }),
            safety: Some(SafetyPolicy {
                block: vec!["harmful_content".into()],
            }),
            memory: Some(MemoryPolicy {
                read: Some(MemoryAccess::Allow),
                write: Some(MemoryWriteAccess::Scoped),
                max_entries: Some(100),
            }),
            output: Some(OutputPolicy {
                format: Some("markdown".into()),
                max_tokens: Some(8192),
            }),
            budget: Some(BudgetPolicy {
                max_tokens: Some(100000),
                max_cost_usd: Some(1.0),
                max_time_seconds: Some(120),
            }),
        })
    }

    // ── Construction ────────────────────────────────────────

    #[test]
    fn minimal_policy_has_correct_version() {
        let ir = minimal_policy();
        assert_eq!(ir.version, "0.1.0");
        assert_eq!(ir.agent.name, "TestAgent");
        assert_eq!(ir.agent.models.len(), 1);
    }

    #[test]
    fn full_policy_has_all_sections() {
        let ir = full_policy();
        assert_eq!(ir.agent.models.len(), 1);
        assert_eq!(ir.agent.fallbacks.len(), 1);
        assert_eq!(ir.agent.tools.len(), 2);
        assert!(ir.agent.evidence.is_some());
        assert!(ir.agent.verify.is_some());
        assert!(ir.agent.safety.is_some());
        assert!(ir.agent.memory.is_some());
        assert!(ir.agent.output.is_some());
        assert!(ir.agent.budget.is_some());
    }

    // ── Enum conversions ────────────────────────────────────

    #[test]
    fn tool_action_from_str() {
        assert_eq!(ToolAction::parse("allow"), Some(ToolAction::Allow));
        assert_eq!(ToolAction::parse("deny"), Some(ToolAction::Deny));
        assert_eq!(
            ToolAction::parse("require_approval"),
            Some(ToolAction::RequireApproval)
        );
        assert_eq!(ToolAction::parse("maybe"), None);
    }

    #[test]
    fn memory_access_from_str() {
        assert_eq!(MemoryAccess::parse("allow"), Some(MemoryAccess::Allow));
        assert_eq!(MemoryAccess::parse("deny"), Some(MemoryAccess::Deny));
        assert_eq!(MemoryAccess::parse("scoped"), None);
    }

    #[test]
    fn memory_write_access_from_str() {
        assert_eq!(
            MemoryWriteAccess::parse("allow"),
            Some(MemoryWriteAccess::Allow)
        );
        assert_eq!(
            MemoryWriteAccess::parse("scoped"),
            Some(MemoryWriteAccess::Scoped)
        );
        assert_eq!(
            MemoryWriteAccess::parse("append_only"),
            Some(MemoryWriteAccess::AppendOnly)
        );
        assert_eq!(MemoryWriteAccess::parse("whatever"), None);
    }

    // ── JSON serialization ──────────────────────────────────

    #[test]
    fn minimal_policy_serializes_to_json() {
        let ir = minimal_policy();
        let json = serde_json::to_string_pretty(&ir).unwrap();
        assert!(json.contains("\"version\": \"0.1.0\""));
        assert!(json.contains("\"name\": \"TestAgent\""));
        assert!(json.contains("\"provider\": \"openai\""));
        // Optional empty fields should be omitted
        assert!(!json.contains("\"fallbacks\""));
        assert!(!json.contains("\"tools\""));
        assert!(!json.contains("\"evidence\""));
        assert!(!json.contains("\"safety\""));
    }

    #[test]
    fn full_policy_serializes_to_json() {
        let ir = full_policy();
        let json = serde_json::to_string_pretty(&ir).unwrap();
        assert!(json.contains("\"name\": \"FullAgent\""));
        assert!(json.contains("\"temperature\": 0.3"));
        assert!(json.contains("\"action\": \"allow\""));
        assert!(json.contains("\"action\": \"deny\""));
        assert!(json.contains("\"require_evidence\": true"));
        assert!(json.contains("\"require_approval\"").not());
        assert!(json.contains("\"min_sources\": 2"));
        assert!(json.contains("\"read\": \"allow\""));
        assert!(json.contains("\"write\": \"scoped\""));
        assert!(json.contains("\"max_entries\": 100"));
        assert!(json.contains("\"format\": \"markdown\""));
        assert!(json.contains("\"max_cost_usd\": 1.0"));
    }

    // Helper for negation
    trait Not {
        fn not(self) -> bool;
    }
    impl Not for bool {
        fn not(self) -> bool {
            !self
        }
    }

    #[test]
    fn json_roundtrip() {
        let ir = full_policy();
        let json = serde_json::to_string(&ir).unwrap();
        let roundtrip: PolicyIr = serde_json::from_str(&json).unwrap();
        assert_eq!(ir, roundtrip);
    }

    #[test]
    fn tool_action_serializes_as_snake_case() {
        let json = serde_json::to_string(&ToolAction::RequireApproval).unwrap();
        assert_eq!(json, "\"require_approval\"");
    }

    #[test]
    fn memory_write_serializes_as_snake_case() {
        let json = serde_json::to_string(&MemoryWriteAccess::AppendOnly).unwrap();
        assert_eq!(json, "\"append_only\"");
    }

    #[test]
    fn minimal_json_deserializes_with_defaults() {
        let json = r#"{
            "version": "0.1.0",
            "agent": {
                "name": "Test",
                "models": [{
                    "name": "m",
                    "provider": "x",
                    "model_name": "y"
                }]
            }
        }"#;
        let ir: PolicyIr = serde_json::from_str(json).unwrap();
        assert_eq!(ir.agent.name, "Test");
        assert!(ir.agent.fallbacks.is_empty());
        assert!(ir.agent.tools.is_empty());
        assert!(ir.agent.evidence.is_none());
    }
}
