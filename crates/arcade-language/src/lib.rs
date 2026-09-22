//! Language services built on top of, but separate from, the text core.
//!
//! Tree-sitter parser workers and highlight queries will live here. Keeping
//! them out of `arcade-core` lets plain-text and huge-file modes stay lean.

/// Reports whether syntax services should be requested for a document.
pub fn syntax_is_enabled_for(byte_length: usize, threshold: usize) -> bool {
    byte_length <= threshold
}
