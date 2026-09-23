//! Language services built on top of, but separate from, the text core.
//!
//! Provides AST-based syntax parsing via Tree-Sitter, semantic highlight queries,
//! markdown document tokenization, and a non-blocking background highlight worker.

pub mod highlight;
pub mod markdown;
pub mod scanners;
pub mod service;

pub use highlight::{
    map_capture_name, resolve_line_tokens, HighlightKind, HighlightSpan, LineToken,
};
pub use markdown::highlight_markdown;
pub use scanners::{highlight_json, highlight_toml, highlight_yaml};
pub use service::{HighlightRequest, HighlightResponse, HighlightWorker, LanguageId, LanguageService};

/// Reports whether syntax services should be requested for a document.
pub fn syntax_is_enabled_for(byte_length: usize, threshold: usize) -> bool {
    byte_length <= threshold
}
