use crate::common::span::Span;

// ── Spanned wrapper ─────────────────────────────────────────────

/// A value paired with its source location.
#[derive(Debug, Clone, PartialEq)]
pub struct Spanned<T> {
    pub value: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    pub fn new(value: T, span: Span) -> Self {
        Self { value, span }
    }
}

// ── Program (root) ──────────────────────────────────────────────

/// The root of a Sarang program. One agent per file in v0.1.
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub agent: AgentDef,
}

// ── Agent definition ────────────────────────────────────────────

/// `agent <Name> { <blocks...> }`
///
/// The top-level declaration. Contains a name and a sequence of
/// blocks that define the agent's policy.
#[derive(Debug, Clone, PartialEq)]
pub struct AgentDef {
    /// The agent's name (e.g., `CodingAssistant`).
    pub name: Spanned<String>,
    /// The blocks inside the agent body, in source order.
    pub blocks: Vec<AgentBlock>,
    /// Span from `agent` keyword to closing `}`.
    pub span: Span,
}

// ── Agent blocks ────────────────────────────────────────────────

/// A block inside an agent definition.
///
/// There are two shapes:
/// - **Named blocks**: `model <name> { ... }`, `fallback <name> { ... }`, `tool <name> { ... }`
/// - **Unnamed blocks**: `evidence { ... }`, `verify { ... }`, etc.
#[derive(Debug, Clone, PartialEq)]
pub enum AgentBlock {
    // Named blocks (keyword + name + fields)
    Model(NamedBlock),
    Fallback(NamedBlock),
    Tool(NamedBlock),

    // Unnamed (singleton) blocks (keyword + fields)
    Evidence(UnnamedBlock),
    Verify(UnnamedBlock),
    Safety(UnnamedBlock),
    Memory(UnnamedBlock),
    Output(UnnamedBlock),
    Budget(UnnamedBlock),
}

impl AgentBlock {
    /// Returns the keyword name for this block (e.g., "model", "safety").
    pub fn keyword(&self) -> &'static str {
        match self {
            AgentBlock::Model(_) => "model",
            AgentBlock::Fallback(_) => "fallback",
            AgentBlock::Tool(_) => "tool",
            AgentBlock::Evidence(_) => "evidence",
            AgentBlock::Verify(_) => "verify",
            AgentBlock::Safety(_) => "safety",
            AgentBlock::Memory(_) => "memory",
            AgentBlock::Output(_) => "output",
            AgentBlock::Budget(_) => "budget",
        }
    }

    /// Returns the span of this block.
    pub fn span(&self) -> Span {
        match self {
            AgentBlock::Model(b) | AgentBlock::Fallback(b) | AgentBlock::Tool(b) => b.span,
            AgentBlock::Evidence(b)
            | AgentBlock::Verify(b)
            | AgentBlock::Safety(b)
            | AgentBlock::Memory(b)
            | AgentBlock::Output(b)
            | AgentBlock::Budget(b) => b.span,
        }
    }

    /// Returns the fields of this block.
    pub fn fields(&self) -> &[Field] {
        match self {
            AgentBlock::Model(b) | AgentBlock::Fallback(b) | AgentBlock::Tool(b) => &b.fields,
            AgentBlock::Evidence(b)
            | AgentBlock::Verify(b)
            | AgentBlock::Safety(b)
            | AgentBlock::Memory(b)
            | AgentBlock::Output(b)
            | AgentBlock::Budget(b) => &b.fields,
        }
    }

    /// Returns `true` if this is a named block (model, fallback, tool).
    pub fn is_named(&self) -> bool {
        matches!(
            self,
            AgentBlock::Model(_) | AgentBlock::Fallback(_) | AgentBlock::Tool(_)
        )
    }
}

// ── Named and unnamed blocks ────────────────────────────────────

/// A block with a name: `<keyword> <name> { <fields...> }`
///
/// Used for `model`, `fallback`, and `tool` blocks.
#[derive(Debug, Clone, PartialEq)]
pub struct NamedBlock {
    /// The block's name (e.g., `primary`, `file_read`).
    pub name: Spanned<String>,
    /// The fields inside the block body.
    pub fields: Vec<Field>,
    /// Span from keyword to closing `}`.
    pub span: Span,
}

/// A block without a name: `<keyword> { <fields...> }`
///
/// Used for singleton blocks: `evidence`, `verify`, `safety`,
/// `memory`, `output`, `budget`.
#[derive(Debug, Clone, PartialEq)]
pub struct UnnamedBlock {
    /// The fields inside the block body.
    pub fields: Vec<Field>,
    /// Span from keyword to closing `}`.
    pub span: Span,
}

// ── Fields ──────────────────────────────────────────────────────

