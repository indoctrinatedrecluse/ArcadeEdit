//! Editor view surface with Solarized minimalist aesthetics, live document rendering, and vibrant syntax highlighting.

use crate::theme::SolarizedTheme;
use arcade_core::{Document, SelectionSet, TextRange};
use arcade_language::{resolve_line_tokens, HighlightKind, HighlightSpan, LineToken};
use gpui::{div, prelude::*, px, Div, IntoElement, ScrollHandle};

/// Represents a syntax-highlighted visual token (preserved for backward compatibility).
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

/// A tokenized segment of text for rich syntax rendering (preserved for backward compatibility).
#[derive(Clone, Debug)]
pub struct TokenSpan {
    /// The grammatical token classification.
    pub kind: SyntaxKind,
    /// The raw string content of this span.
    pub text: String,
}

/// Fast lexical tokenizer to apply vibrant Solarized colors to live document lines.
pub fn tokenize_line(line_str: &str) -> Vec<LineToken> {
    resolve_line_tokens(line_str, 0, &[])
}

/// Renders a language line token with its corresponding Solarized color.
pub fn render_token(theme: &SolarizedTheme, token: &LineToken) -> Div {
    let color = match token.kind {
        HighlightKind::Keyword => theme.syntax_cyan,
        HighlightKind::Function => theme.syntax_blue,
        HighlightKind::Type => theme.syntax_yellow,
        HighlightKind::String => theme.syntax_green,
        HighlightKind::Number => theme.syntax_orange,
        HighlightKind::Comment => theme.text_muted,
        HighlightKind::Operator => theme.syntax_magenta,
        HighlightKind::Punctuation => theme.text_secondary,
        HighlightKind::Macro => theme.syntax_cyan,
        HighlightKind::Attribute => theme.syntax_violet,
        HighlightKind::Variable => theme.text_bright,
        HighlightKind::Constant => theme.syntax_orange,
        HighlightKind::Heading => theme.syntax_yellow,
        HighlightKind::Code => theme.syntax_green,
        HighlightKind::PlainText => theme.text_primary,
    };

    div().text_color(color).child(token.text.clone())
}

/// Renders the live interactive editor surface for a document and its multi-cursor selection set.
pub fn render_live_editor_surface(
    theme: &SolarizedTheme,
    document: &Document,
    selections: &SelectionSet,
    active_filename: &str,
    highlight_spans: &[HighlightSpan],
    find_matches: &[TextRange],
    active_match: Option<TextRange>,
    scroll_handle: &ScrollHandle,
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
        .size_full()
        .min_h_0()
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
        // Main Editor Code Canvas and Minimap Row
        .child(
            div()
                .flex_1()
                .min_h_0()
                .flex()
                // Scrollable Lines Area
                .child(
                    div()
                        .id("editor-canvas-scroll")
                        .track_scroll(scroll_handle)
                        .flex_1()
                        .min_h_0()
                        .overflow_y_scroll()
                        // Gutter & Lines container
                        .child(
                            div()
                                .w_full()
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

                                    let line_start_byte = rope.line_to_byte(line_idx);
                                    let line_end_byte = line_start_byte + clean_str.len();

                                    let has_active_match = active_match.map_or(false, |m| m.start.0 < line_end_byte && m.end.0 > line_start_byte);
                                    let has_any_match = find_matches.iter().any(|m| m.start.0 < line_end_byte && m.end.0 > line_start_byte);

                                    div()
                                        .flex()
                                        .items_center()
                                        .h(px(22.0))
                                        .w_full()
                                        // Active Line or Find Match Horizontal Sheen
                                        .bg(if is_active {
                                            theme.bg_active_glass
                                        } else if has_active_match {
                                            gpui::rgba(0x2aa1982c)
                                        } else if has_any_match {
                                            gpui::rgba(0xb5890018)
                                        } else {
                                            gpui::rgba(0x00000000)
                                        })
                                        .border_l_2()
                                        .border_color(if is_active || has_active_match {
                                            theme.syntax_cyan
                                        } else if has_any_match {
                                            theme.syntax_yellow
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
                                                .font_family("Consolas, 'Cascadia Code', 'Fira Code', 'Courier New', monospace")
                                                .text_size(px(12.0))
                                                .text_color(if is_active || has_active_match {
                                                    theme.syntax_cyan
                                                } else if has_any_match {
                                                    theme.syntax_yellow
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
                                                .font_family("Consolas, 'Cascadia Code', 'Fira Code', 'Courier New', monospace")
                                                .text_size(px(13.0));

                                            if let Some(cursor_col) = matching_cursor {
                                                let chars: Vec<char> = clean_str.chars().collect();
                                                let col = std::cmp::min(cursor_col, chars.len());

                                                let before_str: String = chars[..col].iter().collect();
                                                let after_str: String = chars[col..].iter().collect();

                                                let before_tokens = resolve_line_tokens(
                                                    &before_str,
                                                    line_start_byte,
                                                    highlight_spans,
                                                );
                                                let after_tokens = resolve_line_tokens(
                                                    &after_str,
                                                    line_start_byte + before_str.len(),
                                                    highlight_spans,
                                                );

                                                for tok in &before_tokens {
                                                    line_container =
                                                        line_container.child(render_token(theme, tok));
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
                                                    line_container =
                                                        line_container.child(render_token(theme, tok));
                                                }
                                            } else {
                                                let tokens = resolve_line_tokens(
                                                    clean_str,
                                                    line_start_byte,
                                                    highlight_spans,
                                                );
                                                for tok in &tokens {
                                                    line_container =
                                                        line_container.child(render_token(theme, tok));
                                                }
                                            }

                                            line_container
                                        })
                                })),
                        ),
                )
                // Glassy Scroll / Minimap Rail (pinned to the right edge)
                .child(
                    div()
                        .w(px(48.0))
                        .h_full()
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
