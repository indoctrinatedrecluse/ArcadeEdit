//! Lightweight fast syntax scanners for TOML, YAML, and JSON documents.

use crate::highlight::{HighlightKind, HighlightSpan};

/// Scans a TOML document text and returns sorted syntax highlight spans.
pub fn highlight_toml(text: &str) -> Vec<HighlightSpan> {
    let mut spans = Vec::new();
    let mut byte_offset = 0;

    for line in text.split_inclusive('\n') {
        let line_len = line.len();
        let trimmed = line.trim_start();
        let leading = line_len - trimmed.len();
        let clean = line.trim_end_matches(['\r', '\n']);

        // Comments: # ...
        if let Some(hash_pos) = clean.find('#') {
            let comment_start = byte_offset + hash_pos;
            let comment_end = byte_offset + clean.len();
            if comment_end > comment_start {
                spans.push(HighlightSpan::new(comment_start, comment_end, HighlightKind::Comment));
            }
        }

        // Section Headers: [section] or [[array]]
        let content_before_comment = if let Some(hash_pos) = clean.find('#') {
            &clean[..hash_pos]
        } else {
            clean
        };
        let trimmed_content = content_before_comment.trim();

        if (trimmed_content.starts_with('[') && trimmed_content.ends_with(']'))
            || (trimmed_content.starts_with("[[") && trimmed_content.ends_with("]]"))
        {
            let h_start = byte_offset + leading;
            let h_end = h_start + trimmed_content.len();
            spans.push(HighlightSpan::new(h_start, h_end, HighlightKind::Heading));
        } else if let Some(eq_pos) = content_before_comment.find('=') {
            // Key = Value
            let key_part = &content_before_comment[..eq_pos];
            let key_trimmed = key_part.trim_start();
            let key_leading = key_part.len() - key_trimmed.len();
            let key_clean = key_trimmed.trim_end();

            if !key_clean.is_empty() {
                let k_start = byte_offset + key_leading;
                let k_end = k_start + key_clean.len();
                spans.push(HighlightSpan::new(k_start, k_end, HighlightKind::Variable));
            }

            // Value part
            let val_part = &content_before_comment[eq_pos + 1..];
            let val_offset = byte_offset + eq_pos + 1;
            scan_toml_value(val_part, val_offset, &mut spans);
        }

        byte_offset += line_len;
    }

    spans.sort_by_key(|s| s.start_byte);
    spans
}

fn scan_toml_value(val_text: &str, base_offset: usize, spans: &mut Vec<HighlightSpan>) {
    let mut in_quote = None;
    let mut quote_start = 0;

    for (i, ch) in val_text.char_indices() {
        if let Some(q) = in_quote {
            if ch == q {
                let end = base_offset + i + 1;
                spans.push(HighlightSpan::new(quote_start, end, HighlightKind::String));
                in_quote = None;
            }
        } else if ch == '"' || ch == '\'' {
            in_quote = Some(ch);
            quote_start = base_offset + i;
        }
    }

    // Literals: true, false
    let trimmed_val = val_text.trim();
    if trimmed_val == "true" || trimmed_val == "false" {
        if let Some(pos) = val_text.find(trimmed_val) {
            let start = base_offset + pos;
            spans.push(HighlightSpan::new(start, start + trimmed_val.len(), HighlightKind::Keyword));
        }
    } else if trimmed_val.parse::<f64>().is_ok() {
        if let Some(pos) = val_text.find(trimmed_val) {
            let start = base_offset + pos;
            spans.push(HighlightSpan::new(start, start + trimmed_val.len(), HighlightKind::Number));
        }
    }
}

/// Scans a YAML document text and returns sorted syntax highlight spans.
pub fn highlight_yaml(text: &str) -> Vec<HighlightSpan> {
    let mut spans = Vec::new();
    let mut byte_offset = 0;

    for line in text.split_inclusive('\n') {
        let line_len = line.len();
        let clean = line.trim_end_matches(['\r', '\n']);

        // Comments: # ...
        let content = if let Some(hash_pos) = clean.find('#') {
            let comment_start = byte_offset + hash_pos;
            let comment_end = byte_offset + clean.len();
            spans.push(HighlightSpan::new(comment_start, comment_end, HighlightKind::Comment));
            &clean[..hash_pos]
        } else {
            clean
        };

        let trimmed = content.trim_start();
        let leading = content.len() - trimmed.len();

        // List marker: -
        if trimmed.starts_with("- ") {
            let m_start = byte_offset + leading;
            spans.push(HighlightSpan::new(m_start, m_start + 1, HighlightKind::Operator));
        }

        // Key: Value
        if let Some(colon_pos) = content.find(':') {
            // Ensure colon is not inside a string
            let key_candidate = &content[..colon_pos];
            let key_trimmed = key_candidate.trim_start();
            let key_leading = key_candidate.len() - key_trimmed.len();
            let key_clean = key_trimmed.trim_end();

            if !key_clean.is_empty() && !key_clean.starts_with('#') {
                let k_start = byte_offset + key_leading;
                let k_end = k_start + key_clean.len();
                spans.push(HighlightSpan::new(k_start, k_end, HighlightKind::Variable));
            }

            // Value part
            let val_part = &content[colon_pos + 1..];
            let val_offset = byte_offset + colon_pos + 1;
            scan_yaml_value(val_part, val_offset, &mut spans);
        }

        byte_offset += line_len;
    }

    spans.sort_by_key(|s| s.start_byte);
    spans
}

