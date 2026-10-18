//! # velvet-wasm
//!
//! Velvet's Rust crates compiled to WebAssembly for Velvet JS. The `js/packages/wasm` package
//! builds this crate (`pnpm build:wasm`) and loads it; the packages that wrap it, such as
//! `@velvet/guion`, give games typed functions.
//!
//! ## The boundary
//!
//! - Values cross as **JSON text**: arguments and results are strings that hold JSON. The only
//!   objects that cross are the handles of [`guion::GuionLibrary`] and
//!   [`guion::GuionConversation`].
//! - JSON field names are camelCase. Words that are content keep the names the guion format uses
//!   (scene attributes such as `quien`, condition operators such as `gte`).
//! - A failure is `{"error": "<message>", "code": "<code>"}`. A function that returns JSON returns
//!   that object in place of its result. A function that returns a handle (a constructor, or
//!   `start`) fails with the same JSON text as its error, which the JavaScript side turns into a
//!   `GuionError`. A panic is a bug, not a failure.
//!
//! Every exported function is an ordinary Rust function, so `cargo test -p velvet-wasm` runs the
//! code that JavaScript calls.

#![deny(missing_docs)]

use serde::Serialize;

pub mod guion;

/// A failure at the boundary: a stable `code` that callers branch on, and a message for people.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Failure {
    code: &'static str,
    message: String,
}

impl Failure {
    /// A failure with `code` and `message`.
    pub(crate) fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    /// The failure as the JSON text the boundary returns: `{"error": ..., "code": ...}`.
    pub(crate) fn to_json(&self) -> String {
        serde_json::json!({ "error": self.message, "code": self.code }).to_string()
    }
}

/// `value` as JSON text. A value that cannot be written becomes an `internal` failure.
pub(crate) fn json<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value)
        .unwrap_or_else(|e| Failure::new("internal", e.to_string()).to_json())
}

/// A result as the JSON text the boundary returns: the value, or the failure's object.
pub(crate) fn respond<T: Serialize>(result: Result<T, Failure>) -> String {
    match result {
        Ok(value) => json(&value),
        Err(failure) => failure.to_json(),
    }
}
