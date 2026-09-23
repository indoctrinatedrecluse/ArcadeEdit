//! Help Menu and in-app Documentation modal for ArcadeEdit.
//!
//! Features a dedicated "ir documentation" section detailing the bundled `ir` CLI companion,
//! alongside an "About" section specifying application name (ArcadeEdit), version (1.1.0),
//! author (indoctrinatedrecluse), system architecture, and keyboard shortcuts.

use crate::theme::SolarizedTheme;
use crate::ArcadeShell;
use gpui::{div, prelude::*, px, Context, Div, IntoElement};

/// The selected viewing tab inside the Help modal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HelpSection {
    /// Comprehensive reference documentation for the bundled `ir` CLI utility.
    IrDocs,
    /// About ArcadeEdit, version, author, and architectural breakdown.
    About,
    /// Quick reference cheat-sheet for editor keyboard shortcuts.
    Shortcuts,
}

/// Renders the complete glassy Help modal overlay.
pub fn render_help_modal(
    section: HelpSection,
    theme: &SolarizedTheme,
    cx: &mut Context<ArcadeShell>,
) -> impl IntoElement {
    // Backdrop overlay covering the workspace with a translucent dark acrylic tint
    div()
        .absolute()
        .inset_0()
        .bg(gpui::rgba(0x00101880))
        .flex()
        .justify_center()
        .items_start()
        .pt(px(50.0))
        .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
            this.show_help_modal = false;
            cx.notify();
        }))
        .child(
            // Floating Acrylic Glass Modal Card
            div()
                .w(px(760.0))
                .max_w(px(840.0))
                .h(px(560.0))
                .rounded_2xl()
                .bg(theme.bg_card_glass)
                .border_1()
                .border_color(theme.border_glass)
                .border_t_1()
                .border_color(theme.border_specular_top)
                .shadow_xl()
                .flex()
                .flex_col()
                .overflow_hidden()
                .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| {
                    cx.stop_propagation();
                })
                // =============================================================
                // 1. MODAL HEADER & SECTION TABS
                // =============================================================
                .child(
                    div()
                        .h(px(48.0))
                        .px_5()
                        .flex()
                        .items_center()
                        .justify_between()
                        .bg(theme.bg_surface_glass)
                        .border_b_1()
                        .border_color(theme.border_glass)
                        // Left Title & Section Switchers
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
                                                .text_size(px(14.0))
                                                .child("✦"),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(12.5))
                                                .font_weight(gpui::FontWeight::BOLD)
                                                .text_color(theme.text_bright)
                                                .child("ARCADEEDIT HELP"),
                                        ),
                                )
                                // Tab 1: ir Documentation
                                .child(
                                    div()
                                        .px_3()
                                        .py_1()
                                        .rounded_lg()
                                        .bg(if section == HelpSection::IrDocs {
                                            theme.bg_active_glass
                                        } else {
                                            gpui::rgba(0x00000000)
                                        })
                                        .border_1()
                                        .border_color(if section == HelpSection::IrDocs {
                                            theme.syntax_cyan
                                        } else {
                                            theme.border_subtle
                                        })
                                        .text_size(px(11.5))
                                        .font_weight(if section == HelpSection::IrDocs {
                                            gpui::FontWeight::MEDIUM
                                        } else {
                                            gpui::FontWeight::NORMAL
                                        })
                                        .text_color(if section == HelpSection::IrDocs {
                                            theme.syntax_cyan
                                        } else {
                                            theme.text_secondary
                                        })
                                        .cursor_pointer()
                                        .hover(|s| s.bg(theme.bg_hover_glass))
                                        .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
                                            this.active_help_section = HelpSection::IrDocs;
                                            cx.notify();
                                        }))
                                        .child("📖 ir documentation"),
                                )
                                // Tab 2: About ArcadeEdit
                                .child(
                                    div()
                                        .px_3()
                                        .py_1()
                                        .rounded_lg()
                                        .bg(if section == HelpSection::About {
                                            theme.bg_active_glass
                                        } else {
                                            gpui::rgba(0x00000000)
                                        })
                                        .border_1()
                                        .border_color(if section == HelpSection::About {
                                            theme.syntax_cyan
                                        } else {
                                            theme.border_subtle
                                        })
                                        .text_size(px(11.5))
                                        .font_weight(if section == HelpSection::About {
                                            gpui::FontWeight::MEDIUM
                                        } else {
                                            gpui::FontWeight::NORMAL
                                        })
                                        .text_color(if section == HelpSection::About {
                                            theme.syntax_cyan
                                        } else {
                                            theme.text_secondary
                                        })
                                        .cursor_pointer()
                                        .hover(|s| s.bg(theme.bg_hover_glass))
                                        .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
                                            this.active_help_section = HelpSection::About;
                                            cx.notify();
                                        }))
                                        .child("ℹ️ About"),
                                )
                                // Tab 3: Keyboard Shortcuts
                                .child(
                                    div()
                                        .px_3()
                                        .py_1()
                                        .rounded_lg()
                                        .bg(if section == HelpSection::Shortcuts {
                                            theme.bg_active_glass
                                        } else {
                                            gpui::rgba(0x00000000)
                                        })
                                        .border_1()
                                        .border_color(if section == HelpSection::Shortcuts {
                                            theme.syntax_cyan
                                        } else {
                                            theme.border_subtle
                                        })
                                        .text_size(px(11.5))
                                        .font_weight(if section == HelpSection::Shortcuts {
                                            gpui::FontWeight::MEDIUM
                                        } else {
                                            gpui::FontWeight::NORMAL
                                        })
                                        .text_color(if section == HelpSection::Shortcuts {
                                            theme.syntax_cyan
                                        } else {
                                            theme.text_secondary
                                        })
                                        .cursor_pointer()
                                        .hover(|s| s.bg(theme.bg_hover_glass))
                                        .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
                                            this.active_help_section = HelpSection::Shortcuts;
                                            cx.notify();
                                        }))
                                        .child("⌨️ Shortcuts"),
                                ),
                        )
                        // Right Close Button
                        .child(
                            div()
                                .px_2()
                                .py_1()
                                .rounded_md()
                                .bg(theme.badge_bg)
                                .border_1()
                                .border_color(theme.badge_border)
                                .text_size(px(11.0))
                                .text_color(theme.text_muted)
                                .cursor_pointer()
                                .hover(|s| s.text_color(theme.syntax_magenta).bg(theme.bg_hover_glass))
                                .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
                                    this.show_help_modal = false;
                                    cx.notify();
                                }))
                                .child("ESC ✕"),
                        ),
                )
                // =============================================================
                // 2. MODAL BODY CONTENT
                // =============================================================
                .child(
                    div()
                        .id("help-modal-body-scroll")
                        .flex_1()
                        .p_5()
                        .overflow_y_scroll()
                        .flex()
                        .flex_col()
                        .child(match section {
                            HelpSection::IrDocs => render_ir_docs_content(theme),
                            HelpSection::About => render_about_content(theme),
                            HelpSection::Shortcuts => render_shortcuts_content(theme),
                        }),
                ),
        )
}

