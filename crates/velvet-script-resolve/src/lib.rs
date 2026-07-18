//! Name resolution for Velvet Script 2 — rust-like paths, no Python globals.
//!
//! Builds a symbol table from HIR modules, resolves `use`/`mod` paths,
//! and reports unbound names with spans.

#![deny(missing_docs)]
#![allow(dead_code)]

mod scope;
mod imports;
mod symbols;
mod resolve;
mod diagnostics;
mod prelude_names;

pub use diagnostics::{ResolveDiag, ResolveSeverity};
pub use imports::{ImportEdge, ImportGraph};
pub use resolve::{check_name, resolve_module, resolve_workspace, ResolveResult};
pub use scope::{Scope, ScopeId, ScopeKind, ScopeTree};
pub use symbols::{Symbol, SymbolId, SymbolKind, SymbolTable};
pub use prelude_names::{is_prelude, prelude_ty, PRELUDE};
