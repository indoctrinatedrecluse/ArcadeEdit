//! The custom ArcadeEdit desktop UI boundary.
//!
//! GPUI is intentionally introduced here, powering an original Solarized
//! minimalist interface with glassy sheen layers, macOS/MAUI-inspired modals,
//! and vibrant syntax highlighting.

pub mod command_palette;
pub mod editor_view;
pub mod theme;

use arcade_core::Document;
use command_palette::{default_commands, render_command_palette, CommandItem};
use editor_view::{render_editor_surface, sample_highlighted_lines, HighlightedLine};
use gpui::{div, prelude::*, px, Context, IntoElement, Render, Window};
use theme::SolarizedTheme;

/// Identifies the custom visual system used by the desktop application.
pub const DESIGN_SYSTEM_NAME: &str = "ArcadeEdit Solarized Glass";

/// The rich custom-rendered ArcadeEdit desktop application shell.
pub struct ArcadeShell {
    /// Active document buffer representation.
    pub document: Document,
    /// Theme colors and sheen tokens.
    pub theme: SolarizedTheme,
    /// Controls visibility of the macOS/MAUI Command Palette overlay.
    pub show_command_palette: bool,
    /// Live search query filter inside the command palette.
    pub command_query: String,
    /// Available command actions displayed in the palette.
    pub commands: Vec<CommandItem>,
    /// Sample highlighted code lines demonstrated in the editor view.
    pub highlighted_lines: Vec<HighlightedLine>,
    /// Zero-based index of the active document tab.
    pub active_tab: usize,
}

impl ArcadeShell {
    /// Creates the initial shell configured with Solarized Dark and glassy sheen aesthetics.
    pub fn welcome() -> Self {
        let initial_text = "# Welcome to ArcadeEdit\n\nA native text editor with one shared core.\n\n• GPUI renders this custom desktop surface with Solarized glassmorphism.\n• arcade-headless runs without a display or GPU.\n• The document model is Ropey-backed and revisioned.\n";

        Self {
            document: Document::new(initial_text),
            theme: SolarizedTheme::dark(),
            show_command_palette: true,
            command_query: String::new(),
            commands: default_commands(),
            highlighted_lines: sample_highlighted_lines(),
            active_tab: 0,
        }
    }

    /// Toggles the visibility of the macOS/MAUI style Command Palette.
    pub fn toggle_command_palette(&mut self) {
        self.show_command_palette = !self.show_command_palette;
    }

    /// Closes the Command Palette modal.
    pub fn close_command_palette(&mut self) {
        self.show_command_palette = false;
    }

    /// Selects the active tab by its index.
    pub fn select_tab(&mut self, tab: usize) {
        self.active_tab = tab;
    }
}

