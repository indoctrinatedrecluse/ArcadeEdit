//! Editor view surface with Solarized minimalist aesthetics and vibrant syntax highlighting.

use crate::theme::SolarizedTheme;
use arcade_core::Document;
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
    pub text: &'static str,
}

/// A single rendered line containing highlighted spans.
#[derive(Clone, Debug)]
pub struct HighlightedLine {
    /// 1-based physical line number in the document.
    pub line_number: usize,
    /// Whether the editor cursor / active line highlight resides on this line.
    pub is_active: bool,
    /// The ordered list of tokenized spans forming this line.
    pub tokens: Vec<TokenSpan>,
}

/// Returns rich, vibrant Solarized demonstration code showing off the syntax palette.
pub fn sample_highlighted_lines() -> Vec<HighlightedLine> {
    vec![
        HighlightedLine {
            line_number: 1,
            is_active: false,
            tokens: vec![
                TokenSpan { kind: SyntaxKind::Comment, text: "//! ArcadeEdit native core runtime — fast, native, text-first." },
            ],
        },
        HighlightedLine {
            line_number: 2,
            is_active: false,
            tokens: vec![
                TokenSpan { kind: SyntaxKind::Keyword, text: "use " },
                TokenSpan { kind: SyntaxKind::Plain, text: "arcade_core::{" },
                TokenSpan { kind: SyntaxKind::Type, text: "Document" },
                TokenSpan { kind: SyntaxKind::Plain, text: ", " },
                TokenSpan { kind: SyntaxKind::Type, text: "TextEdit" },
                TokenSpan { kind: SyntaxKind::Plain, text: ", " },
                TokenSpan { kind: SyntaxKind::Type, text: "ByteOffset" },
                TokenSpan { kind: SyntaxKind::Plain, text: "};" },
            ],
        },
        HighlightedLine {
            line_number: 3,
            is_active: false,
            tokens: vec![
                TokenSpan { kind: SyntaxKind::Keyword, text: "use " },
                TokenSpan { kind: SyntaxKind::Plain, text: "ropey::" },
                TokenSpan { kind: SyntaxKind::Type, text: "Rope" },
                TokenSpan { kind: SyntaxKind::Plain, text: ";" },
            ],
        },
        HighlightedLine {
            line_number: 4,
            is_active: false,
            tokens: vec![],
        },
        HighlightedLine {
            line_number: 5,
            is_active: false,
            tokens: vec![
                TokenSpan { kind: SyntaxKind::Keyword, text: "pub struct " },
                TokenSpan { kind: SyntaxKind::Type, text: "EditorBuffer " },
                TokenSpan { kind: SyntaxKind::Punctuation, text: "{" },
            ],
        },
        HighlightedLine {
            line_number: 6,
            is_active: false,
            tokens: vec![
                TokenSpan { kind: SyntaxKind::Plain, text: "    " },
                TokenSpan { kind: SyntaxKind::Keyword, text: "pub " },
                TokenSpan { kind: SyntaxKind::Plain, text: "doc: " },
                TokenSpan { kind: SyntaxKind::Type, text: "Document" },
                TokenSpan { kind: SyntaxKind::Punctuation, text: "," },
            ],
        },
        HighlightedLine {
            line_number: 7,
            is_active: false,
            tokens: vec![
                TokenSpan { kind: SyntaxKind::Plain, text: "    " },
                TokenSpan { kind: SyntaxKind::Keyword, text: "pub " },
                TokenSpan { kind: SyntaxKind::Plain, text: "cursors: " },
                TokenSpan { kind: SyntaxKind::Type, text: "Vec" },
                TokenSpan { kind: SyntaxKind::Punctuation, text: "<" },
                TokenSpan { kind: SyntaxKind::Type, text: "ByteOffset" },
                TokenSpan { kind: SyntaxKind::Punctuation, text: ">," },
            ],
        },
        HighlightedLine {
            line_number: 8,
            is_active: false,
            tokens: vec![
                TokenSpan { kind: SyntaxKind::Plain, text: "    " },
                TokenSpan { kind: SyntaxKind::Keyword, text: "pub " },
                TokenSpan { kind: SyntaxKind::Plain, text: "glass_sheen_enabled: " },
                TokenSpan { kind: SyntaxKind::Type, text: "bool" },
                TokenSpan { kind: SyntaxKind::Punctuation, text: "," },
            ],
        },
        HighlightedLine {
            line_number: 9,
            is_active: false,
            tokens: vec![
                TokenSpan { kind: SyntaxKind::Punctuation, text: "}" },
            ],
        },
        HighlightedLine {
            line_number: 10,
            is_active: false,
            tokens: vec![],
        },
        HighlightedLine {
            line_number: 11,
            is_active: true,
            tokens: vec![
                TokenSpan { kind: SyntaxKind::Keyword, text: "impl " },
                TokenSpan { kind: SyntaxKind::Type, text: "EditorBuffer " },
                TokenSpan { kind: SyntaxKind::Punctuation, text: "{" },
            ],
        },
        HighlightedLine {
            line_number: 12,
            is_active: false,
            tokens: vec![
                TokenSpan { kind: SyntaxKind::Plain, text: "    " },
                TokenSpan { kind: SyntaxKind::Keyword, text: "pub fn " },
                TokenSpan { kind: SyntaxKind::Function, text: "apply_transaction" },
                TokenSpan { kind: SyntaxKind::Punctuation, text: "(" },
                TokenSpan { kind: SyntaxKind::Operator, text: "&mut " },
                TokenSpan { kind: SyntaxKind::Plain, text: "self, edit: " },
                TokenSpan { kind: SyntaxKind::Type, text: "TextEdit" },
                TokenSpan { kind: SyntaxKind::Punctuation, text: ") -> " },
                TokenSpan { kind: SyntaxKind::Type, text: "Result" },
                TokenSpan { kind: SyntaxKind::Punctuation, text: "<(), ()> {" },
            ],
        },
        HighlightedLine {
            line_number: 13,
            is_active: false,
            tokens: vec![
                TokenSpan { kind: SyntaxKind::Plain, text: "        " },
                TokenSpan { kind: SyntaxKind::Keyword, text: "let " },
                TokenSpan { kind: SyntaxKind::Plain, text: "revision = self.doc." },
                TokenSpan { kind: SyntaxKind::Function, text: "revision" },
                TokenSpan { kind: SyntaxKind::Punctuation, text: "();" },
            ],
        },
        HighlightedLine {
            line_number: 14,
            is_active: false,
            tokens: vec![
                TokenSpan { kind: SyntaxKind::Plain, text: "        " },
                TokenSpan { kind: SyntaxKind::Plain, text: "println!(" },
                TokenSpan { kind: SyntaxKind::StringLiteral, text: "\"✨ Applied edit at revision {}\"" },
                TokenSpan { kind: SyntaxKind::Punctuation, text: ", revision.0);" },
            ],
        },
        HighlightedLine {
            line_number: 15,
            is_active: false,
            tokens: vec![
                TokenSpan { kind: SyntaxKind::Plain, text: "        " },
                TokenSpan { kind: SyntaxKind::Type, text: "Ok" },
                TokenSpan { kind: SyntaxKind::Punctuation, text: "(())" },
            ],
        },
        HighlightedLine {
            line_number: 16,
            is_active: false,
            tokens: vec![
                TokenSpan { kind: SyntaxKind::Plain, text: "    }" },
            ],
        },
        HighlightedLine {
            line_number: 17,
            is_active: false,
            tokens: vec![
                TokenSpan { kind: SyntaxKind::Punctuation, text: "}" },
            ],
        },
    ]
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

    div().text_color(color).child(token.text)
}

