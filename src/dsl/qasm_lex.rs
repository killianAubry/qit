// OpenQASM 2–oriented syntax highlighting (line-oriented).
//
// `//` starts a line comment. Each line is lexed independently.

use super::lex::{Token, TokenKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum QasmTok {
    Comment,
    Keyword,
    Gate,
    Register,
    Pi,
    Number,
    Punct,
    Ident,
    Ws,
    Str,
}

#[derive(Clone, Copy, Debug)]
struct Qt<'a> {
    kind: QasmTok,
    text: &'a str,
}

pub fn qasm_tokens_as_editor_tokens(line: &str) -> Vec<Token<'_>> {
    tokenize(line)
        .into_iter()
        .map(|t| Token {
            kind: match t.kind {
                QasmTok::Comment => TokenKind::Comment,
                QasmTok::Keyword => TokenKind::Keyword,
                QasmTok::Gate | QasmTok::Register => TokenKind::Qubit,
                QasmTok::Pi => TokenKind::Number,
                QasmTok::Number => TokenKind::Number,
                QasmTok::Punct => TokenKind::Punctuation,
                QasmTok::Ident => TokenKind::Identifier,
                QasmTok::Ws => TokenKind::Whitespace,
                QasmTok::Str => TokenKind::StringLit,
            },
            text: t.text,
        })
        .collect()
}

fn tokenize(line: &str) -> Vec<Qt<'_>> {
    let bytes = line.as_bytes();
    let mut out = Vec::with_capacity(12);
    let mut i = 0;

    while i < bytes.len() {
        // OpenQASM 2.0 officially uses `//`; many editor buffers also mix in
        // `#` shell-style comments — treat both like micro / Python.
        if bytes[i] == b'#'
            || (i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'/')
        {
            out.push(Qt {
                kind: QasmTok::Comment,
                text: &line[i..],
            });
            return out;
        }

        let c = bytes[i] as char;
        if c.is_ascii_whitespace() {
            let start = i;
            while i < bytes.len() && (bytes[i] as char).is_ascii_whitespace() {
                i += 1;
            }
            out.push(Qt {
                kind: QasmTok::Ws,
                text: &line[start..i],
            });
            continue;
        }

        if c == '-' && i + 1 < bytes.len() && bytes[i + 1] == b'>' {
            out.push(Qt {
                kind: QasmTok::Punct,
                text: &line[i..i + 2],
            });
            i += 2;
            continue;
        }

        if matches!(c, ',' | ';' | '(' | ')' | '[' | ']' | '{' | '}' | '+' | '-' | '*' | '/' | '^' | '=') {
            out.push(Qt {
                kind: QasmTok::Punct,
                text: &line[i..=i],
            });
            i += 1;
            continue;
        }

        if c == '"' {
            let start = i;
            i += 1;
            while i < bytes.len() && bytes[i] != b'"' {
                i += 1;
            }
            if i < bytes.len() {
                i += 1; // closing "
            }
            out.push(Qt {
                kind: QasmTok::Str,
                text: &line[start..i],
            });
            continue;
        }

        let start = i;
        while i < bytes.len() {
            let ch = bytes[i] as char;
            if ch.is_ascii_whitespace()
                || is_punctuation(ch)
                || ch == '"'
                || bytes[i] == b'#'
                || (ch == '-' && i + 1 < bytes.len() && bytes[i + 1] == b'>')
                || (ch == '/' && i + 1 < bytes.len() && bytes[i + 1] == b'/')
            {
                break;
            }
            i += 1;
        }
        let text = &line[start..i];
        let mut kind = classify_word(text);
        if matches!(kind, QasmTok::Ident) && i < bytes.len() && bytes[i] == b'[' {
            kind = QasmTok::Register;
        }
        out.push(Qt {
            kind,
            text,
        });
    }

    out
}

fn classify_word(s: &str) -> QasmTok {
    let u = s.to_ascii_uppercase();
    if u == "PI" {
        return QasmTok::Pi;
    }
    if matches!(
        u.as_str(),
        "OPENQASM" | "INCLUDE" | "QREG" | "CREG" | "GATE" | "OPAQUE" | "MEASURE" | "BARRIER" | "IF"
    ) {
        return QasmTok::Keyword;
    }
    if is_gate(&u) {
        return QasmTok::Gate;
    }
    if looks_number(s) {
        return QasmTok::Number;
    }
    QasmTok::Ident
}

fn is_gate(u: &str) -> bool {
    matches!(
        u,
        "U" | "CX" | "U0" | "U1" | "U2" | "U3" | "H" | "X" | "Y" | "Z" | "S" | "SDG" | "T" | "TDG"
            | "RX" | "RY" | "RZ" | "CZ" | "CY" | "CH" | "CCX" | "CRZ" | "CU1" | "CU3" | "SWAP" | "ID"
            | "RESET"
    )
}

fn looks_number(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    s.parse::<f64>().is_ok()
}

fn is_punctuation(c: char) -> bool {
    matches!(
        c,
        ',' | ';' | '(' | ')' | '[' | ']' | '{' | '}' | '+' | '-' | '*' | '/' | '^' | '='
    )
}

#[cfg(test)]
mod tests {
    use super::qasm_tokens_as_editor_tokens;
    use crate::dsl::TokenKind;

    #[test]
    fn highlights_qasm_keywords_gates_and_registers() {
        let toks = qasm_tokens_as_editor_tokens("OPENQASM 2.0; include \"qelib1.inc\"; h q[0];");
        assert!(toks.iter().any(|t| t.text == "OPENQASM" && t.kind == TokenKind::Keyword));
        assert!(toks.iter().any(|t| t.text == "include" && t.kind == TokenKind::Keyword));
        assert!(toks.iter().any(|t| t.text == "\"qelib1.inc\"" && t.kind == TokenKind::StringLit));
        assert!(toks.iter().any(|t| t.text == "h" && t.kind == TokenKind::Qubit));
        assert!(toks.iter().any(|t| t.text == "q" && t.kind == TokenKind::Qubit));
    }

    #[test]
    fn splits_parameter_expressions_into_highlightable_tokens() {
        let toks = qasm_tokens_as_editor_tokens("ry(-pi/2) q[0];");
        assert!(toks.iter().any(|t| t.text == "ry" && t.kind == TokenKind::Qubit));
        assert!(toks.iter().any(|t| t.text == "-" && t.kind == TokenKind::Punctuation));
        assert!(toks.iter().any(|t| t.text.eq_ignore_ascii_case("pi") && t.kind == TokenKind::Number));
        assert!(toks.iter().any(|t| t.text == "/" && t.kind == TokenKind::Punctuation));
        assert!(toks.iter().any(|t| t.text == "2" && t.kind == TokenKind::Number));
    }
}
