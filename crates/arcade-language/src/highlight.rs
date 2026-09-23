//! Syntax highlighting types, token classification, and line resolution.

/// Semantic category of a syntax token for visual styling.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HighlightKind {
    /// Language keywords and control statements (e.g., `fn`, `let`, `if`, `match`).
    Keyword,
    /// Function and method declarations or invocations.
    Function,
    /// Types, structs, traits, and primitive types (e.g., `String`, `u32`, `Vec`).
    Type,
    /// String and character literals.
    String,
    /// Numeric integer and floating point literals.
    Number,
    /// Line, block, and documentation comments.
    Comment,
    /// Mathematical, logical, bitwise, and assignment operators.
    Operator,
    /// Brackets, braces, parentheses, and punctuation delimiters.
    Punctuation,
    /// Macro invocations (e.g., `println!`, `vec!`).
    Macro,
    /// Compiler annotations and attributes (e.g., `#[derive(...)]`).
    Attribute,
    /// Variable and parameter identifiers.
    Variable,
    /// Constants and static values.
    Constant,
    /// Markdown heading elements.
    Heading,
    /// Markdown inline or fenced code.
    Code,
    /// Unclassified or plain text.
    PlainText,
}

impl HighlightKind {
    /// Returns a priority rank for resolving overlapping highlights.
    ///
    /// Higher numbers take precedence when multiple spans cover the same character.
    pub const fn priority(self) -> u8 {
        match self {
            Self::Comment => 13,
            Self::String => 12,
            Self::Keyword => 11,
            Self::Number => 10,
            Self::Macro => 9,
            Self::Attribute => 8,
            Self::Function => 7,
            Self::Type => 6,
            Self::Constant => 5,
            Self::Heading => 4,
            Self::Code => 4,
            Self::Operator => 3,
            Self::Variable => 2,
            Self::Punctuation => 1,
            Self::PlainText => 0,
        }
    }
}

/// A highlighted byte span within a document's source text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HighlightSpan {
    /// Starting byte offset in the document (inclusive).
    pub start_byte: usize,
    /// Ending byte offset in the document (exclusive).
    pub end_byte: usize,
    /// Syntax category assigned to this span.
    pub kind: HighlightKind,
}

impl HighlightSpan {
    /// Constructs a new highlight span.
    pub const fn new(start_byte: usize, end_byte: usize, kind: HighlightKind) -> Self {
        Self {
            start_byte,
            end_byte,
            kind,
        }
    }
}

/// A rendered line token containing a substring slice and its visual highlight kind.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineToken {
    /// Token text content.
    pub text: String,
    /// Visual syntax token kind.
    pub kind: HighlightKind,
}

impl LineToken {
    /// Constructs a new line token.
    pub fn new(text: impl Into<String>, kind: HighlightKind) -> Self {
        Self {
            text: text.into(),
            kind,
        }
    }
}

/// Maps a Tree-Sitter capture query name (e.g. `"keyword.function"`, `"string"`)
/// to an ArcadeEdit `HighlightKind`.
pub fn map_capture_name(name: &str) -> HighlightKind {
    if name.starts_with("keyword") {
        HighlightKind::Keyword
    } else if name.starts_with("function.macro") {
        HighlightKind::Macro
    } else if name.starts_with("function") || name.starts_with("method") {
        HighlightKind::Function
    } else if name.starts_with("type")
        || name.starts_with("constructor")
        || name.starts_with("module")
        || name.starts_with("namespace")
    {
        HighlightKind::Type
    } else if name.starts_with("string") || name.starts_with("character") {
        HighlightKind::String
    } else if name.starts_with("number") || name.starts_with("float") || name.starts_with("boolean") {
        HighlightKind::Number
    } else if name.starts_with("comment") {
        HighlightKind::Comment
    } else if name.starts_with("operator") {
        HighlightKind::Operator
    } else if name.starts_with("punctuation") {
        HighlightKind::Punctuation
    } else if name.starts_with("attribute") || name.starts_with("label") {
        HighlightKind::Attribute
    } else if name.starts_with("constant") {
        HighlightKind::Constant
    } else if name.starts_with("property") || name.starts_with("field") {
        HighlightKind::Variable
    } else if name.starts_with("variable") {
        HighlightKind::Variable
    } else {
        HighlightKind::PlainText
    }
}

