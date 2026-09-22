//! Editor view surface with Solarized minimalist aesthetics, live document rendering, and vibrant syntax highlighting.

use crate::theme::SolarizedTheme;
use arcade_core::{Document, SelectionSet};
use gpui::{div, prelude::*, px, Div, IntoElement};

/// Represents a syntax-highlighted visual token.
#[derive(Clone, Debug)]
pub enum SyntaxKind {
    /// Language keywords and control statements.
    Keyword,
    /// Function and method declarations and invocations.
    Function,
    /// Type, struct, enum, and trait names.
    Type,
    /// String and character literals.
    StringLiteral,
    /// Numeric and integer literals.
    Number,
    /// Code comments and docstrings.
    Comment,
    /// Mathematical and logical operators.
    Operator,
    /// Punctuation, delimiters, and separators.
    Punctuation,
    /// Default plain unstyled text.
    Plain,
}

/// A tokenized segment of text for rich syntax rendering.
#[derive(Clone, Debug)]
pub struct TokenSpan {
    /// The grammatical token classification.
    pub kind: SyntaxKind,
    /// The raw string content of this span.
    pub text: String,
}

/// Fast lexical tokenizer to apply vibrant Solarized colors to live document lines.
pub fn tokenize_line(line_str: &str) -> Vec<TokenSpan> {
    if line_str.trim_start().starts_with("//") || line_str.trim_start().starts_with("#") {
        return vec![TokenSpan {
            kind: SyntaxKind::Comment,
            text: line_str.to_string(),
        }];
    }

    let mut tokens = Vec::new();
    let chars: Vec<char> = line_str.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        let ch = chars[i];

        // Whitespace
        if ch.is_whitespace() {
            let mut end = i + 1;
            while end < len && chars[end].is_whitespace() {
                end += 1;
            }
            tokens.push(TokenSpan {
                kind: SyntaxKind::Plain,
                text: chars[i..end].iter().collect(),
            });
            i = end;
            continue;
        }

        // Line comment
        if ch == '/' && i + 1 < len && chars[i + 1] == '/' {
            tokens.push(TokenSpan {
                kind: SyntaxKind::Comment,
                text: chars[i..].iter().collect(),
            });
            break;
        }

        // String literal
        if ch == '"' || ch == '\'' {
            let quote = ch;
            let mut end = i + 1;
            let mut escaped = false;
            while end < len {
                if escaped {
                    escaped = false;
                } else if chars[end] == '\\' {
                    escaped = true;
                } else if chars[end] == quote {
                    end += 1;
                    break;
                }
                end += 1;
            }
            tokens.push(TokenSpan {
                kind: SyntaxKind::StringLiteral,
                text: chars[i..end].iter().collect(),
            });
            i = end;
            continue;
        }

        // Numbers
        if ch.is_ascii_digit() {
            let mut end = i + 1;
            while end < len && (chars[end].is_ascii_alphanumeric() || chars[end] == '.') {
                end += 1;
            }
            tokens.push(TokenSpan {
                kind: SyntaxKind::Number,
                text: chars[i..end].iter().collect(),
            });
            i = end;
            continue;
        }

        // Identifiers and Keywords
        if ch.is_alphabetic() || ch == '_' {
            let mut end = i + 1;
            while end < len && (chars[end].is_alphanumeric() || chars[end] == '_') {
                end += 1;
            }
            let word: String = chars[i..end].iter().collect();

            let kind = match word.as_str() {
                "use" | "pub" | "fn" | "struct" | "enum" | "impl" | "let" | "mut" | "if" | "else"
                | "match" | "for" | "while" | "loop" | "return" | "break" | "continue" | "mod"
                | "trait" | "type" | "const" | "static" | "as" | "where" | "async" | "await" => {
                    SyntaxKind::Keyword
                }
                "self" | "super" | "crate" => SyntaxKind::Operator,
                "true" | "false" | "Some" | "None" | "Ok" | "Err" => SyntaxKind::Number,
                w if w.chars().next().map_or(false, |c| c.is_uppercase()) => SyntaxKind::Type,
                _ if end < len && chars[end] == '(' => SyntaxKind::Function,
                _ => SyntaxKind::Plain,
            };

            tokens.push(TokenSpan { kind, text: word });
            i = end;
            continue;
        }

        // Punctuation and Operators
        let kind = match ch {
            '+' | '-' | '*' | '/' | '%' | '=' | '!' | '<' | '>' | '&' | '|' | '^' => {
                SyntaxKind::Operator
            }
            '{' | '}' | '(' | ')' | '[' | ']' | ';' | ',' | '.' | ':' => SyntaxKind::Punctuation,
            _ => SyntaxKind::Plain,
        };
        tokens.push(TokenSpan {
            kind,
            text: ch.to_string(),
        });
        i += 1;
    }

    tokens
}

/// Renders the token span with its corresponding Solarized color.
pub fn render_token(theme: &SolarizedTheme, token: &TokenSpan) -> Div {
    let color = match token.kind {
        SyntaxKind::Keyword => theme.syntax_cyan,
        SyntaxKind::Function => theme.syntax_blue,
        SyntaxKind::Type => theme.syntax_yellow,
        SyntaxKind::StringLiteral => theme.syntax_green,
        SyntaxKind::Number => theme.syntax_orange,
        SyntaxKind::Comment => theme.text_muted,
        SyntaxKind::Operator => theme.syntax_magenta,
        SyntaxKind::Punctuation => theme.text_secondary,
        SyntaxKind::Plain => theme.text_primary,
    };

    div().text_color(color).child(token.text.clone())
}

