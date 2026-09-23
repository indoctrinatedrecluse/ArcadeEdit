//! Floating Command Palette and Modal components styled with macOS/MAUI glassy sheen.

use crate::theme::SolarizedTheme;
use crate::ArcadeShell;
use gpui::{div, prelude::*, px, Context, Div, IntoElement};

/// A single selectable command action in the palette.
#[derive(Clone, Debug)]
pub struct CommandItem {
    /// Categorical grouping (e.g., "FILE", "VIEW", "TRANSFORM", "TERMINAL").
    pub category: &'static str,
    /// Human-readable title of the command.
    pub title: &'static str,
    /// Detailed description or hint.
    pub description: &'static str,
    /// Keybinding representation (e.g., "Ctrl+Shift+P", "Ctrl+P").
    pub shortcut: Option<&'static str>,
    /// Whether this item is currently focused/selected in the list.
    pub is_selected: bool,
}

impl CommandItem {
    /// Constructs a command item.
    pub const fn new(
        category: &'static str,
        title: &'static str,
        description: &'static str,
        shortcut: Option<&'static str>,
        is_selected: bool,
    ) -> Self {
        Self {
            category,
            title,
            description,
            shortcut,
            is_selected,
        }
    }
}

/// Returns a default curated set of editor commands for the palette.
pub fn default_commands() -> Vec<CommandItem> {
    vec![
        CommandItem::new(
            "FILE",
            "Open Document...",
            "Open any file from disk into the editor",
            Some("Ctrl+O"),
            true,
        ),
        CommandItem::new(
            "FILE",
            "Open Folder...",
            "Open a folder or project workspace into ArcadeEdit",
            Some("Ctrl+K Ctrl+O"),
            false,
        ),
        CommandItem::new(
            "FILE",
            "Save Document",
            "Write the current buffer revision to storage",
            Some("Ctrl+S"),
            false,
        ),
        CommandItem::new(
            "EDIT",
            "Multi-Cursor: Add Next Occurrence",
            "Place an additional cursor at the next match",
            Some("Ctrl+D"),
            false,
        ),
        CommandItem::new(
            "VIEW",
            "Toggle Workspace File Explorer",
            "Show or hide the workspace navigation pane",
            Some("Ctrl+B"),
            false,
        ),
        CommandItem::new(
            "TRANSFORM",
            "Arcade Headless: Preview Edits",
            "Execute headless transformation rules in dry-run mode",
            Some("Ctrl+Shift+P"),
            false,
        ),
        CommandItem::new(
            "TERMINAL",
            "Open Integrated Terminal with `ir`",
            "Launch the companion shell session powered by pure-Rust `ir`",
            Some("Ctrl+`"),
            false,
        ),
        CommandItem::new(
            "THEME",
            "Toggle Solarized Sheen Contrast",
            "Switch between deep acrylic teal and light solarized modes",
            Some("Ctrl+K Ctrl+T"),
            false,
        ),
        CommandItem::new(
            "HELP",
            "ir Documentation",
            "Browse reference manual for the bundled `ir` CLI companion utility",
            Some("F1"),
            false,
        ),
        CommandItem::new(
            "HELP",
            "About ArcadeEdit",
            "View application version, author credits, and system architecture",
            None,
            false,
        ),
        CommandItem::new(
            "HELP",
            "Help: Keyboard Shortcuts",
            "Cheat sheet for common editing and navigation shortcuts",
            None,
            false,
        ),
    ]
}

/// Renders a keycap badge resembling physical macOS/MAUI keyboard shortcuts (`kbd`).
pub fn render_keycap(theme: &SolarizedTheme, shortcut: &str) -> Div {
    let keys: Vec<String> = shortcut.split('+').map(|s| s.to_string()).collect();

    div()
        .flex()
        .items_center()
        .gap_1()
        .children(keys.into_iter().map(|key| {
            div()
                .px_1p5()
                .py_0p5()
                .rounded_md()
                .bg(theme.badge_bg)
                .border_1()
                .border_color(theme.badge_border)
                .border_t_1()
                .border_color(theme.border_specular_top)
                .text_size(px(11.0))
                .text_color(theme.text_bright)
                .shadow_sm()
                .child(key)
        }))
}

