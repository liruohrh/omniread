//! HTML Parser module using scraper + boa_engine
//!
//! Provides a JavaScript API for parsing HTML documents.

mod context;
mod dom;
mod js_runtime;
mod rule;
mod rule_parser;

pub use context::*;
pub use dom::*;
pub use js_runtime::*;
pub use rule::*;
pub use rule_parser::*;
