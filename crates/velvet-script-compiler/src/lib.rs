//! # velvet-script-compiler
//!
//! Compiles Velvet Script AST into bytecode modules.

#![deny(missing_docs)]

mod compiler;

pub use compiler::{compile, compile_source, CompileError, CompileResult};

/// VS2 HIR helpers.
pub mod vs2_lower;
pub mod vs2_codegen;
