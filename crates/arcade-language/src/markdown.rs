//! Lightweight syntax scanner and highlighter for Markdown documents.

use crate::highlight::{HighlightKind, HighlightSpan};

/// Scans a markdown document text and returns a sequence of syntax highlight spans.
pub fn highlight_markdown(text: &str) -> Vec<HighlightSpan> {
    let mut spans = Vec::new();
    let mut in_code_block = false;
    let mut byte_offset = 0;

    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_start();
        let leading_spaces = line.len() - trimmed.len();
        let line_start = byte_offset;
        let line_len = line.trim_end_matches(['\r', '\n']).len();
        let content_start = line_start + leading_spaces;
        let content_end = line_start + line_len;

        // Fenced code block delimiters
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_code_block = !in_code_block;
            if content_end > content_start {
                spans.push(HighlightSpan::new(content_start, content_end, HighlightKind::Code));
            }
            byte_offset += line.len();
            continue;
        }

        if in_code_block {
            if content_end > content_start {
                spans.push(HighlightSpan::new(content_start, content_end, HighlightKind::Code));
            }
            byte_offset += line.len();
            continue;
        }

        // Headings (#, ##, ###, etc.)
        if trimmed.starts_with('#') {
            if content_end > content_start {
                spans.push(HighlightSpan::new(content_start, content_end, HighlightKind::Heading));
            }
            byte_offset += line.len();
            continue;
        }

        // Blockquotes (>)
        if trimmed.starts_with('>') {
            if content_end > content_start {
                spans.push(HighlightSpan::new(content_start, content_end, HighlightKind::Comment));
            }
            byte_offset += line.len();
            continue;
        }

        // Unordered or ordered list markers (- , * , + , 1. )
        if trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("+ ") {
            spans.push(HighlightSpan::new(content_start, content_start + 2, HighlightKind::Keyword));
        } else if let Some(dot_pos) = trimmed.find(". ") {
            if trimmed[..dot_pos].chars().all(|c| c.is_ascii_digit()) {
                spans.push(HighlightSpan::new(content_start, content_start + dot_pos + 2, HighlightKind::Keyword));
            }
        }

        // Inline code spans (`...`)
        let mut in_inline_code = false;
        let mut code_start = 0;
        for (idx, ch) in line.char_indices() {
            if ch == '`' {
                if in_inline_code {
                    spans.push(HighlightSpan::new(
                        line_start + code_start,
                        line_start + idx + 1,
                        HighlightKind::Code,
                    ));
                    in_inline_code = false;
                } else {
                    code_start = idx;
                    in_inline_code = true;
                }
            }
        }

        byte_offset += line.len();
    }

    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highlights_markdown_elements() {
        let md = "# Title\n\n- Item 1\n\n```rust\nfn main() {}\n```\n\n`code` here\n";
        let spans = highlight_markdown(md);
        assert!(!spans.is_empty());
        assert!(spans.iter().any(|s| s.kind == HighlightKind::Heading));
        assert!(spans.iter().any(|s| s.kind == HighlightKind::Keyword));
        assert!(spans.iter().any(|s| s.kind == HighlightKind::Code));
    }
}

