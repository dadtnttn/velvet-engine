//! # velvet-script-compiler
//!
//! Compiles Velvet Script AST into bytecode modules.

#![deny(missing_docs)]

mod compiler;

pub use compiler::{compile, compile_source, CompileError, CompileResult};

pub mod vs2_codegen;
/// VS2 HIR helpers.
pub mod vs2_lower;