/// Renders the live interactive editor surface for a document and its multi-cursor selection set.
pub fn render_live_editor_surface(
    theme: &SolarizedTheme,
    document: &Document,
    selections: &SelectionSet,
    active_filename: &str,
) -> impl IntoElement {
    let rope = document.rope();
    let num_lines = document.len_lines();

    // Map which lines currently hold a cursor head, and where on the line (in char index)
    let mut cursors_by_line: Vec<(usize, usize)> = Vec::new();
    for sel in selections.as_slice() {
        let line_idx = document.line_of_byte(sel.head);
        let line_start_char = rope.line_to_char(line_idx);
        let head_char = document.byte_to_char(sel.head).unwrap_or(0);
        let col_char = head_char.saturating_sub(line_start_char);
        cursors_by_line.push((line_idx, col_char));
    }

    div()
        .flex_1()
        .flex()
        .flex_col()
        .bg(theme.bg_canvas)
        // Editor Breadcrumb bar with glassy border
        .child(
            div()
                .h(px(32.0))
                .flex()
                .items_center()
                .px_4()
                .bg(theme.bg_surface_glass)
                .border_b_1()
                .border_color(theme.border_subtle)
                .border_t_1()
                .border_color(theme.border_specular_top)
                .text_size(px(12.0))
                .gap_2()
                .child(div().text_color(theme.syntax_cyan).child("ArcadeEdit"))
                .child(div().text_color(theme.text_muted).child("›"))
                .child(div().text_color(theme.syntax_cyan).child("workspace"))
                .child(div().text_color(theme.text_muted).child("›"))
                .child(
                    div()
                        .text_color(theme.text_bright)
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .child(active_filename.to_string()),
                )
                .child(
                    div()
                        .ml_auto()
                        .px_2()
                        .py_0p5()
                        .rounded_full()
                        .bg(theme.badge_bg)
                        .border_1()
                        .border_color(theme.border_glass)
                        .text_size(px(10.5))
                        .text_color(theme.syntax_green)
                        .child(format!("● {} cursors", selections.len())),
                ),
        )
        // Main Editor Code Canvas
        .child(
            div()
                .flex_1()
                .flex()
                .overflow_hidden()
                // Gutter & Lines container
                .child(
                    div()
                        .flex_1()
                        .py_3()
                        .flex()
                        .flex_col()
                        .children((0..num_lines).map(|line_idx| {
                            let line_slice = rope.line(line_idx);
                            let line_str = line_slice.to_string();
                            // Strip line endings for clean rendering
                            let clean_str = line_str.trim_end_matches(&['\r', '\n'][..]);

                            // Check if this line has any cursor
                            let matching_cursor = cursors_by_line
                                .iter()
                                .find(|(l, _)| *l == line_idx)
                                .map(|(_, col)| *col);
                            let is_active = matching_cursor.is_some();

                            div()
                                .flex()
                                .items_center()
                                .h(px(22.0))
                                .w_full()
                                // Active Line Horizontal Sheen
                                .bg(if is_active {
                                    theme.bg_active_glass
                                } else {
                                    gpui::rgba(0x00000000)
                                })
                                .border_l_2()
                                .border_color(if is_active {
                                    theme.syntax_cyan
                                } else {
                                    gpui::rgba(0x00000000)
                                })
                                // Line Number Gutter
                                .child(
                                    div()
                                        .w(px(52.0))
                                        .pr_4()
                                        .flex()
                                        .justify_end()
                                        .text_size(px(12.0))
                                        .text_color(if is_active {
                                            theme.syntax_cyan
                                        } else {
                                            theme.text_muted
                                        })
                                        .child(format!("{}", line_idx + 1)),
                                )
                                // Highlighted Code Spans and Cursor
                                .child({
                                    let mut line_container = div()
                                        .flex()
                                        .items_center()
                                        .text_size(px(13.0));

                                    if let Some(cursor_col) = matching_cursor {
                                        let chars: Vec<char> = clean_str.chars().collect();
                                        let col = std::cmp::min(cursor_col, chars.len());

                                        let before_str: String = chars[..col].iter().collect();
                                        let after_str: String = chars[col..].iter().collect();

                                        let before_tokens = tokenize_line(&before_str);
                                        let after_tokens = tokenize_line(&after_str);

                                        for tok in &before_tokens {
                                            line_container = line_container.child(render_token(theme, tok));
                                        }

                                        // Glowing Cyan Vertical Cursor
                                        line_container = line_container.child(
                                            div()
                                                .w(px(2.0))
                                                .h(px(16.0))
                                                .bg(theme.syntax_cyan)
                                                .rounded_full()
                                                .shadow_sm(),
                                        );

                                        for tok in &after_tokens {
                                            line_container = line_container.child(render_token(theme, tok));
                                        }
                                    } else {
                                        let tokens = tokenize_line(clean_str);
                                        for tok in &tokens {
                                            line_container = line_container.child(render_token(theme, tok));
                                        }
                                    }

                                    line_container
                                })
                        })),
                )
                // Glassy Scroll / Minimap Rail
                .child(
                    div()
                        .w(px(48.0))
                        .bg(theme.bg_surface_glass)
                        .border_l_1()
                        .border_color(theme.border_subtle)
                        .p_2()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .h(px(36.0))
                                .rounded_md()
                                .bg(theme.bg_sheen_highlight)
                                .border_1()
                                .border_color(theme.border_glass)
                                .border_t_1()
                                .border_color(theme.border_specular_top),
                        ),
                ),
        )
}