/// Renders the comprehensive "ir documentation" section.
fn render_ir_docs_content(theme: &SolarizedTheme) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_3()
        .child(
            // Header Card
            div()
                .p_3()
                .rounded_xl()
                .bg(theme.bg_surface_glass)
                .border_1()
                .border_color(theme.border_glass)
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_0p5()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    div()
                                        .text_size(px(14.0))
                                        .font_weight(gpui::FontWeight::BOLD)
                                        .text_color(theme.text_bright)
                                        .child("🛠️ ir-cli-utility Reference Guide"),
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
                                        .child("v3.8 Bundled"),
                                ),
                        )
                        .child(
                            div()
                                .text_size(px(11.5))
                                .text_color(theme.text_muted)
                                .child("Pure-Rust cross-platform filesystem & shell utility by @indoctrinatedrecluse"),
                        ),
                )
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(theme.syntax_cyan)
                        .child("github.com/indoctrinatedrecluse/ir-cli-utility"),
                ),
        )
        // Commands Grid
        .child(
            div()
                .flex_1()
                .flex()
                .flex_col()
                .gap_2()
                .overflow_hidden()
                // Category 1: Filesystem & Directory Operations
                .child(render_doc_category(
                    theme,
                    "📂 Filesystem Operations",
                    &[
                        ("ir list [path]  (alias 'ls')", "List files and directories with sizes, dates, and permissions"),
                        ("ir create <path> (alias 'touch')", "Create empty files or new directories recursively"),
                        ("ir remove <path> (alias 'rm')", "Remove files or directories safely and recursively"),
                        ("ir copy <src> <dest> (alias 'cp')", "Copy files and folders with high-speed buffered streaming"),
                        ("ir move <src> <dest> (alias 'mv')", "Move or rename files across filesystem paths"),
                        ("ir archive <src> <out> (alias 'tar')", "Create or extract compressed archives (.zip, .tar.gz)"),
                    ],
                ))
                // Category 2: Text Search & Inspection
                .child(render_doc_category(
                    theme,
                    "🔍 Search & Text Tools",
                    &[
                        ("ir cat <file>", "Print file contents to the terminal surface"),
                        ("ir grep <query> [file]", "Search for regex patterns or text in files and standard input"),
                        ("ir search <pattern> <dir>", "Recursively grep across entire directory trees"),
                        ("ir sort [switches] [file]", "Sort lines alphabetically, numerically (-n), or unique (-u)"),
                        ("ir diff <file1> <file2>", "Compare two text files side-by-side with visual diffs"),
                        ("ir find --name <pat>", "Find files by pattern, extension, depth, or emptiness"),
                    ],
                ))
                // Category 3: System Monitors & Interactive TUIs
                .child(render_doc_category(
                    theme,
                    "⚡ System Diagnostics & TUI Utilities",
                    &[
                        ("ir fastfetch (alias 'ff')", "Display system specifications, OS, hardware, and ASCII logo"),
                        ("ir sysinfo (alias 'sys')", "Launch live graphical CPU, RAM, and hardware resource monitor"),
                        ("ir pmon (alias 'ptop')", "Interactive live process monitor with CPU/memory sorting"),
                        ("ir dua (alias 'ncdu')", "Interactive disk space visualizer and folder usage analyzer"),
                        ("ir nettop (alias 'ntop')", "Live graphical network throughput and bandwidth monitor"),
                        ("ir scrape <url> --format <ext>", "Crawl and download linked files by format extension"),
                    ],
                )),
        )
}