/// Resolves a line of text against global document highlight spans into a sequence
/// of non-overlapping `LineToken`s.
///
/// Ensures strict UTF-8 safety and deterministic resolution of overlapping spans.
pub fn resolve_line_tokens(
    line_text: &str,
    line_start_byte: usize,
    spans: &[HighlightSpan],
) -> Vec<LineToken> {
    if line_text.is_empty() {
        return Vec::new();
    }

    let line_end_byte = line_start_byte + line_text.len();

    // Spans are strictly sorted by `start_byte`.
    // All spans that could overlap [line_start_byte, line_end_byte) MUST have start_byte < line_end_byte.
    // Partition point by start_byte is strictly monotonic and 100% mathematically exact.
    let end_idx = spans.partition_point(|s| s.start_byte < line_end_byte);

    // Collect overlapping candidate spans from spans[..end_idx] scanning backwards.
    let mut relevant_spans: Vec<&HighlightSpan> = Vec::new();
    for span in spans[..end_idx].iter().rev() {
        if span.end_byte > line_start_byte {
            relevant_spans.push(span);
        } else if line_start_byte.saturating_sub(span.start_byte) > 10_000 {
            // No realistic single token spans more than 10KB prior to this line without overlapping
            break;
        }
    }
    relevant_spans.reverse();

    if relevant_spans.is_empty() {
        return vec![LineToken::new(line_text, HighlightKind::PlainText)];
    }

    let mut tokens: Vec<LineToken> = Vec::new();
    let mut current_kind: Option<HighlightKind> = None;
    let mut current_segment = String::new();

    for (c_idx, ch) in line_text.char_indices() {
        let byte_pos = line_start_byte + c_idx;

        // Find the highest priority kind covering this character
        let mut best_kind = HighlightKind::PlainText;
        let mut best_priority = 0;

        for span in &relevant_spans {
            if byte_pos >= span.start_byte && byte_pos < span.end_byte {
                let prio = span.kind.priority();
                if prio >= best_priority {
                    best_priority = prio;
                    best_kind = span.kind;
                }
            }
        }

        match current_kind {
            Some(k) if k == best_kind => {
                current_segment.push(ch);
            }
            Some(k) => {
                tokens.push(LineToken::new(current_segment, k));
                current_segment = String::new();
                current_segment.push(ch);
                current_kind = Some(best_kind);
            }
            None => {
                current_segment.push(ch);
                current_kind = Some(best_kind);
            }
        }
    }

    if let Some(k) = current_kind {
        if !current_segment.is_empty() {
            tokens.push(LineToken::new(current_segment, k));
        }
    }

    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_tree_sitter_capture_names_accurately() {
        assert_eq!(map_capture_name("keyword.function"), HighlightKind::Keyword);
        assert_eq!(map_capture_name("function.method"), HighlightKind::Function);
        assert_eq!(map_capture_name("function.macro"), HighlightKind::Macro);
        assert_eq!(map_capture_name("type.builtin"), HighlightKind::Type);
        assert_eq!(map_capture_name("string"), HighlightKind::String);
        assert_eq!(map_capture_name("comment.line"), HighlightKind::Comment);
        assert_eq!(map_capture_name("number"), HighlightKind::Number);
        assert_eq!(map_capture_name("operator"), HighlightKind::Operator);
        assert_eq!(map_capture_name("unknown.capture"), HighlightKind::PlainText);
    }

    #[test]
    fn resolves_line_tokens_with_overlapping_spans() {
        let line = "let count = 42;";
        // Variable covers 4..9 ("count")
        // Number covers 12..14 ("42")
        // Keyword covers 0..3 ("let")
        let spans = vec![
            HighlightSpan::new(0, 3, HighlightKind::Keyword),
            HighlightSpan::new(4, 9, HighlightKind::Variable),
            HighlightSpan::new(12, 14, HighlightKind::Number),
        ];

        let tokens = resolve_line_tokens(line, 0, &spans);
        assert_eq!(tokens.len(), 6);
        assert_eq!(tokens[0].text, "let");
        assert_eq!(tokens[0].kind, HighlightKind::Keyword);
        assert_eq!(tokens[1].text, " ");
        assert_eq!(tokens[1].kind, HighlightKind::PlainText);
        assert_eq!(tokens[2].text, "count");
        assert_eq!(tokens[2].kind, HighlightKind::Variable);
        assert_eq!(tokens[3].text, " = ");
        assert_eq!(tokens[3].kind, HighlightKind::PlainText);
        assert_eq!(tokens[4].text, "42");
        assert_eq!(tokens[4].kind, HighlightKind::Number);
        assert_eq!(tokens[5].text, ";");
        assert_eq!(tokens[5].kind, HighlightKind::PlainText);
    }

    #[test]
    fn handles_empty_line_and_no_spans() {
        assert_eq!(resolve_line_tokens("", 0, &[]), Vec::new());

        let tokens = resolve_line_tokens("plain text", 0, &[]);
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].text, "plain text");
        assert_eq!(tokens[0].kind, HighlightKind::PlainText);
    }

    #[test]
    fn resolves_line_tokens_with_many_spans_across_multiple_lines() {
        // Generate 120 spans across 30 lines to test partitioning well beyond threshold of 16
        let mut spans = Vec::new();
        let mut line_start = 0;
        let mut lines = Vec::new();

        for i in 0..30 {
            let line = format!("pub fn func_{i}(arg: u32) -> u32 {{ arg + {i} }}\n");
            let func_name = format!("func_{i}");
            let func_start = line.find(&func_name).unwrap();
            let u32_start = line.find("u32").unwrap();

            // "pub" (0..3)
            spans.push(HighlightSpan::new(line_start, line_start + 3, HighlightKind::Keyword));
            // "fn" (4..6)
            spans.push(HighlightSpan::new(line_start + 4, line_start + 6, HighlightKind::Keyword));
            // func_name
            spans.push(HighlightSpan::new(line_start + func_start, line_start + func_start + func_name.len(), HighlightKind::Function));
            // "u32"
            spans.push(HighlightSpan::new(line_start + u32_start, line_start + u32_start + 3, HighlightKind::Type));

            line_start += line.len();
            lines.push(line);
        }

        assert_eq!(spans.len(), 120);
        // Spans are strictly sorted by start_byte
        spans.sort_by_key(|s| s.start_byte);

        // Verify tokens on line 0
        let tokens0 = resolve_line_tokens("pub fn func_0(arg: u32) -> u32 { arg + 0 }", 0, &spans);
        assert!(tokens0.iter().any(|t| t.text == "pub" && t.kind == HighlightKind::Keyword));
        assert!(tokens0.iter().any(|t| t.text == "fn" && t.kind == HighlightKind::Keyword));
        assert!(tokens0.iter().any(|t| t.text == "func_0" && t.kind == HighlightKind::Function));
        assert!(tokens0.iter().any(|t| t.text == "u32" && t.kind == HighlightKind::Type));

        // Verify tokens on line 15 (deep inside document with start_byte > 500)
        let line15_start: usize = lines[..15].iter().map(|l| l.len()).sum();
        let tokens15 = resolve_line_tokens("pub fn func_15(arg: u32) -> u32 { arg + 15 }", line15_start, &spans);
        assert!(tokens15.iter().any(|t| t.text == "pub" && t.kind == HighlightKind::Keyword));
        assert!(tokens15.iter().any(|t| t.text == "fn" && t.kind == HighlightKind::Keyword));
        assert!(tokens15.iter().any(|t| t.text == "func_15" && t.kind == HighlightKind::Function));
        assert!(tokens15.iter().any(|t| t.text == "u32" && t.kind == HighlightKind::Type));

        // Verify tokens on last line 29
        let line29_start: usize = lines[..29].iter().map(|l| l.len()).sum();
        let tokens29 = resolve_line_tokens("pub fn func_29(arg: u32) -> u32 { arg + 29 }", line29_start, &spans);
        assert!(tokens29.iter().any(|t| t.text == "pub" && t.kind == HighlightKind::Keyword));
        assert!(tokens29.iter().any(|t| t.text == "fn" && t.kind == HighlightKind::Keyword));
        assert!(tokens29.iter().any(|t| t.text == "func_29" && t.kind == HighlightKind::Function));
        assert!(tokens29.iter().any(|t| t.text == "u32" && t.kind == HighlightKind::Type));
    }

    #[test]
    fn maps_extended_tree_sitter_captures() {
        assert_eq!(map_capture_name("boolean"), HighlightKind::Number);
        assert_eq!(map_capture_name("property"), HighlightKind::Variable);
        assert_eq!(map_capture_name("field"), HighlightKind::Variable);
        assert_eq!(map_capture_name("constructor"), HighlightKind::Type);
        assert_eq!(map_capture_name("module"), HighlightKind::Type);
        assert_eq!(map_capture_name("namespace"), HighlightKind::Type);
        assert_eq!(map_capture_name("method"), HighlightKind::Function);
        assert_eq!(map_capture_name("label"), HighlightKind::Attribute);
    }
}
