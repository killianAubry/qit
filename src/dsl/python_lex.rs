// Python (Qiskit) line lexer for syntax highlighting.
//
// Line-oriented to match the existing editor pipeline. Strings detected
// inside a single line only — multi-line `"""…"""` blocks are colored only
// on the lines they fully open and close. Good enough for the expected
// short Qiskit programs; can be promoted to a stateful lexer later.

use super::lex::{Token, TokenKind};

pub fn python_tokens_as_editor_tokens(line: &str) -> Vec<Token<'_>> {
    let bytes = line.as_bytes();
    let mut out: Vec<Token<'_>> = Vec::with_capacity(12);
    let mut i = 0;

    while i < bytes.len() {
        let c = bytes[i] as char;

        // line comment to EOL
        if c == '#' {
            out.push(Token { kind: TokenKind::Comment, text: &line[i..] });
            return out;
        }

        // whitespace run
        if c.is_ascii_whitespace() {
            let start = i;
            while i < bytes.len() && (bytes[i] as char).is_ascii_whitespace() {
                i += 1;
            }
            out.push(Token { kind: TokenKind::Whitespace, text: &line[start..i] });
            continue;
        }

        // strings — handle ' " and triple variants on one line
        if c == '"' || c == '\'' {
            let triple = i + 2 < bytes.len() && bytes[i + 1] == c as u8 && bytes[i + 2] == c as u8;
            let start = i;
            if triple {
                i += 3;
                let mut closed = false;
                while i + 2 < bytes.len() {
                    if bytes[i] == c as u8 && bytes[i + 1] == c as u8 && bytes[i + 2] == c as u8 {
                        i += 3;
                        closed = true;
                        break;
                    }
                    i += 1;
                }
                if !closed {
                    i = bytes.len();
                }
            } else {
                i += 1;
                while i < bytes.len() {
                    let ch = bytes[i];
                    if ch == b'\\' && i + 1 < bytes.len() {
                        i += 2;
                        continue;
                    }
                    if ch == c as u8 {
                        i += 1;
                        break;
                    }
                    i += 1;
                }
            }
            out.push(Token { kind: TokenKind::StringLit, text: &line[start..i] });
            continue;
        }

        // punctuation
        if matches!(c, ',' | ';' | '(' | ')' | '[' | ']' | '{' | '}' | ':' | '=' | '+' | '-' | '*' | '/' | '%' | '<' | '>' | '|' | '&' | '@' | '.') {
            // collapse runs of operator chars except `.` (kept as separate punctuation
            // so attribute access like `qc.h` stays neat).
            let start = i;
            if c == '.' {
                i += 1;
            } else {
                while i < bytes.len()
                    && matches!(
                        bytes[i] as char,
                        ',' | ';' | '(' | ')' | '[' | ']' | '{' | '}' | ':' | '=' | '+' | '-' | '*' | '/' | '%' | '<' | '>' | '|' | '&' | '@'
                    )
                {
                    i += 1;
                }
            }
            out.push(Token { kind: TokenKind::Punctuation, text: &line[start..i] });
            continue;
        }

        // word / number
        let start = i;
        while i < bytes.len() {
            let ch = bytes[i] as char;
            if ch.is_ascii_alphanumeric() || ch == '_' {
                i += 1;
            } else {
                break;
            }
        }
        if start == i {
            let Some(ch) = line[i..].chars().next() else {
                break;
            };
            let w = ch.len_utf8();
            i += w;
            out.push(Token {
                kind: TokenKind::Identifier,
                text: &line[start..i],
            });
            continue;
        }
        let text = &line[start..i];
        out.push(Token { kind: classify(text), text });
    }

    out
}

fn classify(s: &str) -> TokenKind {
    if is_keyword(s) {
        return TokenKind::Keyword;
    }
    if is_qiskit_method_or_class(s) {
        // Color Qiskit-recognised symbols in the same slot we already use
        // for "important" identifiers (purple).
        return TokenKind::Qubit;
    }
    if looks_number(s) {
        return TokenKind::Number;
    }
    TokenKind::Identifier
}

fn is_keyword(s: &str) -> bool {
    matches!(
        s,
        "False"
            | "None"
            | "True"
            | "and"
            | "as"
            | "assert"
            | "async"
            | "await"
            | "break"
            | "class"
            | "continue"
            | "def"
            | "del"
            | "elif"
            | "else"
            | "except"
            | "finally"
            | "for"
            | "from"
            | "global"
            | "if"
            | "import"
            | "in"
            | "is"
            | "lambda"
            | "nonlocal"
            | "not"
            | "or"
            | "pass"
            | "raise"
            | "return"
            | "try"
            | "while"
            | "with"
            | "yield"
    )
}

fn is_qiskit_method_or_class(s: &str) -> bool {
    matches!(
        s,
        "QuantumCircuit"
            | "QuantumRegister"
            | "ClassicalRegister"
            | "Statevector"
            | "DensityMatrix"
            | "Operator"
            | "transpile"
            | "execute"
            | "Aer"
            | "BasicAer"
            | "qiskit"
            | "numpy"
            | "np"
            | "pi"
            // common gate methods
            | "h" | "x" | "y" | "z" | "s" | "sdg" | "t" | "tdg"
            | "rx" | "ry" | "rz" | "cx" | "cy" | "cz" | "ch" | "ccx"
            | "swap" | "cswap" | "u" | "u1" | "u2" | "u3"
            | "measure" | "reset" | "barrier"
    )
}

fn looks_number(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    s.parse::<f64>().is_ok() || s.parse::<i64>().is_ok()
}