/// Helper function to render a command category inside the docs.
fn render_doc_category(
    theme: &SolarizedTheme,
    title: &'static str,
    commands: &[(&'static str, &'static str)],
) -> Div {
    let cmd_rows: Vec<_> = commands
        .iter()
        .map(|(cmd, desc)| {
            div()
                .flex()
                .items_center()
                .justify_between()
                .py_0p5()
                .px_2()
                .rounded_md()
                .hover(|s| s.bg(theme.bg_hover_glass))
                .child(
                    div()
                        .font_family("Consolas, 'Cascadia Code', monospace")
                        .text_size(px(11.0))
                        .text_color(theme.syntax_cyan)
                        .child(*cmd),
                )
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(theme.text_secondary)
                        .child(*desc),
                )
        })
        .collect();

    div()
        .p_2p5()
        .rounded_xl()
        .bg(theme.bg_surface_glass)
        .border_1()
        .border_color(theme.border_subtle)
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .text_size(px(11.5))
                .font_weight(gpui::FontWeight::BOLD)
                .text_color(theme.syntax_yellow)
                .child(title),
        )
        .children(cmd_rows)
}

/// Renders the "About ArcadeEdit" section.
fn render_about_content(theme: &SolarizedTheme) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_4()
        // Brand Hero Card
        .child(
            div()
                .p_4()
                .rounded_2xl()
                .bg(theme.bg_surface_glass)
                .border_1()
                .border_color(theme.border_glass)
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    div()
                                        .text_size(px(22.0))
                                        .text_color(theme.syntax_cyan)
                                        .child("✦"),
                                )
                                .child(
                                    div()
                                        .text_size(px(18.0))
                                        .font_weight(gpui::FontWeight::BOLD)
                                        .text_color(theme.text_bright)
                                        .child("ArcadeEdit"),
                                )
                                .child(
                                    div()
                                        .px_2()
                                        .py_0p5()
                                        .rounded_full()
                                        .bg(theme.badge_bg)
                                        .border_1()
                                        .border_color(theme.syntax_cyan)
                                        .text_size(px(10.5))
                                        .text_color(theme.syntax_cyan)
                                        .child("v1.1.0"),
                                ),
                        )
                        .child(
                            div()
                                .text_size(px(12.5))
                                .text_color(theme.text_secondary)
                                .child("A fast, native text editor with custom GPU-rendered desktop experience and a first-class headless core."),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .items_end()
                        .gap_0p5()
                        .child(
                            div()
                                .text_size(px(11.0))
                                .text_color(theme.text_muted)
                                .child("Created & Maintained by"),
                        )
                        .child(
                            div()
                                .text_size(px(13.0))
                                .font_weight(gpui::FontWeight::BOLD)
                                .text_color(theme.syntax_green)
                                .child("indoctrinatedrecluse"),
                        ),
                ),
        )
        // Architectural Breakdown Card
        .child(
            div()
                .p_4()
                .rounded_xl()
                .bg(theme.bg_surface_glass)
                .border_1()
                .border_color(theme.border_subtle)
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .text_size(px(12.5))
                        .font_weight(gpui::FontWeight::BOLD)
                        .text_color(theme.text_bright)
                        .child("🧱 Architecture & Subsystems"),
                )
                .child(render_about_row(theme, "🧠 arcade-core", "Ropey UTF-8 text storage, multi-cursor transactions, bounded undo/redo, atomic I/O"))
                .child(render_about_row(theme, "🎨 arcade-ui / desktop", "Custom GPUI frontend, Solarized Glass sheen, floating palette, integrated terminal"))
                .child(render_about_row(theme, "🌳 arcade-language", "Asynchronous Tree-Sitter AST syntax highlighting worker with Rust & Markdown grammars"))
                .child(render_about_row(theme, "⚙️ arcade-headless", "Deterministic CLI automation runtime communicating over versioned JSON protocol (v1)"))
                .child(render_about_row(theme, "🛠️ ir-cli-utility", "Bundled companion command-line utility by @indoctrinatedrecluse (v3.8)"))
                .child(render_about_row(theme, "📜 License", "Dual-licensed under MIT and Apache-2.0 terms")),
        )
}

