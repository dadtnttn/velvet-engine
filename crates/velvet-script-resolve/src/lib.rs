//! Name resolution for Velvet Script 2 — rust-like paths, no Python globals.
//!
//! Builds a symbol table from HIR modules, resolves `use`/`mod` paths,
//! and reports unbound names with spans.

#![deny(missing_docs)]
#![allow(dead_code)]

mod diagnostics;
mod imports;
mod prelude_names;
mod resolve;
mod scope;
mod symbols;

pub use diagnostics::{ResolveDiag, ResolveSeverity};
pub use imports::{ImportEdge, ImportGraph};
pub use prelude_names::{is_prelude, prelude_ty, PRELUDE};
pub use resolve::{check_name, resolve_module, resolve_workspace, ResolveResult};
pub use scope::{Scope, ScopeId, ScopeKind, ScopeTree};
pub use symbols::{Symbol, SymbolId, SymbolKind, SymbolTable};