/// A field assignment: `<name>: <value>`
///
/// Fields appear inside blocks. A block may contain multiple fields
/// with the same name (e.g., repeated `rule:` in a `verify` block).
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    /// The field name (e.g., `provider`, `action`, `max_tokens`).
    pub name: Spanned<String>,
    /// The field value.
    pub value: Value,
    /// Span from field name to end of value.
    pub span: Span,
}

// ── Values ──────────────────────────────────────────────────────

/// A value in a field assignment.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// A string literal: `"openai"`
    String(Spanned<String>),
    /// An integer literal: `8192`
    Int(Spanned<i64>),
    /// A float literal: `0.3`
    Float(Spanned<f64>),
    /// A boolean literal: `true` or `false`
    Bool(Spanned<bool>),
    /// A list of values: `["a", "b", "c"]`
    List(ListValue),
}

impl Value {
    /// Returns the span of this value.
    pub fn span(&self) -> Span {
        match self {
            Value::String(s) => s.span,
            Value::Int(i) => i.span,
            Value::Float(f) => f.span,
            Value::Bool(b) => b.span,
            Value::List(l) => l.span,
        }
    }

    /// Returns a human-readable type name for diagnostics.
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::String(_) => "string",
            Value::Int(_) => "integer",
            Value::Float(_) => "float",
            Value::Bool(_) => "boolean",
            Value::List(_) => "list",
        }
    }
}

/// A list of values: `[<value>, <value>, ...]`
#[derive(Debug, Clone, PartialEq)]
pub struct ListValue {
    /// The elements of the list.
    pub elements: Vec<Value>,
    /// Span from `[` to `]`.
    pub span: Span,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sp(start: usize, end: usize) -> Span {
        Span::new(start, end)
    }

    // ── Spanned ─────────────────────────────────────────────

    #[test]
    fn spanned_wraps_value_and_span() {
        let s = Spanned::new("hello".to_owned(), sp(0, 5));
        assert_eq!(s.value, "hello");
        assert_eq!(s.span, sp(0, 5));
    }

    // ── Value ───────────────────────────────────────────────

    #[test]
    fn value_span_returns_inner_span() {
        let v = Value::String(Spanned::new("x".into(), sp(10, 13)));
        assert_eq!(v.span(), sp(10, 13));

        let v = Value::Int(Spanned::new(42, sp(0, 2)));
        assert_eq!(v.span(), sp(0, 2));

        let v = Value::Float(Spanned::new(3.125, sp(5, 9)));
        assert_eq!(v.span(), sp(5, 9));

        let v = Value::Bool(Spanned::new(true, sp(1, 5)));
        assert_eq!(v.span(), sp(1, 5));

        let v = Value::List(ListValue {
            elements: vec![],
            span: sp(0, 2),
        });
        assert_eq!(v.span(), sp(0, 2));
    }

    #[test]
    fn value_type_name() {
        assert_eq!(
            Value::String(Spanned::new("x".into(), sp(0, 1))).type_name(),
            "string"
        );
        assert_eq!(
            Value::Int(Spanned::new(1, sp(0, 1))).type_name(),
            "integer"
        );
        assert_eq!(
            Value::Float(Spanned::new(1.0, sp(0, 1))).type_name(),
            "float"
        );
        assert_eq!(
            Value::Bool(Spanned::new(true, sp(0, 1))).type_name(),
            "boolean"
        );
        assert_eq!(
            Value::List(ListValue {
                elements: vec![],
                span: sp(0, 1)
            })
            .type_name(),
            "list"
        );
    }

    // ── AgentBlock ──────────────────────────────────────────

    #[test]
    fn agent_block_keyword() {
        let named = NamedBlock {
            name: Spanned::new("x".into(), sp(0, 1)),
            fields: vec![],
            span: sp(0, 5),
        };
        let unnamed = UnnamedBlock {
            fields: vec![],
            span: sp(0, 5),
        };

        assert_eq!(AgentBlock::Model(named.clone()).keyword(), "model");
        assert_eq!(AgentBlock::Fallback(named.clone()).keyword(), "fallback");
        assert_eq!(AgentBlock::Tool(named).keyword(), "tool");
        assert_eq!(AgentBlock::Evidence(unnamed.clone()).keyword(), "evidence");
        assert_eq!(AgentBlock::Verify(unnamed.clone()).keyword(), "verify");
        assert_eq!(AgentBlock::Safety(unnamed.clone()).keyword(), "safety");
        assert_eq!(AgentBlock::Memory(unnamed.clone()).keyword(), "memory");
        assert_eq!(AgentBlock::Output(unnamed.clone()).keyword(), "output");
        assert_eq!(AgentBlock::Budget(unnamed).keyword(), "budget");
    }

