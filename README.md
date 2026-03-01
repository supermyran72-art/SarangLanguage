# Sarang

**A high-performance AI-native language for trustworthy AI systems.**

Sarang is a domain-specific language for defining AI agent policies, tool orchestration rules, verification requirements, safety constraints, and structured AI behavior. It compiles to a typed Policy IR that can be serialized to JSON and consumed by any AI runtime or evaluation harness.

## Status

**v0.1.0** — under active development.

## What Sarang Does

Sarang lets you express:

- **Agent definitions** — what an AI agent is, which models it uses, how it falls back
- **Tool access policies** — which tools are allowed, denied, or require approval
- **Evidence requirements** — what citations or sources an agent must provide
- **Verification rules** — checks that agent output must satisfy
- **Safety constraints** — content and behavior that must be blocked
- **Memory rules** — how an agent reads and writes persistent state
- **Output contracts** — format, length, and structure requirements
- **Budgets** — cost, token, and time limits

## Quick Start

```bash
# Build
cargo build

# Check a Sarang file for errors
cargo run -- check examples/basic_agent.sarang

# Compile to Policy IR JSON
cargo run -- compile examples/coding_assistant.sarang

# Print version
cargo run -- version
```

## Example

```sarang
agent CodingAssistant {
    model primary {
        provider: "anthropic"
        name: "claude-sonnet-4-20250514"
        temperature: 0.3
    }

    tool file_read {
        action: "allow"
        require_evidence: true
    }

    tool shell_exec {
        action: "deny"
    }

    safety {
        block: ["arbitrary_code_execution"]
    }

    output {
        format: "markdown"
        max_tokens: 8192
    }
}
```

## Architecture

```
.sarang source → Lexer → Parser → AST → Validator → Policy IR → JSON
```

Implemented in Rust for speed and reliability.

## License

MIT
