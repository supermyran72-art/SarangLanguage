pub mod common;
pub mod diagnostics;
pub mod emit;
pub mod ir;
pub mod lexer;
pub mod parser;
pub mod validator;

use std::fmt;

/// Error returned by [`compile`] when the source fails to parse, validate, or lower.
#[derive(Debug)]
pub enum CompileError {
    /// Source had parse errors.
    Parse(diagnostics::DiagnosticBag),
    /// Source had validation errors.
    Validation(diagnostics::DiagnosticBag),
    /// Internal lowering error (indicates a compiler bug).
    Lowering(ir::LoweringError),
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompileError::Parse(_) => write!(f, "parse errors"),
            CompileError::Validation(_) => write!(f, "validation errors"),
            CompileError::Lowering(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for CompileError {}

/// Compile Sarang source code to a [`ir::PolicyIr`] in one step.
///
/// Chains parse → validate → lower. Returns the policy IR on success,
/// or a [`CompileError`] with diagnostics on failure.
pub fn compile(source: &str) -> Result<ir::PolicyIr, CompileError> {
    let program = parser::parse(source).map_err(CompileError::Parse)?;

    let diags = validator::validate(&program);
    if diags.has_errors() {
        return Err(CompileError::Validation(diags));
    }

    ir::lower(&program).map_err(CompileError::Lowering)
}