fn render_about_row(theme: &SolarizedTheme, label: &'static str, desc: &'static str) -> Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .py_1()
        .px_2()
        .rounded_md()
        .hover(|s| s.bg(theme.bg_hover_glass))
        .child(
            div()
                .text_size(px(11.5))
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(theme.syntax_cyan)
                .child(label),
        )
        .child(
            div()
                .text_size(px(11.5))
                .text_color(theme.text_secondary)
                .child(desc),
        )
}

/// Renders the "Keyboard Shortcuts" section.
fn render_shortcuts_content(theme: &SolarizedTheme) -> Div {
    let shortcuts = [
        ("Ctrl + P", "Toggle macOS/MAUI Command Palette modal"),
        ("Ctrl + O", "Open native file picker to load document"),
        ("Ctrl + K  Ctrl + O", "Open native folder picker for project workspace"),
        ("Ctrl + S", "Save active document changes to disk"),
        ("Ctrl + `", "Toggle integrated terminal drawer (`ir` CLI)"),
        ("Ctrl + B", "Toggle workspace file explorer sidebar"),
        ("Ctrl + D", "Multi-cursor: add next cursor occurrence"),
        ("Ctrl + Z", "Undo last text editing transaction"),
        ("Ctrl + Y / Ctrl+Shift+Z", "Redo last undone transaction"),
        ("F1", "Open in-app Help menu & documentation modal"),
        ("ESC", "Dismiss command palette, help modal, or collapse selections"),
    ];

    let rows: Vec<_> = shortcuts
        .iter()
        .map(|(key, desc)| {
            div()
                .flex()
                .items_center()
                .justify_between()
                .py_1p5()
                .px_3()
                .rounded_lg()
                .bg(theme.bg_surface_glass)
                .border_1()
                .border_color(theme.border_subtle)
                .child(
                    div()
                        .px_2()
                        .py_0p5()
                        .rounded_md()
                        .bg(theme.badge_bg)
                        .border_1()
                        .border_color(theme.border_glass)
                        .font_family("Consolas, 'Cascadia Code', monospace")
                        .text_size(px(11.0))
                        .text_color(theme.syntax_cyan)
                        .child(*key),
                )
                .child(
                    div()
                        .text_size(px(11.5))
                        .text_color(theme.text_primary)
                        .child(*desc),
                )
        })
        .collect();

    div()
        .flex()
        .flex_col()
        .gap_2()
        .children(rows)
}