    #[test]
    fn agent_block_is_named() {
        let named = NamedBlock {
            name: Spanned::new("x".into(), sp(0, 1)),
            fields: vec![],
            span: sp(0, 5),
        };
        let unnamed = UnnamedBlock {
            fields: vec![],
            span: sp(0, 5),
        };

        assert!(AgentBlock::Model(named.clone()).is_named());
        assert!(AgentBlock::Fallback(named.clone()).is_named());
        assert!(AgentBlock::Tool(named).is_named());
        assert!(!AgentBlock::Evidence(unnamed.clone()).is_named());
        assert!(!AgentBlock::Safety(unnamed).is_named());
    }

    #[test]
    fn agent_block_fields_accessor() {
        let field = Field {
            name: Spanned::new("key".into(), sp(0, 3)),
            value: Value::String(Spanned::new("val".into(), sp(5, 10))),
            span: sp(0, 10),
        };
        let named = NamedBlock {
            name: Spanned::new("blk".into(), sp(0, 3)),
            fields: vec![field.clone()],
            span: sp(0, 20),
        };
        let block = AgentBlock::Model(named);
        assert_eq!(block.fields().len(), 1);
        assert_eq!(block.fields()[0].name.value, "key");
    }

    #[test]
    fn agent_block_span_accessor() {
        let named = NamedBlock {
            name: Spanned::new("x".into(), sp(0, 1)),
            fields: vec![],
            span: sp(10, 50),
        };
        assert_eq!(AgentBlock::Model(named).span(), sp(10, 50));

        let unnamed = UnnamedBlock {
            fields: vec![],
            span: sp(20, 60),
        };
        assert_eq!(AgentBlock::Safety(unnamed).span(), sp(20, 60));
    }

    // ── Full AST construction ───────────────────────────────

    #[test]
    fn construct_minimal_program() {
        let program = Program {
            agent: AgentDef {
                name: Spanned::new("Minimal".into(), sp(6, 13)),
                blocks: vec![AgentBlock::Model(NamedBlock {
                    name: Spanned::new("default".into(), sp(22, 29)),
                    fields: vec![
                        Field {
                            name: Spanned::new("provider".into(), sp(40, 48)),
                            value: Value::String(Spanned::new("openai".into(), sp(50, 58))),
                            span: sp(40, 58),
                        },
                        Field {
                            name: Spanned::new("name".into(), sp(67, 71)),
                            value: Value::String(Spanned::new("gpt-4".into(), sp(73, 80))),
                            span: sp(67, 80),
                        },
                    ],
                    span: sp(16, 86),
                })],
                span: sp(0, 88),
            },
        };

        assert_eq!(program.agent.name.value, "Minimal");
        assert_eq!(program.agent.blocks.len(), 1);
        assert_eq!(program.agent.blocks[0].keyword(), "model");
        assert_eq!(program.agent.blocks[0].fields().len(), 2);
    }

    #[test]
    fn construct_agent_with_list_value() {
        let list = Value::List(ListValue {
            elements: vec![
                Value::String(Spanned::new("a".into(), sp(10, 13))),
                Value::String(Spanned::new("b".into(), sp(15, 18))),
            ],
            span: sp(9, 19),
        });

        let field = Field {
            name: Spanned::new("block".into(), sp(0, 5)),
            value: list,
            span: sp(0, 19),
        };

        assert_eq!(field.value.type_name(), "list");
        if let Value::List(lv) = &field.value {
            assert_eq!(lv.elements.len(), 2);
        } else {
            panic!("expected list value");
        }
    }

    #[test]
    fn construct_agent_with_all_block_types() {
        let named = |name: &str| NamedBlock {
            name: Spanned::new(name.into(), sp(0, name.len())),
            fields: vec![],
            span: sp(0, 10),
        };
        let unnamed = || UnnamedBlock {
            fields: vec![],
            span: sp(0, 10),
        };

        let program = Program {
            agent: AgentDef {
                name: Spanned::new("Full".into(), sp(0, 4)),
                blocks: vec![
                    AgentBlock::Model(named("primary")),
                    AgentBlock::Fallback(named("backup")),
                    AgentBlock::Tool(named("web_search")),
                    AgentBlock::Evidence(unnamed()),
                    AgentBlock::Verify(unnamed()),
                    AgentBlock::Safety(unnamed()),
                    AgentBlock::Memory(unnamed()),
                    AgentBlock::Output(unnamed()),
                    AgentBlock::Budget(unnamed()),
                ],
                span: sp(0, 100),
            },
        };

        assert_eq!(program.agent.blocks.len(), 9);

        let keywords: Vec<_> = program.agent.blocks.iter().map(|b| b.keyword()).collect();
        assert_eq!(
            keywords,
            vec![
                "model", "fallback", "tool", "evidence", "verify",
                "safety", "memory", "output", "budget"
            ]
        );
    }
}
