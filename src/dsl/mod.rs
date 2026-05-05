// Source-code highlighting.
//
// Both editor modes are driven by a per-line tokenizer:
//   * `qasm_lex`   — OpenQASM 2
//   * `python_lex` — Python (Qiskit)
//
// The `Token` / `TokenKind` types in `lex` are the small surface area shared
// with the editor's layouter (`components::editor`).
//
// `Diagnostic` lives here so the rest of the app can render runner errors
// without having to know which simulator produced them.

pub mod lex;
pub mod python_lex;
pub mod python_parse;
pub mod qasm_lex;
pub mod qasm_parse;

pub use lex::TokenKind;
pub use python_lex::python_tokens_as_editor_tokens;
pub use python_parse::parse_python;
pub use qasm_lex::qasm_tokens_as_editor_tokens;
pub use qasm_parse::parse_qasm;

#[derive(Clone, Debug)]
pub struct Diagnostic {
    pub line: usize,
    pub message: String,
}
