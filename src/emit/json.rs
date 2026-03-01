use crate::ir::PolicyIr;
use std::io;

/// Serialize a Policy IR to a compact JSON string.
pub fn emit_json(ir: &PolicyIr) -> serde_json::Result<String> {
    serde_json::to_string(ir)
}

/// Serialize a Policy IR to a pretty-printed JSON string.
pub fn emit_json_pretty(ir: &PolicyIr) -> serde_json::Result<String> {
    serde_json::to_string_pretty(ir)
}

/// Serialize a Policy IR as compact JSON to any `io::Write` sink.
pub fn emit_json_to_writer<W: io::Write>(ir: &PolicyIr, writer: W) -> serde_json::Result<()> {
    serde_json::to_writer(writer, ir)
}

/// Serialize a Policy IR as pretty-printed JSON to any `io::Write` sink.
pub fn emit_json_pretty_to_writer<W: io::Write>(ir: &PolicyIr, writer: W) -> serde_json::Result<()> {
    serde_json::to_writer_pretty(writer, ir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::*;

    fn test_ir() -> PolicyIr {
        PolicyIr::new(AgentPolicy {
            name: "TestAgent".into(),
            models: vec![ModelConfig {
                name: "primary".into(),
                provider: "openai".into(),
                model_name: "gpt-4".into(),
                temperature: Some(0.5),
            }],
            fallbacks: vec![],
            tools: vec![ToolPolicy {
                name: "search".into(),
                action: ToolAction::Allow,
                require_evidence: Some(true),
            }],
            evidence: None,
            verify: None,
            safety: Some(SafetyPolicy {
                block: vec!["harmful".into()],
            }),
            memory: None,
            output: None,
            budget: None,
        })
    }

    #[test]
    fn emit_compact_json() {
        let ir = test_ir();
        let json = emit_json(&ir).unwrap();
        assert!(!json.contains('\n'));
        assert!(json.contains("\"name\":\"TestAgent\""));
        assert!(json.contains("\"action\":\"allow\""));
    }

    #[test]
    fn emit_pretty_json() {
        let json = emit_json_pretty(&test_ir()).unwrap();
        assert!(json.contains('\n'));
        assert!(json.contains("  "));
        assert!(json.contains("\"name\": \"TestAgent\""));
    }

    #[test]
    fn emit_to_writer() {
        let mut buf = Vec::new();
        emit_json_to_writer(&test_ir(), &mut buf).unwrap();
        let json = String::from_utf8(buf).unwrap();
        assert!(json.contains("TestAgent"));
    }

    #[test]
    fn emit_pretty_to_writer() {
        let mut buf = Vec::new();
        emit_json_pretty_to_writer(&test_ir(), &mut buf).unwrap();
        let json = String::from_utf8(buf).unwrap();
        assert!(json.contains('\n'));
        assert!(json.contains("TestAgent"));
    }

    #[test]
    fn emit_roundtrip() {
        let ir = test_ir();
        let json = emit_json(&ir).unwrap();
        let parsed: PolicyIr = serde_json::from_str(&json).unwrap();
        assert_eq!(ir, parsed);
    }

    #[test]
    fn emit_omits_empty_optional_fields() {
        let ir = PolicyIr::new(AgentPolicy {
            name: "Minimal".into(),
            models: vec![ModelConfig {
                name: "m".into(),
                provider: "p".into(),
                model_name: "n".into(),
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
        });
        let json = emit_json_pretty(&ir).unwrap();
        assert!(!json.contains("fallbacks"));
        assert!(!json.contains("tools"));
        assert!(!json.contains("evidence"));
        assert!(!json.contains("temperature"));
    }
}
