// Shared editor token types used by every line lexer in this module.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenKind {
    Comment,
    Keyword,
    /// "Highlighted identifier" slot — used for QASM gate names and Qiskit
    /// methods/classes. Named historically after qubit refs (`qN`).
    Qubit,
    Number,
    Identifier,
    Whitespace,
    /// Delimiters: , ; ( ) [ ] { } and `->` (OpenQASM).
    Punctuation,
    /// Quoted string literal.
    StringLit,
}

#[derive(Clone, Copy, Debug)]
pub struct Token<'a> {
    pub kind: TokenKind,
    pub text: &'a str,
}