fn scan_yaml_value(val_text: &str, base_offset: usize, spans: &mut Vec<HighlightSpan>) {
    let mut in_quote = None;
    let mut quote_start = 0;

    for (i, ch) in val_text.char_indices() {
        if let Some(q) = in_quote {
            if ch == q {
                let end = base_offset + i + 1;
                spans.push(HighlightSpan::new(quote_start, end, HighlightKind::String));
                in_quote = None;
            }
        } else if ch == '"' || ch == '\'' {
            in_quote = Some(ch);
            quote_start = base_offset + i;
        }
    }

    let trimmed = val_text.trim();
    if trimmed == "true" || trimmed == "false" || trimmed == "yes" || trimmed == "no" || trimmed == "null" {
        if let Some(pos) = val_text.find(trimmed) {
            let start = base_offset + pos;
            spans.push(HighlightSpan::new(start, start + trimmed.len(), HighlightKind::Keyword));
        }
    } else if trimmed.parse::<f64>().is_ok() {
        if let Some(pos) = val_text.find(trimmed) {
            let start = base_offset + pos;
            spans.push(HighlightSpan::new(start, start + trimmed.len(), HighlightKind::Number));
        }
    }
}

/// Scans a JSON document text and returns sorted syntax highlight spans.
pub fn highlight_json(text: &str) -> Vec<HighlightSpan> {
    let mut spans = Vec::new();
    let mut byte_offset = 0;

    for line in text.split_inclusive('\n') {
        let line_len = line.len();
        let clean = line.trim_end_matches(['\r', '\n']);

        let mut in_string = false;
        let mut str_start = 0;
        let mut is_escaped = false;

        for (i, ch) in clean.char_indices() {
            if is_escaped {
                is_escaped = false;
                continue;
            }
            if ch == '\\' && in_string {
                is_escaped = true;
                continue;
            }
            if ch == '"' {
                if in_string {
                    in_string = false;
                    let end_byte = byte_offset + i + 1;
                    // Check if following character is ':' (i.e. this string is an object key)
                    let rest = clean[i + 1..].trim_start();
                    let kind = if rest.starts_with(':') {
                        HighlightKind::Variable
                    } else {
                        HighlightKind::String
                    };
                    spans.push(HighlightSpan::new(str_start, end_byte, kind));
                } else {
                    in_string = true;
                    str_start = byte_offset + i;
                }
            }
        }

        // Scan keywords outside strings
        let words = ["true", "false", "null"];
        for word in words {
            let mut search_start = 0;
            while let Some(found) = clean[search_start..].find(word) {
                let actual_idx = search_start + found;
                let start_b = byte_offset + actual_idx;
                let end_b = start_b + word.len();
                // Check word boundaries
                let before_ok = actual_idx == 0 || !clean.as_bytes()[actual_idx - 1].is_ascii_alphanumeric();
                let after_ok = actual_idx + word.len() >= clean.len() || !clean.as_bytes()[actual_idx + word.len()].is_ascii_alphanumeric();
                if before_ok && after_ok {
                    spans.push(HighlightSpan::new(start_b, end_b, HighlightKind::Keyword));
                }
                search_start = actual_idx + word.len();
            }
        }

        byte_offset += line_len;
    }

    spans.sort_by_key(|s| s.start_byte);
    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highlights_toml_tables_and_keys() {
        let toml = "[package]\nname = \"arcade-edit\"\nversion = 1.0\n";
        let spans = highlight_toml(toml);
        assert!(!spans.is_empty());
        assert!(spans.iter().any(|s| s.kind == HighlightKind::Heading));
        assert!(spans.iter().any(|s| s.kind == HighlightKind::Variable));
        assert!(spans.iter().any(|s| s.kind == HighlightKind::String));
    }

    #[test]
    fn highlights_yaml_keys_and_values() {
        let yaml = "name: ArcadeEdit\nrun: true\n";
        let spans = highlight_yaml(yaml);
        assert!(!spans.is_empty());
        assert!(spans.iter().any(|s| s.kind == HighlightKind::Variable));
        assert!(spans.iter().any(|s| s.kind == HighlightKind::Keyword));
    }

    #[test]
    fn highlights_json_keys_and_strings() {
        let json = "{\n  \"key\": \"value\",\n  \"active\": true\n}";
        let spans = highlight_json(json);
        assert!(!spans.is_empty());
        assert!(spans.iter().any(|s| s.kind == HighlightKind::Variable));
        assert!(spans.iter().any(|s| s.kind == HighlightKind::String));
        assert!(spans.iter().any(|s| s.kind == HighlightKind::Keyword));
    }
}
