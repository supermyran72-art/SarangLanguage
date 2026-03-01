use crate::common::span::Span;
use crate::lexer::token::{Token, TokenKind};

/// The Sarang lexer. Converts source text into a stream of tokens.
///
/// Operates on a byte slice of the source string. Whitespace and
/// comments are silently skipped. Every token carries a `Span`
/// recording its exact byte offsets.
pub struct Lexer<'src> {
    source: &'src str,
    bytes: &'src [u8],
    pos: usize,
}

impl<'src> Lexer<'src> {
    pub fn new(source: &'src str) -> Self {
        Self {
            source,
            bytes: source.as_bytes(),
            pos: 0,
        }
    }

    /// Tokenize the entire source, returning all tokens including a final `Eof`.
    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            let tok = self.next_token();
            let is_eof = tok.kind == TokenKind::Eof;
            tokens.push(tok);
            if is_eof {
                break;
            }
        }
        tokens
    }

    /// Produce the next token, advancing the cursor past it.
    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace_and_comments();

        if self.is_at_end() {
            return Token::new(TokenKind::Eof, Span::new(self.pos, self.pos));
        }

        let start = self.pos;
        let ch = self.advance();

        match ch {
            b'{' => Token::new(TokenKind::LBrace, Span::new(start, self.pos)),
            b'}' => Token::new(TokenKind::RBrace, Span::new(start, self.pos)),
            b'[' => Token::new(TokenKind::LBracket, Span::new(start, self.pos)),
            b']' => Token::new(TokenKind::RBracket, Span::new(start, self.pos)),
            b':' => Token::new(TokenKind::Colon, Span::new(start, self.pos)),
            b',' => Token::new(TokenKind::Comma, Span::new(start, self.pos)),
            b'"' => self.lex_string(start),
            c if is_ident_start(c) => self.lex_identifier_or_keyword(start),
            c if c.is_ascii_digit() => self.lex_number(start),
            _ => {
                let bad = &self.source[start..self.pos];
                Token::new(
                    TokenKind::Error(format!("unexpected character: {bad:?}")),
                    Span::new(start, self.pos),
                )
            }
        }
    }

    // ── Internal helpers ────────────────────────────────────────

    fn is_at_end(&self) -> bool {
        self.pos >= self.bytes.len()
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn advance(&mut self) -> u8 {
        let ch = self.bytes[self.pos];
        self.pos += 1;
        ch
    }

    /// Skip whitespace (spaces, tabs, newlines, carriage returns)
    /// and line comments (`// ...` until end of line).
    fn skip_whitespace_and_comments(&mut self) {
        loop {
            // Skip whitespace
            while let Some(ch) = self.peek() {
                if ch == b' ' || ch == b'\t' || ch == b'\n' || ch == b'\r' {
                    self.pos += 1;
                } else {
                    break;
                }
            }

            // Skip line comment
            if self.pos + 1 < self.bytes.len()
                && self.bytes[self.pos] == b'/'
                && self.bytes[self.pos + 1] == b'/'
            {
                // Advance past the entire line
                while let Some(ch) = self.peek() {
                    self.pos += 1;
                    if ch == b'\n' {
                        break;
                    }
                }
                continue; // There may be more whitespace/comments after
            }

            break;
        }
    }

    /// Lex a double-quoted string literal. The opening `"` has already been consumed.
    fn lex_string(&mut self, start: usize) -> Token {
        let mut value = String::new();

        loop {
            if self.is_at_end() {
                return Token::new(
                    TokenKind::Error("unterminated string literal".into()),
                    Span::new(start, self.pos),
                );
            }

            let ch = self.advance();
            match ch {
                b'"' => {
                    return Token::new(
                        TokenKind::StringLiteral(value),
                        Span::new(start, self.pos),
                    );
                }
                b'\\' => {
                    if self.is_at_end() {
                        return Token::new(
                            TokenKind::Error("unterminated string escape".into()),
                            Span::new(start, self.pos),
                        );
                    }
                    let escaped = self.advance();
                    match escaped {
                        b'"' => value.push('"'),
                        b'\\' => value.push('\\'),
                        b'n' => value.push('\n'),
                        b't' => value.push('\t'),
                        b'r' => value.push('\r'),
                        _ => {
                            return Token::new(
                                TokenKind::Error(format!(
                                    "invalid escape sequence: \\{}",
                                    escaped as char
                                )),
                                Span::new(start, self.pos),
                            );
                        }
                    }
                }
                b'\n' => {
                    return Token::new(
                        TokenKind::Error("unterminated string literal (newline in string)".into()),
                        Span::new(start, self.pos),
                    );
                }
                _ => {
                    // Safe: Sarang strings are ASCII-oriented for policy values.
                    // Non-ASCII bytes are passed through as-is.
                    value.push(ch as char);
                }
            }
        }
    }

    /// Lex an identifier or keyword. The first character has already been consumed.
    fn lex_identifier_or_keyword(&mut self, start: usize) -> Token {
        while let Some(ch) = self.peek() {
            if is_ident_continue(ch) {
                self.pos += 1;
            } else {
                break;
            }
        }

        let text = &self.source[start..self.pos];
        let kind = TokenKind::keyword_from_str(text)
            .unwrap_or_else(|| TokenKind::Ident(text.to_owned()));

        Token::new(kind, Span::new(start, self.pos))
    }

    /// Lex a number literal (integer or float). The first digit has already been consumed.
    fn lex_number(&mut self, start: usize) -> Token {
        // Consume all leading digits
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                self.pos += 1;
            } else {
                break;
            }
        }

        // Check for a decimal point followed by a digit
        let is_float = self.pos + 1 < self.bytes.len()
            && self.bytes[self.pos] == b'.'
            && self.bytes[self.pos + 1].is_ascii_digit();

        if is_float {
            self.pos += 1; // consume '.'
            while let Some(ch) = self.peek() {
                if ch.is_ascii_digit() {
                    self.pos += 1;
                } else {
                    break;
                }
            }
            let text = &self.source[start..self.pos];
            match text.parse::<f64>() {
                Ok(val) => Token::new(TokenKind::FloatLiteral(val), Span::new(start, self.pos)),
                Err(_) => Token::new(
                    TokenKind::Error(format!("invalid float literal: {text}")),
                    Span::new(start, self.pos),
                ),
            }
        } else {
            let text = &self.source[start..self.pos];
            match text.parse::<i64>() {
                Ok(val) => Token::new(TokenKind::IntLiteral(val), Span::new(start, self.pos)),
                Err(_) => Token::new(
                    TokenKind::Error(format!("invalid integer literal: {text}")),
                    Span::new(start, self.pos),
                ),
            }
        }
    }
}