/// Renders the full editor surface with gutter, glassy active line sheen, and vibrant tokens.
pub fn render_editor_surface(
    theme: &SolarizedTheme,
    _document: &Document,
    lines: &[HighlightedLine],
) -> impl IntoElement {
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
                .child(
                    div()
                        .text_color(theme.syntax_cyan)
                        .child("crates"),
                )
                .child(div().text_color(theme.text_muted).child("›"))
                .child(
                    div()
                        .text_color(theme.syntax_cyan)
                        .child("arcade-core"),
                )
                .child(div().text_color(theme.text_muted).child("›"))
                .child(
                    div()
                        .text_color(theme.syntax_cyan)
                        .child("src"),
                )
                .child(div().text_color(theme.text_muted).child("›"))
                .child(
                    div()
                        .text_color(theme.text_bright)
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .child("buffer.rs"),
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
                        .child("● Rust 2021"),
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
                        .children(lines.iter().map(|line| {
                            let is_active = line.is_active;

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
                                        .child(format!("{}", line.line_number)),
                                )
                                // Highlighted Code Spans
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .text_size(px(13.0))
                                        .children(
                                            line.tokens
                                                .iter()
                                                .map(|tok| render_token(theme, tok)),
                                        )
                                        // Active Cursor Indicator
                                        .when(is_active, |row| {
                                            row.child(
                                                div()
                                                    .w(px(2.0))
                                                    .h(px(16.0))
                                                    .ml_0p5()
                                                    .bg(theme.syntax_cyan)
                                                    .rounded_full()
                                                    .shadow_sm(),
                                            )
                                        }),
                                )
                        })),
                )
                // Glassy Minimap Placeholder / Scroll Indicator
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

