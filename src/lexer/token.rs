use crate::common::span::Span;

/// Every kind of token the Sarang lexer can produce.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // ── Keywords (block-level) ──────────────────────────────────
    Agent,
    Model,
    Fallback,
    Tool,
    Evidence,
    Verify,
    Safety,
    Memory,
    Output,
    Budget,

    // ── Boolean literals ────────────────────────────────────────
    True,
    False,

    // ── Literals ────────────────────────────────────────────────
    /// A double-quoted string literal (value stored without quotes).
    StringLiteral(String),
    /// An integer literal.
    IntLiteral(i64),
    /// A floating-point literal.
    FloatLiteral(f64),

    // ── Identifiers ─────────────────────────────────────────────
    /// Any identifier that is not a keyword: agent names, block names, field names.
    Ident(String),

    // ── Punctuation ─────────────────────────────────────────────
    LBrace,   // {
    RBrace,   // }
    LBracket, // [
    RBracket, // ]
    Colon,    // :
    Comma,    // ,

    // ── Special ─────────────────────────────────────────────────
    /// End of file.
    Eof,
    /// An unrecognized character or malformed token.
    Error(String),
}

impl TokenKind {
    /// Returns `true` if this token is a block-level keyword.
    pub fn is_block_keyword(&self) -> bool {
        matches!(
            self,
            TokenKind::Agent
                | TokenKind::Model
                | TokenKind::Fallback
                | TokenKind::Tool
                | TokenKind::Evidence
                | TokenKind::Verify
                | TokenKind::Safety
                | TokenKind::Memory
                | TokenKind::Output
                | TokenKind::Budget
        )
    }

    /// Returns `true` if this is a literal value (string, int, float, bool).
    pub fn is_literal(&self) -> bool {
        matches!(
            self,
            TokenKind::StringLiteral(_)
                | TokenKind::IntLiteral(_)
                | TokenKind::FloatLiteral(_)
                | TokenKind::True
                | TokenKind::False
        )
    }

    /// Human-readable description for diagnostics.
    pub fn describe(&self) -> &'static str {
        match self {
            TokenKind::Agent => "keyword `agent`",
            TokenKind::Model => "keyword `model`",
            TokenKind::Fallback => "keyword `fallback`",
            TokenKind::Tool => "keyword `tool`",
            TokenKind::Evidence => "keyword `evidence`",
            TokenKind::Verify => "keyword `verify`",
            TokenKind::Safety => "keyword `safety`",
            TokenKind::Memory => "keyword `memory`",
            TokenKind::Output => "keyword `output`",
            TokenKind::Budget => "keyword `budget`",
            TokenKind::True => "`true`",
            TokenKind::False => "`false`",
            TokenKind::StringLiteral(_) => "string literal",
            TokenKind::IntLiteral(_) => "integer literal",
            TokenKind::FloatLiteral(_) => "float literal",
            TokenKind::Ident(_) => "identifier",
            TokenKind::LBrace => "`{`",
            TokenKind::RBrace => "`}`",
            TokenKind::LBracket => "`[`",
            TokenKind::RBracket => "`]`",
            TokenKind::Colon => "`:`",
            TokenKind::Comma => "`,`",
            TokenKind::Eof => "end of file",
            TokenKind::Error(_) => "error",
        }
    }

    /// Try to match an identifier string to a keyword.
    pub fn keyword_from_str(s: &str) -> Option<TokenKind> {
        match s {
            "agent" => Some(TokenKind::Agent),
            "model" => Some(TokenKind::Model),
            "fallback" => Some(TokenKind::Fallback),
            "tool" => Some(TokenKind::Tool),
            "evidence" => Some(TokenKind::Evidence),
            "verify" => Some(TokenKind::Verify),
            "safety" => Some(TokenKind::Safety),
            "memory" => Some(TokenKind::Memory),
            "output" => Some(TokenKind::Output),
            "budget" => Some(TokenKind::Budget),
            "true" => Some(TokenKind::True),
            "false" => Some(TokenKind::False),
            _ => None,
        }
    }
}

/// A token with its kind and source span.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }

    pub fn is_eof(&self) -> bool {
        self.kind == TokenKind::Eof
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyword_lookup_matches_all_keywords() {
        let keywords = [
            ("agent", TokenKind::Agent),
            ("model", TokenKind::Model),
            ("fallback", TokenKind::Fallback),
            ("tool", TokenKind::Tool),
            ("evidence", TokenKind::Evidence),
            ("verify", TokenKind::Verify),
            ("safety", TokenKind::Safety),
            ("memory", TokenKind::Memory),
            ("output", TokenKind::Output),
            ("budget", TokenKind::Budget),
            ("true", TokenKind::True),
            ("false", TokenKind::False),
        ];
        for (text, expected) in &keywords {
            assert_eq!(
                TokenKind::keyword_from_str(text).as_ref(),
                Some(expected),
                "keyword_from_str({text:?}) should match"
            );
        }
    }

    #[test]
    fn keyword_lookup_returns_none_for_non_keywords() {
        assert_eq!(TokenKind::keyword_from_str("foo"), None);
        assert_eq!(TokenKind::keyword_from_str("Agent"), None); // case-sensitive
        assert_eq!(TokenKind::keyword_from_str(""), None);
        assert_eq!(TokenKind::keyword_from_str("model_name"), None);
    }

    #[test]
    fn is_block_keyword() {
        assert!(TokenKind::Agent.is_block_keyword());
        assert!(TokenKind::Budget.is_block_keyword());
        assert!(!TokenKind::True.is_block_keyword());
        assert!(!TokenKind::Ident("foo".into()).is_block_keyword());
        assert!(!TokenKind::Colon.is_block_keyword());
    }

    #[test]
    fn is_literal() {
        assert!(TokenKind::True.is_literal());
        assert!(TokenKind::False.is_literal());
        assert!(TokenKind::StringLiteral("hi".into()).is_literal());
        assert!(TokenKind::IntLiteral(42).is_literal());
        assert!(TokenKind::FloatLiteral(3.125).is_literal());
        assert!(!TokenKind::Agent.is_literal());
        assert!(!TokenKind::Ident("x".into()).is_literal());
    }

    #[test]
    fn describe_gives_human_readable_strings() {
        assert_eq!(TokenKind::Agent.describe(), "keyword `agent`");
        assert_eq!(TokenKind::LBrace.describe(), "`{`");
        assert_eq!(TokenKind::StringLiteral("x".into()).describe(), "string literal");
        assert_eq!(TokenKind::Eof.describe(), "end of file");
    }

    #[test]
    fn token_new_and_eof_check() {
        let tok = Token::new(TokenKind::Eof, Span::new(10, 10));
        assert!(tok.is_eof());

        let tok2 = Token::new(TokenKind::Agent, Span::new(0, 5));
        assert!(!tok2.is_eof());
    }

    #[test]
    fn token_equality() {
        let a = Token::new(TokenKind::Colon, Span::new(5, 6));
        let b = Token::new(TokenKind::Colon, Span::new(5, 6));
        assert_eq!(a, b);

        let c = Token::new(TokenKind::Colon, Span::new(5, 7));
        assert_ne!(a, c);
    }
}