/// Valid start of an identifier: `[a-zA-Z_]`
fn is_ident_start(ch: u8) -> bool {
    ch.is_ascii_alphabetic() || ch == b'_'
}

/// Valid continuation of an identifier: `[a-zA-Z0-9_]`
fn is_ident_continue(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || ch == b'_'
}

/// Convenience function: tokenize a source string in one call.
pub fn tokenize(source: &str) -> Vec<Token> {
    Lexer::new(source).tokenize()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: tokenize and return just the kinds (ignoring spans).
    fn kinds(source: &str) -> Vec<TokenKind> {
        tokenize(source).into_iter().map(|t| t.kind).collect()
    }

    /// Helper: tokenize and return (kind, source_text) pairs.
    fn kinds_and_text(source: &str) -> Vec<(TokenKind, String)> {
        tokenize(source)
            .into_iter()
            .map(|t| {
                let text = if t.kind == TokenKind::Eof {
                    String::new()
                } else {
                    source[t.span.start..t.span.end].to_owned()
                };
                (t.kind, text)
            })
            .collect()
    }

    // ── Empty and whitespace ────────────────────────────────────

    #[test]
    fn empty_source() {
        assert_eq!(kinds(""), vec![TokenKind::Eof]);
    }

    #[test]
    fn whitespace_only() {
        assert_eq!(kinds("   \t\n\r\n  "), vec![TokenKind::Eof]);
    }

    // ── Comments ────────────────────────────────────────────────

    #[test]
    fn line_comment_skipped() {
        assert_eq!(kinds("// this is a comment"), vec![TokenKind::Eof]);
    }

    #[test]
    fn comment_before_token() {
        assert_eq!(
            kinds("// comment\nagent"),
            vec![TokenKind::Agent, TokenKind::Eof]
        );
    }

    #[test]
    fn comment_after_token() {
        assert_eq!(
            kinds("agent // trailing comment"),
            vec![TokenKind::Agent, TokenKind::Eof]
        );
    }

    #[test]
    fn multiple_comments() {
        let src = "// first\n// second\nagent";
        assert_eq!(kinds(src), vec![TokenKind::Agent, TokenKind::Eof]);
    }

    // ── Punctuation ─────────────────────────────────────────────

    #[test]
    fn all_punctuation() {
        assert_eq!(
            kinds("{ } [ ] : ,"),
            vec![
                TokenKind::LBrace,
                TokenKind::RBrace,
                TokenKind::LBracket,
                TokenKind::RBracket,
                TokenKind::Colon,
                TokenKind::Comma,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn punctuation_no_spaces() {
        assert_eq!(
            kinds("{}[]:,"),
            vec![
                TokenKind::LBrace,
                TokenKind::RBrace,
                TokenKind::LBracket,
                TokenKind::RBracket,
                TokenKind::Colon,
                TokenKind::Comma,
                TokenKind::Eof,
            ]
        );
    }

    // ── Keywords ────────────────────────────────────────────────

    #[test]
    fn all_keywords() {
        let src = "agent model fallback tool evidence verify safety memory output budget";
        assert_eq!(
            kinds(src),
            vec![
                TokenKind::Agent,
                TokenKind::Model,
                TokenKind::Fallback,
                TokenKind::Tool,
                TokenKind::Evidence,
                TokenKind::Verify,
                TokenKind::Safety,
                TokenKind::Memory,
                TokenKind::Output,
                TokenKind::Budget,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn true_and_false() {
        assert_eq!(
            kinds("true false"),
            vec![TokenKind::True, TokenKind::False, TokenKind::Eof]
        );
    }

    // ── Identifiers ─────────────────────────────────────────────

    #[test]
    fn simple_identifiers() {
        assert_eq!(
            kinds("foo bar_baz _priv"),
            vec![
                TokenKind::Ident("foo".into()),
                TokenKind::Ident("bar_baz".into()),
                TokenKind::Ident("_priv".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn identifier_with_digits() {
        assert_eq!(
            kinds("model2 v0_1"),
            vec![
                TokenKind::Ident("model2".into()),
                TokenKind::Ident("v0_1".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn keyword_prefix_is_identifier() {
        // "agents" is not the keyword "agent"
        assert_eq!(
            kinds("agents modeler"),
            vec![
                TokenKind::Ident("agents".into()),
                TokenKind::Ident("modeler".into()),
                TokenKind::Eof,
            ]
        );
    }

    // ── String literals ─────────────────────────────────────────

    #[test]
    fn simple_string() {
        assert_eq!(
            kinds(r#""hello""#),
            vec![TokenKind::StringLiteral("hello".into()), TokenKind::Eof]
        );
    }

    #[test]
    fn empty_string() {
        assert_eq!(
            kinds(r#""""#),
            vec![TokenKind::StringLiteral(String::new()), TokenKind::Eof]
        );
    }

    #[test]
    fn string_with_escapes() {
        assert_eq!(
            kinds(r#""line\none\ttwo\\end\"quote""#),
            vec![
                TokenKind::StringLiteral("line\none\ttwo\\end\"quote".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn string_with_special_chars() {
        assert_eq!(
            kinds(r#""gpt-4.0""#),
            vec![TokenKind::StringLiteral("gpt-4.0".into()), TokenKind::Eof]
        );
    }

    #[test]
    fn unterminated_string() {
        let tokens = tokenize(r#""hello"#);
        assert!(matches!(tokens[0].kind, TokenKind::Error(_)));
    }

    #[test]
    fn string_with_newline() {
        let tokens = tokenize("\"hello\nworld\"");
        assert!(matches!(tokens[0].kind, TokenKind::Error(_)));
    }

    #[test]
    fn string_bad_escape() {
        let tokens = tokenize(r#""bad\xescape""#);
        assert!(matches!(tokens[0].kind, TokenKind::Error(_)));
    }

    // ── Integer literals ────────────────────────────────────────

    #[test]
    fn simple_integers() {
        assert_eq!(
            kinds("0 42 100000"),
            vec![
                TokenKind::IntLiteral(0),
                TokenKind::IntLiteral(42),
                TokenKind::IntLiteral(100000),
                TokenKind::Eof,
            ]
        );
    }

    // ── Float literals ──────────────────────────────────────────

    #[test]
    fn simple_floats() {
        assert_eq!(
            kinds("0.7 3.14 1.00"),
            vec![
                TokenKind::FloatLiteral(0.7),
                TokenKind::FloatLiteral(3.14),
                TokenKind::FloatLiteral(1.0),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn integer_followed_by_dot_no_digit_is_not_float() {
        // "42." followed by non-digit should lex as int 42 then error on '.'
        let tokens = tokenize("42.x");
        assert_eq!(tokens[0].kind, TokenKind::IntLiteral(42));
        // '.' is not a valid start, so it becomes an error
        assert!(matches!(tokens[1].kind, TokenKind::Error(_)));
    }

    // ── Error tokens ────────────────────────────────────────────

    #[test]
    fn unknown_character() {
        let tokens = tokenize("@");
        assert!(matches!(tokens[0].kind, TokenKind::Error(_)));
        assert_eq!(tokens[1].kind, TokenKind::Eof);
    }

    #[test]
    fn error_does_not_stop_lexing() {
        let tokens = tokenize("@ agent");
        assert!(matches!(tokens[0].kind, TokenKind::Error(_)));
        assert_eq!(tokens[1].kind, TokenKind::Agent);
        assert_eq!(tokens[2].kind, TokenKind::Eof);
    }

    // ── Span accuracy ───────────────────────────────────────────

    #[test]
    fn spans_match_source_text() {
        let src = r#"agent Foo { name: "bar" }"#;
        let pairs = kinds_and_text(src);
        assert_eq!(pairs[0], (TokenKind::Agent, "agent".into()));
        assert_eq!(pairs[1], (TokenKind::Ident("Foo".into()), "Foo".into()));
        assert_eq!(pairs[2], (TokenKind::LBrace, "{".into()));
        assert_eq!(pairs[3], (TokenKind::Ident("name".into()), "name".into()));
        assert_eq!(pairs[4], (TokenKind::Colon, ":".into()));
        assert_eq!(
            pairs[5],
            (TokenKind::StringLiteral("bar".into()), "\"bar\"".into())
        );
        assert_eq!(pairs[6], (TokenKind::RBrace, "}".into()));
    }

    #[test]
    fn span_offsets_are_correct() {
        let src = "agent Foo";
        let tokens = tokenize(src);
        // "agent" is bytes 0..5
        assert_eq!(tokens[0].span, Span::new(0, 5));
        // "Foo" is bytes 6..9
        assert_eq!(tokens[1].span, Span::new(6, 9));
    }

    // ── Full program ────────────────────────────────────────────

    #[test]
    fn lex_minimal_agent() {
        let src = r#"agent Minimal {
    model default {
        provider: "openai"
        name: "gpt-4"
    }
}"#;
        let k = kinds(src);
        assert_eq!(
            k,
            vec![
                TokenKind::Agent,
                TokenKind::Ident("Minimal".into()),
                TokenKind::LBrace,
                TokenKind::Model,
                TokenKind::Ident("default".into()),
                TokenKind::LBrace,
                TokenKind::Ident("provider".into()),
                TokenKind::Colon,
                TokenKind::StringLiteral("openai".into()),
                TokenKind::Ident("name".into()),
                TokenKind::Colon,
                TokenKind::StringLiteral("gpt-4".into()),
                TokenKind::RBrace,
                TokenKind::RBrace,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lex_list_syntax() {
        let src = r#"block: ["a", "b", "c"]"#;
        assert_eq!(
            kinds(src),
            vec![
                TokenKind::Ident("block".into()),
                TokenKind::Colon,
                TokenKind::LBracket,
                TokenKind::StringLiteral("a".into()),
                TokenKind::Comma,
                TokenKind::StringLiteral("b".into()),
                TokenKind::Comma,
                TokenKind::StringLiteral("c".into()),
                TokenKind::RBracket,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lex_mixed_values() {
        let src = r#"temperature: 0.3 max_tokens: 8192 require_evidence: true"#;
        assert_eq!(
            kinds(src),
            vec![
                TokenKind::Ident("temperature".into()),
                TokenKind::Colon,
                TokenKind::FloatLiteral(0.3),
                TokenKind::Ident("max_tokens".into()),
                TokenKind::Colon,
                TokenKind::IntLiteral(8192),
                TokenKind::Ident("require_evidence".into()),
                TokenKind::Colon,
                TokenKind::True,
                TokenKind::Eof,
            ]
        );
    }
}