/// Renders the complete macOS/MAUI style floating Command Palette overlay.
pub fn render_command_palette(
    theme: &SolarizedTheme,
    query: &str,
    commands: &[CommandItem],
    cx: &mut Context<ArcadeShell>,
) -> impl IntoElement {
    let query_owned = query.to_string();

    let mut command_items = Vec::with_capacity(commands.len());
    for (ix, item) in commands.iter().enumerate() {
        let is_sel = item.is_selected;
        let category_color = match item.category {
            "FILE" => theme.syntax_blue,
            "EDIT" => theme.syntax_cyan,
            "VIEW" => theme.syntax_violet,
            "TRANSFORM" => theme.syntax_magenta,
            "TERMINAL" => theme.syntax_green,
            _ => theme.syntax_yellow,
        };

        let node = div()
            .flex()
            .items_center()
            .justify_between()
            .px_3()
            .py_2()
            .rounded_xl()
            .bg(if is_sel {
                theme.bg_active_glass
            } else {
                gpui::rgba(0x00000000)
            })
            .border_1()
            .border_color(if is_sel {
                theme.border_glass
            } else {
                gpui::rgba(0x00000000)
            })
            .border_t_1()
            .border_color(if is_sel {
                theme.border_specular_top
            } else {
                gpui::rgba(0x00000000)
            })
            .cursor_pointer()
            .hover(|s| s.bg(theme.bg_hover_glass))
            .on_mouse_down(gpui::MouseButton::Left, cx.listener(move |this, _, _, cx| {
                cx.stop_propagation();
                this.execute_command_at(ix);
                cx.notify();
            }))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    // Category Pill
                    .child(
                        div()
                            .px_1p5()
                            .py_0p5()
                            .rounded_md()
                            .bg(theme.badge_bg)
                            .border_1()
                            .border_color(theme.border_subtle)
                            .text_size(px(9.5))
                            .text_color(category_color)
                            .child(item.category),
                    )
                    // Title and description
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_size(px(13.0))
                                    .text_color(if is_sel {
                                        theme.text_bright
                                    } else {
                                        theme.text_primary
                                    })
                                    .child(item.title),
                            )
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(theme.text_muted)
                                    .child(item.description),
                            ),
                    ),
            )
            .when_some(item.shortcut, |el, sc| {
                el.child(render_keycap(theme, sc))
            });

        command_items.push(node);
    }

    let command_list_element = if command_items.is_empty() {
        div()
            .id("command-palette-empty")
            .py_6()
            .px_4()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_1()
            .child(
                div()
                    .text_size(px(13.0))
                    .text_color(theme.text_muted)
                    .child(format!("No commands matching \"{}\"", query_owned)),
            )
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(theme.syntax_cyan)
                    .child("Press Backspace to clear query or ESC to close"),
            )
    } else {
        div()
            .id("command-palette-scroll")
            .max_h(px(380.0))
            .overflow_y_scroll()
            .p_2()
            .flex()
            .flex_col()
            .gap_1()
            .children(command_items)
    };

    // Backdrop overlay covering the workspace with a translucent tint
    div()
        .absolute()
        .inset_0()
        .bg(gpui::rgba(0x00101880))
        .flex()
        .justify_center()
        .items_start()
        .pt(px(72.0))
        .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
            this.close_command_palette();
            cx.notify();
        }))
        .child(
            // Floating Acrylic Glass Modal Card
            div()
                .w(px(600.0))
                .max_w(px(720.0))
                .rounded_2xl()
                .bg(theme.bg_card_glass)
                .border_1()
                .border_color(theme.border_glass)
                .border_t_1()
                .border_color(theme.border_specular_top)
                .shadow_lg()
                .flex()
                .flex_col()
                .overflow_hidden()
                .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| {
                    cx.stop_propagation();
                })
                // Top Specular Sheen Stripe
                .child(
                    div()
                        .h(px(1.5))
                        .w_full()
                        .bg(theme.border_specular_top),
                )
                // Search Input Header Bar
                .child(
                    div()
                        .flex()
                        .items_center()
                        .px_4()
                        .py_3()
                        .border_b_1()
                        .border_color(theme.border_subtle)
                        .child(
                            div()
                                .mr_3()
                                .text_color(theme.syntax_cyan)
                                .text_size(px(15.0))
                                .child("✦"),
                        )
                        .child(
                            div()
                                .flex_1()
                                .px_3()
                                .py_1p5()
                                .rounded_lg()
                                .bg(theme.bg_input_glass)
                                .border_1()
                                .border_color(theme.border_glass)
                                .text_color(if query_owned.is_empty() {
                                    theme.text_muted
                                } else {
                                    theme.text_bright
                                })
                                .text_size(px(13.5))
                                .child(if query_owned.is_empty() {
                                    "Type a command or search (e.g. 'open', 'terminal', 'theme')".to_string()
                                } else {
                                    format!("{}▌", query_owned)
                                }),
                        )
                        .child(
                            div()
                                .ml_3()
                                .px_2p5()
                                .py_1()
                                .rounded_md()
                                .bg(theme.badge_bg)
                                .border_1()
                                .border_color(theme.border_subtle)
                                .text_size(px(11.0))
                                .text_color(theme.text_secondary)
                                .cursor_pointer()
                                .hover(|s| s.bg(theme.bg_hover_glass).text_color(theme.text_bright))
                                .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
                                    cx.stop_propagation();
                                    this.close_command_palette();
                                    cx.notify();
                                }))
                                .child("ESC ✕"),
                        ),
                )
                // Command List
                .child(command_list_element)
                // Bottom hint bar
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .px_4()
                        .py_2()
                        .bg(theme.bg_input_glass)
                        .border_t_1()
                        .border_color(theme.border_subtle)
                        .text_size(px(11.0))
                        .text_color(theme.text_muted)
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child("↑↓ to navigate")
                                .child("•")
                                .child("↵ to execute")
                                .child("•")
                                .child("ESC to close"),
                        )
                        .child(
                            div()
                                .text_color(theme.syntax_cyan)
                                .child("ArcadeEdit Protocol v1"),
                        ),
                ),
        )
}

