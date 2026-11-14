//! A token that never prints itself.

use std::fmt;

/// A bearer token for the engine.
///
/// `Debug` and `Display` both print `<redacted>`, so a token cannot reach a log line,
/// a panic message or a UI error through formatting. The value is available only
/// through [`Token::expose`], which is called in exactly one place: the request
/// builder.
#[derive(Clone, PartialEq, Eq)]
pub struct Token(String);

impl Token {
    /// Wrap a token value.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// The token value, for the request header.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }

    /// Whether the token is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<redacted>")
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<redacted>")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_never_reveals_the_value() {
        let token = Token::new("super-secret");

        assert_eq!(format!("{token:?}"), "<redacted>");
    }

    #[test]
    fn display_never_reveals_the_value() {
        let token = Token::new("super-secret");

        assert_eq!(format!("{token}"), "<redacted>");
    }

    #[test]
    fn expose_returns_the_value() {
        assert_eq!(Token::new("abc").expose(), "abc");
    }

    #[test]
    fn empty_tokens_are_detected() {
        assert!(Token::new("").is_empty());
        assert!(!Token::new("x").is_empty());
    }

    #[test]
    fn equality_compares_values() {
        assert_eq!(Token::new("a"), Token::new("a"));
        assert_ne!(Token::new("a"), Token::new("b"));
    }
}
