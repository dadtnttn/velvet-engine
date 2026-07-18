//! # velvet-script-parser
//!
//! Recursive-descent parser for Velvet Script with basic error recovery.

#![deny(missing_docs)]

mod parser;

pub use parser::{parse, parse_file, ParseError, ParseResult};