impl Render for ArcadeShell {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let show_palette = self.show_command_palette;
        let query = &self.command_query;
        let commands = &self.commands;
        let lines = &self.highlighted_lines;
        let active_tab = self.active_tab;

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.bg_canvas)
            .text_color(theme.text_primary)
            // =================================================================
            // 1. TOP HEADER / TITLEBAR (Glassy Sheen & macOS/MAUI Title Styling)
            // =================================================================
            .child(
                div()
                    .h(px(46.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_4()
                    .bg(theme.bg_surface_glass)
                    .border_b_1()
                    .border_color(theme.border_subtle)
                    .border_t_1()
                    .border_color(theme.border_specular_top)
                    .shadow_sm()
                    // Left Brand & Title
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_color(theme.syntax_cyan)
                                            .text_size(px(16.0))
                                            .child("✦"),
                                    )
                                    .child(
                                        div()
                                            .font_weight(gpui::FontWeight::BOLD)
                                            .text_size(px(13.0))
                                            .text_color(theme.text_bright)
                                            .child("ARCADEEDIT"),
                                    ),
                            )
                            .child(
                                div()
                                    .px_2()
                                    .py_0p5()
                                    .rounded_md()
                                    .bg(theme.badge_bg)
                                    .border_1()
                                    .border_color(theme.border_subtle)
                                    .text_size(px(10.0))
                                    .text_color(theme.syntax_cyan)
                                    .child("SOLARIZED GLASS"),
                            ),
                    )
                    // Center Command Palette & Search Trigger (macOS/MAUI Pill)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .px_4()
                            .py_1()
                            .w(px(340.0))
                            .rounded_xl()
                            .bg(theme.bg_input_glass)
                            .border_1()
                            .border_color(theme.border_glass)
                            .border_t_1()
                            .border_color(theme.border_specular_top)
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.bg_hover_glass))
                            .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, _| {
                                this.toggle_command_palette();
                            }))
                            .child(
                                div()
                                    .text_color(theme.syntax_cyan)
                                    .text_size(px(12.0))
                                    .child("🔍"),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .text_size(px(12.0))
                                    .text_color(theme.text_muted)
                                    .child("Quick command or search..."),
                            )
                            .child(
                                div()
                                    .px_1p5()
                                    .py_0p5()
                                    .rounded_md()
                                    .bg(theme.badge_bg)
                                    .border_1()
                                    .border_color(theme.badge_border)
                                    .text_size(px(10.0))
                                    .text_color(theme.text_bright)
                                    .child("Ctrl+P"),
                            ),
                    )
                    // Right Workspace Status & Badges
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .px_2()
                                    .py_0p5()
                                    .rounded_md()
                                    .bg(theme.badge_bg)
                                    .border_1()
                                    .border_color(theme.border_subtle)
                                    .text_size(px(11.0))
                                    .text_color(theme.syntax_yellow)
                                    .child(" master"),
                            )
                            .child(
                                div()
                                    .px_2()
                                    .py_0p5()
                                    .rounded_md()
                                    .bg(theme.badge_bg)
                                    .border_1()
                                    .border_color(theme.border_subtle)
                                    .text_size(px(11.0))
                                    .text_color(theme.syntax_green)
                                    .child("GPU: Direct3D"),
                            ),
                    ),
            )
            // =================================================================
            // 2. TABS BAR (Glassy Translucent Tabs with Specular Sheen)
            // =================================================================
            .child(
                div()
                    .h(px(36.0))
                    .flex()
                    .items_center()
                    .px_2()
                    .gap_1()
                    .bg(theme.bg_canvas)
                    .border_b_1()
                    .border_color(theme.border_subtle)
                    // Active Tab (buffer.rs) - Index 0
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .px_3()
                            .py_1p5()
                            .rounded_t_lg()
                            .bg(if active_tab == 0 {
                                theme.bg_surface_glass
                            } else {
                                theme.bg_canvas
                            })
                            .border_1()
                            .border_color(if active_tab == 0 {
                                theme.border_glass
                            } else {
                                gpui::rgba(0x00000000)
                            })
                            .border_b_0()
                            .border_t_2()
                            .border_color(if active_tab == 0 {
                                theme.syntax_cyan
                            } else {
                                gpui::rgba(0x00000000)
                            })
                            .cursor_pointer()
                            .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, _| {
                                this.select_tab(0);
                            }))
                            .child(
                                div()
                                    .text_color(theme.syntax_cyan)
                                    .text_size(px(12.0))
                                    .child("🦀"),
                            )
                            .child(
                                div()
                                    .text_size(px(12.5))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(if active_tab == 0 {
                                        theme.text_bright
                                    } else {
                                        theme.text_secondary
                                    })
                                    .child("buffer.rs"),
                            )
                            .child(
                                div()
                                    .ml_1()
                                    .text_color(theme.text_muted)
                                    .hover(|s| s.text_color(theme.syntax_magenta))
                                    .text_size(px(11.0))
                                    .child("×"),
                            ),
                    )
                    // Inactive Tab 1 (Welcome.md) - Index 1
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .px_3()
                            .py_1p5()
                            .rounded_t_lg()
                            .bg(if active_tab == 1 {
                                theme.bg_surface_glass
                            } else {
                                theme.bg_canvas
                            })
                            .border_1()
                            .border_color(if active_tab == 1 {
                                theme.border_glass
                            } else {
                                gpui::rgba(0x00000000)
                            })
                            .border_b_0()
                            .border_t_2()
                            .border_color(if active_tab == 1 {
                                theme.syntax_blue
                            } else {
                                gpui::rgba(0x00000000)
                            })
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.bg_hover_glass))
                            .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, _| {
                                this.select_tab(1);
                            }))
                            .child(
                                div()
                                    .text_color(theme.syntax_blue)
                                    .text_size(px(12.0))
                                    .child("📄"),
                            )
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(if active_tab == 1 {
                                        theme.text_bright
                                    } else {
                                        theme.text_secondary
                                    })
                                    .child("Welcome.md"),
                            )
                            .child(
                                div()
                                    .ml_1()
                                    .text_color(theme.text_muted)
                                    .text_size(px(11.0))
                                    .child("×"),
                            ),
                    )
                    // Inactive Tab 2 (Cargo.toml) - Index 2
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .px_3()
                            .py_1p5()
                            .rounded_t_lg()
                            .bg(if active_tab == 2 {
                                theme.bg_surface_glass
                            } else {
                                theme.bg_canvas
                            })
                            .border_1()
                            .border_color(if active_tab == 2 {
                                theme.border_glass
                            } else {
                                gpui::rgba(0x00000000)
                            })
                            .border_b_0()
                            .border_t_2()
                            .border_color(if active_tab == 2 {
                                theme.syntax_orange
                            } else {
                                gpui::rgba(0x00000000)
                            })
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.bg_hover_glass))
                            .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, _| {
                                this.select_tab(2);
                            }))
                            .child(
                                div()
                                    .text_color(theme.syntax_orange)
                                    .text_size(px(12.0))
                                    .child("⚙️"),
                            )
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(if active_tab == 2 {
                                        theme.text_bright
                                    } else {
                                        theme.text_secondary
                                    })
                                    .child("Cargo.toml"),
                            )
                            .child(
                                div()
                                    .ml_1()
                                    .text_color(theme.text_muted)
                                    .text_size(px(11.0))
                                    .child("×"),
                            ),
                    )
                    // Add Tab Button
                    .child(
                        div()
                            .px_2()
                            .py_1()
                            .rounded_md()
                            .text_color(theme.text_muted)
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.bg_hover_glass).text_color(theme.text_bright))
                            .child("+"),
                    ),
            )
            // =================================================================
            // 3. MAIN WORKSPACE (Sidebar + Editor Surface)
            // =================================================================
            .child(
                div()
                    .flex()
                    .flex_1()
                    .relative()
                    // Sidebar
                    .child(
                        div()
                            .w(px(230.0))
                            .bg(theme.bg_surface_glass)
                            .border_r_1()
                            .border_color(theme.border_subtle)
                            .flex()
                            .flex_col()
                            // Sidebar Header
                            .child(
                                div()
                                    .px_4()
                                    .py_2p5()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .border_b_1()
                                    .border_color(theme.border_subtle)
                                    .child(
                                        div()
                                            .text_size(px(11.0))
                                            .font_weight(gpui::FontWeight::BOLD)
                                            .text_color(theme.text_muted)
                                            .child("EXPLORER"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.0))
                                            .text_color(theme.syntax_cyan)
                                            .child("ArcadeEdit"),
                                    ),
                            )
                            // File Tree
                            .child(
                                div()
                                    .p_2()
                                    .flex()
                                    .flex_col()
                                    .gap_0p5()
                                    .text_size(px(12.5))
                                    // crates/ folder
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .px_2()
                                            .py_1()
                                            .text_color(theme.syntax_yellow)
                                            .child("▾ 📁 crates"),
                                    )
                                    // arcade-core/
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .pl_5()
                                            .py_1()
                                            .text_color(theme.syntax_yellow)
                                            .child("▾ 📁 arcade-core"),
                                    )
                                    // buffer.rs (Selected)
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .pl_8()
                                            .py_1()
                                            .rounded_lg()
                                            .bg(theme.bg_active_glass)
                                            .border_1()
                                            .border_color(theme.border_glass)
                                            .text_color(theme.syntax_cyan)
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .child("🦀 buffer.rs"),
                                    )
                                    // lib.rs
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .pl_8()
                                            .py_1()
                                            .text_color(theme.text_secondary)
                                            .hover(|s| s.bg(theme.bg_hover_glass))
                                            .child("🦀 lib.rs"),
                                    )
                                    // arcade-ui/
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .pl_5()
                                            .py_1()
                                            .text_color(theme.text_muted)
                                            .hover(|s| s.bg(theme.bg_hover_glass))
                                            .child("▸ 📁 arcade-ui"),
                                    )
                                    // arcade-desktop/
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .pl_5()
                                            .py_1()
                                            .text_color(theme.text_muted)
                                            .hover(|s| s.bg(theme.bg_hover_glass))
                                            .child("▸ 📁 arcade-desktop"),
                                    )
                                    // Root files
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .px_2()
                                            .py_1()
                                            .text_color(theme.text_secondary)
                                            .hover(|s| s.bg(theme.bg_hover_glass))
                                            .child("⚙️ Cargo.toml"),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .px_2()
                                            .py_1()
                                            .text_color(theme.text_secondary)
                                            .hover(|s| s.bg(theme.bg_hover_glass))
                                            .child("📄 README.md"),
                                    ),
                            ),
                    )
                    // Editor Canvas Surface
                    .child(render_editor_surface(&theme, &self.document, lines))
                    // Command Palette Modal Overlay
                    .when(show_palette, |parent| {
                        parent.child(render_command_palette(&theme, query, commands))
                    }),
            )
            // =================================================================
            // 4. STATUS BAR (Minimalist Glassy Bottom Strip)
            // =================================================================
            .child(
                div()
                    .h(px(28.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_4()
                    .bg(theme.bg_surface_glass)
                    .border_t_1()
                    .border_color(theme.border_subtle)
                    .border_t_1()
                    .border_color(theme.border_specular_top)
                    .text_size(px(11.5))
                    // Left Status Indicators
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(
                                div()
                                    .px_2()
                                    .py_0p5()
                                    .rounded_md()
                                    .bg(theme.syntax_cyan)
                                    .text_color(theme.bg_canvas)
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_size(px(10.0))
                                    .child("NORMAL"),
                            )
                            .child(
                                div()
                                    .text_color(theme.text_muted)
                                    .child("UTF-8  •  LF  •  Rust"),
                            ),
                    )
                    // Right Position & Revision Badges
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_4()
                            .child(
                                div()
                                    .text_color(theme.text_secondary)
                                    .child("Ln 11, Col 24"),
                            )
                            .child(
                                div()
                                    .text_color(theme.text_muted)
                                    .child("Spaces: 4"),
                            )
                            .child(
                                div()
                                    .text_color(theme.syntax_green)
                                    .child("Revision 1 (Clean)"),
                            ),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initializes_arcade_shell_with_solarized_glass_defaults() {
        let shell = ArcadeShell::welcome();
        assert_eq!(shell.active_tab, 0);
        assert!(shell.show_command_palette);
        assert!(!shell.commands.is_empty());
        assert_eq!(DESIGN_SYSTEM_NAME, "ArcadeEdit Solarized Glass");
    }

    #[test]
    fn toggles_command_palette_visibility() {
        let mut shell = ArcadeShell::welcome();
        assert!(shell.show_command_palette);

        shell.toggle_command_palette();
        assert!(!shell.show_command_palette);

        shell.toggle_command_palette();
        assert!(shell.show_command_palette);

        shell.close_command_palette();
        assert!(!shell.show_command_palette);
    }

    #[test]
    fn switches_active_tabs() {
        let mut shell = ArcadeShell::welcome();
        assert_eq!(shell.active_tab, 0);

        shell.select_tab(1);
        assert_eq!(shell.active_tab, 1);

        shell.select_tab(2);
        assert_eq!(shell.active_tab, 2);
    }
}
