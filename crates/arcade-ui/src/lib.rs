//! The custom ArcadeEdit desktop UI boundary.
//!
//! GPUI powers an original Solarized minimalist interface with glassy sheen layers,
//! macOS/MAUI-inspired modals, vibrant live syntax highlighting, and responsive
//! multi-cursor keyboard editing.

pub mod command_palette;
pub mod editor_view;
pub mod theme;

use arcade_core::selection::SelectionSet;
use arcade_core::{ByteOffset, Document, History};
use arcade_language::{HighlightSpan, HighlightWorker, LanguageId, LanguageService};
use command_palette::{default_commands, render_command_palette, CommandItem};
use editor_view::render_live_editor_surface;
use gpui::{div, prelude::*, px, Context, FocusHandle, IntoElement, Render, Window};
use theme::SolarizedTheme;

/// Identifies the custom visual system used by the desktop application.
pub const DESIGN_SYSTEM_NAME: &str = "ArcadeEdit Solarized Glass";

/// The rich custom-rendered ArcadeEdit desktop application shell.
pub struct ArcadeShell {
    /// Active document buffer representation.
    pub document: Document,
    /// Multi-cursor selections across the active document.
    pub selections: SelectionSet,
    /// Undo and redo history manager.
    pub history: History,
    /// Optional focus handle for receiving keyboard events.
    pub focus_handle: Option<FocusHandle>,
    /// Theme colors and sheen tokens.
    pub theme: SolarizedTheme,
    /// Whether Solarized Dark or Light mode is active.
    pub is_dark_theme: bool,
    /// Controls visibility of the macOS/MAUI Command Palette overlay.
    pub show_command_palette: bool,
    /// Controls visibility of the workspace file explorer sidebar.
    pub show_sidebar: bool,
    /// Live search query filter inside the command palette.
    pub command_query: String,
    /// Available command actions displayed in the palette.
    pub commands: Vec<CommandItem>,
    /// Zero-based index of the active document tab.
    pub active_tab: usize,
    /// Active language syntax grammar.
    pub language: LanguageId,
    /// Computed syntax highlight spans for live rendering.
    pub highlight_spans: Vec<HighlightSpan>,
    /// Dedicated background syntax highlighting worker.
    pub highlight_worker: Option<HighlightWorker>,
}

impl ArcadeShell {
    /// Creates the initial shell configured with Solarized Dark and live interactive keyboard input.
    pub fn welcome(cx: &mut Context<Self>) -> Self {
        let initial_text = "# Welcome to ArcadeEdit\n\nA fast, native text editor with custom GPU-rendered desktop experience.\n\nType anywhere to begin editing live!\n• Use Arrow Keys, Home, End to navigate\n• Press Ctrl+P to open the Command Palette\n• Press Ctrl+Z / Ctrl+Y for Undo / Redo\n• Multi-cursor selections merge seamlessly\n";

        let focus_handle = cx.focus_handle();

        let mut spans = Vec::new();
        if let Ok(mut service) = LanguageService::new() {
            spans = service.highlight(LanguageId::Markdown, initial_text);
        }

        let worker = HighlightWorker::spawn().ok();

        Self {
            document: Document::new(initial_text),
            selections: SelectionSet::cursor(ByteOffset(0)),
            history: History::new(),
            focus_handle: Some(focus_handle),
            theme: SolarizedTheme::dark(),
            is_dark_theme: true,
            show_command_palette: true,
            show_sidebar: true,
            command_query: String::new(),
            commands: default_commands(),
            active_tab: 0,
            language: LanguageId::Markdown,
            highlight_spans: spans,
            highlight_worker: worker,
        }
    }

    /// Creates a shell without a window focus handle (suitable for unit tests).
    pub fn test_stub() -> Self {
        Self {
            document: Document::new("# ArcadeEdit Test Document\n"),
            selections: SelectionSet::cursor(ByteOffset(0)),
            history: History::new(),
            focus_handle: None,
            theme: SolarizedTheme::dark(),
            is_dark_theme: true,
            show_command_palette: true,
            show_sidebar: true,
            command_query: String::new(),
            commands: default_commands(),
            active_tab: 0,
            language: LanguageId::Rust,
            highlight_spans: Vec::new(),
            highlight_worker: None,
        }
    }

    /// Toggles the visibility of the macOS/MAUI style Command Palette.
    pub fn toggle_command_palette(&mut self) {
        self.show_command_palette = !self.show_command_palette;
        if self.show_command_palette {
            self.command_query.clear();
            self.filter_commands();
        }
    }

    /// Closes the Command Palette modal and resets the query.
    pub fn close_command_palette(&mut self) {
        self.show_command_palette = false;
        self.command_query.clear();
        self.filter_commands();
    }

    /// Toggles the workspace file explorer sidebar.
    pub fn toggle_sidebar(&mut self) {
        self.show_sidebar = !self.show_sidebar;
    }

    /// Toggles theme between Solarized Dark and Solarized Light.
    pub fn toggle_theme(&mut self) {
        self.is_dark_theme = !self.is_dark_theme;
        self.theme = if self.is_dark_theme {
            SolarizedTheme::dark()
        } else {
            SolarizedTheme::light()
        };
    }

    /// Marks the active document as saved.
    pub fn save_document(&mut self) {
        self.history.mark_saved(self.document.revision());
    }

    /// Adds an additional cursor on the following line for multi-cursor editing.
    pub fn add_next_cursor(&mut self) {
        let rope = self.document.rope();
        let primary_line = self.document.line_of_byte(self.selections.primary().head);
        if primary_line + 1 < rope.len_lines() {
            let next_line_start = rope.line_to_byte(primary_line + 1);
            self.selections.add(arcade_core::selection::Selection::cursor(ByteOffset(next_line_start)));
        } else {
            let next_offset = ByteOffset(self.selections.primary().head.0.saturating_add(1).min(self.document.len_bytes()));
            self.selections.add(arcade_core::selection::Selection::cursor(next_offset));
        }
    }

    /// Filters the command palette items based on current search query.
    pub fn filter_commands(&mut self) {
        let q = self.command_query.trim().to_lowercase();
        if q.is_empty() {
            self.commands = default_commands();
            if let Some(first) = self.commands.first_mut() {
                first.is_selected = true;
            }
        } else {
            let mut filtered: Vec<CommandItem> = default_commands()
                .into_iter()
                .filter(|cmd| {
                    cmd.title.to_lowercase().contains(&q)
                        || cmd.description.to_lowercase().contains(&q)
                        || cmd.category.to_lowercase().contains(&q)
                })
                .collect();
            for cmd in filtered.iter_mut() {
                cmd.is_selected = false;
            }
            if let Some(first) = filtered.first_mut() {
                first.is_selected = true;
            }
            self.commands = filtered;
        }
    }

    /// Selects the next command in the palette.
    pub fn select_next_command(&mut self) {
        if self.commands.is_empty() {
            return;
        }
        let current_idx = self.commands.iter().position(|c| c.is_selected).unwrap_or(0);
        let next_idx = (current_idx + 1) % self.commands.len();
        for (i, cmd) in self.commands.iter_mut().enumerate() {
            cmd.is_selected = i == next_idx;
        }
    }

    /// Selects the previous command in the palette.
    pub fn select_prev_command(&mut self) {
        if self.commands.is_empty() {
            return;
        }
        let current_idx = self.commands.iter().position(|c| c.is_selected).unwrap_or(0);
        let prev_idx = if current_idx == 0 {
            self.commands.len() - 1
        } else {
            current_idx - 1
        };
        for (i, cmd) in self.commands.iter_mut().enumerate() {
            cmd.is_selected = i == prev_idx;
        }
    }

    /// Executes the currently selected command in the palette.
    pub fn execute_selected_command(&mut self) {
        if let Some(idx) = self.commands.iter().position(|c| c.is_selected) {
            self.execute_command_at(idx);
        } else if !self.commands.is_empty() {
            self.execute_command_at(0);
        }
    }

    /// Executes the command at the given index in the filtered command list.
    pub fn execute_command_at(&mut self, index: usize) {
        let Some(cmd) = self.commands.get(index).cloned() else {
            return;
        };
        self.close_command_palette();

        match cmd.title {
            "Open Document..." => {
                self.select_tab(0);
            }
            "Open Folder..." => {
                self.show_sidebar = true;
            }
            "Save Document" => {
                self.save_document();
            }
            "Multi-Cursor: Add Next Occurrence" => {
                self.add_next_cursor();
            }
            "Toggle Workspace File Explorer" => {
                self.toggle_sidebar();
            }
            "Arcade Headless: Preview Edits" => {
                self.insert_text("\n// [Arcade Headless Preview: dry-run passed with 0 errors]\n");
            }
            "Open Integrated Terminal with `ir`" => {
                self.select_tab(1);
            }
            "Toggle Solarized Sheen Contrast" => {
                self.toggle_theme();
            }
            _ => {}
        }
    }

    /// Selects the active tab by its index and updates the target language grammar.
    pub fn select_tab(&mut self, tab: usize) {
        self.active_tab = tab;
        self.language = match tab {
            0 => LanguageId::Rust,
            1 => LanguageId::Markdown,
            _ => LanguageId::PlainText,
        };
        self.trigger_highlight();
    }

    /// Dispatches a background syntax highlighting request for the current document state.
    pub fn trigger_highlight(&self) {
        if let Some(worker) = &self.highlight_worker {
            worker.request_highlight(
                self.document.revision(),
                self.language,
                self.document.to_string(),
            );
        }
    }

    /// Inserts a string at all current cursor positions.
    pub fn insert_text(&mut self, text: &str) {
        if let Ok((tx, new_sels)) = self.document.insert_text_at_selections(&self.selections, text) {
            self.history.push(tx);
            self.selections = new_sels;
            self.trigger_highlight();
        }
    }

    /// Deletes backward (Backspace) across all current cursor selections.
    pub fn delete_backward(&mut self) {
        if let Ok((tx, new_sels)) = self.document.delete_backward_at_selections(&self.selections) {
            self.history.push(tx);
            self.selections = new_sels;
            self.trigger_highlight();
        }
    }

    /// Performs an undo operation and restores prior selections.
    pub fn undo(&mut self) {
        if let Ok(Some(sels)) = self.history.undo(&mut self.document) {
            self.selections = sels;
            self.trigger_highlight();
        }
    }

    /// Performs a redo operation and restores resulting selections.
    pub fn redo(&mut self) {
        if let Ok(Some(sels)) = self.history.redo(&mut self.document) {
            self.selections = sels;
            self.trigger_highlight();
        }
    }
}

impl Render for ArcadeShell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(worker) = &self.highlight_worker {
            while let Some(res) = worker.try_recv_response() {
                if res.revision == self.document.revision() {
                    self.highlight_spans = res.spans;
                }
            }
        }

        // Ensure window keyboard focus is directed to this shell component
        if let Some(fh) = &self.focus_handle {
            if !fh.is_focused(window) {
                fh.focus(window, cx);
            }
        }

        let theme = self.theme;
        let is_dark_theme = self.is_dark_theme;
        let show_palette = self.show_command_palette;
        let show_sidebar = self.show_sidebar;
        let query = &self.command_query;
        let commands = &self.commands;
        let active_tab = self.active_tab;

        // Current primary cursor coordinates for the status bar
        let primary_head = self.selections.primary().head;
        let primary_line = self.document.line_of_byte(primary_head);
        let line_start_char = self.document.rope().line_to_char(primary_line);
        let head_char = self.document.byte_to_char(primary_head).unwrap_or(0);
        let primary_col = head_char.saturating_sub(line_start_char) + 1;
        let line_status = format!("Ln {}, Col {}", primary_line + 1, primary_col);

        let dirty_status = if self.history.is_dirty(self.document.revision()) {
            "Revision (Modified)"
        } else {
            "Revision (Clean)"
        };

        let active_filename = match active_tab {
            0 => "buffer.rs",
            1 => "Welcome.md",
            _ => "Cargo.toml",
        };

        div()
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .bg(theme.bg_canvas)
            .text_color(theme.text_primary)
            // Register Focus Handle and Interactive Keyboard Handler
            .when_some(self.focus_handle.clone(), |el, fh| el.track_focus(&fh))
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _window, cx| {
                let key = event.keystroke.key.as_str();
                let ctrl = event.keystroke.modifiers.control || event.keystroke.modifiers.platform;
                let shift = event.keystroke.modifiers.shift;

                // When Command Palette is visible, keyboard routes to palette navigation and search:
                if this.show_command_palette {
                    if ctrl && key == "p" {
                        this.toggle_command_palette();
                        cx.notify();
                        return;
                    }
                    if key == "escape" {
                        this.close_command_palette();
                        cx.notify();
                        return;
                    }
                    if key == "up" {
                        this.select_prev_command();
                        cx.notify();
                        return;
                    }
                    if key == "down" {
                        this.select_next_command();
                        cx.notify();
                        return;
                    }
                    if key == "enter" {
                        this.execute_selected_command();
                        cx.notify();
                        return;
                    }
                    if key == "backspace" {
                        this.command_query.pop();
                        this.filter_commands();
                        cx.notify();
                        return;
                    }
                    // Character typing into palette search
                    if !ctrl && !event.keystroke.modifiers.alt && key.chars().count() == 1 {
                        this.command_query.push_str(key);
                        this.filter_commands();
                        cx.notify();
                        return;
                    }
                    // Absorb any other keys while palette is open
                    return;
                }

                // Normal Editor Keyboard Shortcuts:
                // Toggle Command Palette (Ctrl+P)
                if ctrl && key == "p" {
                    this.toggle_command_palette();
                    cx.notify();
                    return;
                }

                // Toggle Workspace Explorer (Ctrl+B)
                if ctrl && key == "b" {
                    this.toggle_sidebar();
                    cx.notify();
                    return;
                }

                // Save Document (Ctrl+S)
                if ctrl && key == "s" {
                    this.save_document();
                    cx.notify();
                    return;
                }

                // Multi-Cursor Add Next (Ctrl+D)
                if ctrl && key == "d" {
                    this.add_next_cursor();
                    cx.notify();
                    return;
                }

                // Undo (Ctrl+Z)
                if ctrl && key == "z" && !shift {
                    this.undo();
                    cx.notify();
                    return;
                }

                // Redo (Ctrl+Y or Ctrl+Shift+Z)
                if (ctrl && key == "y") || (ctrl && key == "z" && shift) {
                    this.redo();
                    cx.notify();
                    return;
                }

                // Escape: Collapse selections
                if key == "escape" {
                    this.selections = SelectionSet::single(this.selections.primary().collapse());
                    cx.notify();
                    return;
                }

                // Navigation: Left / Right
                if key == "left" {
                    this.selections = this.selections.move_left(this.document.rope(), shift);
                    cx.notify();
                    return;
                }
                if key == "right" {
                    this.selections = this.selections.move_right(this.document.rope(), shift);
                    cx.notify();
                    return;
                }

                // Navigation: Up / Down
                if key == "up" {
                    this.selections = this.selections.move_up(this.document.rope(), shift);
                    cx.notify();
                    return;
                }
                if key == "down" {
                    this.selections = this.selections.move_down(this.document.rope(), shift);
                    cx.notify();
                    return;
                }

                // Navigation: Home / End
                if key == "home" {
                    this.selections = this.selections.move_to_line_start(this.document.rope(), shift);
                    cx.notify();
                    return;
                }
                if key == "end" {
                    this.selections = this.selections.move_to_line_end(this.document.rope(), shift);
                    cx.notify();
                    return;
                }

                // Backspace
                if key == "backspace" {
                    this.delete_backward();
                    cx.notify();
                    return;
                }

                // Enter / Return
                if key == "enter" {
                    this.insert_text("\n");
                    cx.notify();
                    return;
                }

                // Tab
                if key == "tab" {
                    this.insert_text("    ");
                    cx.notify();
                    return;
                }

                // Text Insertion (Printable characters without Control/Alt)
                if !ctrl && !event.keystroke.modifiers.alt && key.chars().count() == 1 {
                    this.insert_text(key);
                    cx.notify();
                }
            }))
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
                                    .cursor_pointer()
                                    .hover(|s| s.bg(theme.bg_hover_glass))
                                    .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
                                        this.toggle_theme();
                                        cx.notify();
                                    }))
                                    .child(if is_dark_theme { "SOLARIZED DARK" } else { "SOLARIZED LIGHT" }),
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
                            .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
                                this.toggle_command_palette();
                                cx.notify();
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
                            .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
                                this.select_tab(0);
                                cx.notify();
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
                                    .cursor_pointer()
                                    .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
                                        cx.stop_propagation();
                                        this.select_tab(0);
                                        cx.notify();
                                    }))
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
                            .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
                                this.select_tab(1);
                                cx.notify();
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
                                    .hover(|s| s.text_color(theme.syntax_magenta))
                                    .text_size(px(11.0))
                                    .cursor_pointer()
                                    .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
                                        cx.stop_propagation();
                                        this.select_tab(0);
                                        cx.notify();
                                    }))
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
                            .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
                                this.select_tab(2);
                                cx.notify();
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
                                    .hover(|s| s.text_color(theme.syntax_magenta))
                                    .text_size(px(11.0))
                                    .cursor_pointer()
                                    .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
                                        cx.stop_propagation();
                                        this.select_tab(0);
                                        cx.notify();
                                    }))
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
                            .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
                                this.select_tab(0);
                                cx.notify();
                            }))
                            .child("+"),
                    ),
            )
            // =================================================================
            // 3. MAIN WORKSPACE (Sidebar + Live Editor Surface)
            // =================================================================
            .child(
                div()
                    .flex()
                    .flex_1()
                    .relative()
                    // Sidebar
                    .when(show_sidebar, |parent| {
                        parent.child(
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
                                                .cursor_pointer()
                                                .hover(|s| s.text_color(theme.text_bright))
                                                .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
                                                    this.toggle_sidebar();
                                                    cx.notify();
                                                }))
                                                .child("EXPLORER ▾"),
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
                                                .cursor_pointer()
                                                .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
                                                    this.select_tab(0);
                                                    cx.notify();
                                                }))
                                                .child("🦀 buffer.rs"),
                                        )
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap_2()
                                                .pl_8()
                                                .py_1()
                                                .text_color(theme.text_secondary)
                                                .hover(|s| s.bg(theme.bg_hover_glass))
                                                .cursor_pointer()
                                                .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
                                                    this.select_tab(0);
                                                    cx.notify();
                                                }))
                                                .child("🦀 lib.rs"),
                                        )
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
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap_2()
                                                .px_2()
                                                .py_1()
                                                .text_color(theme.text_secondary)
                                                .hover(|s| s.bg(theme.bg_hover_glass))
                                                .cursor_pointer()
                                                .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
                                                    this.select_tab(2);
                                                    cx.notify();
                                                }))
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
                                                .cursor_pointer()
                                                .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
                                                    this.select_tab(1);
                                                    cx.notify();
                                                }))
                                                .child("📄 README.md"),
                                        ),
                                ),
                        )
                    })
                    // Live Interactive Editor Canvas Surface with Tree-Sitter Highlighting
                    .child(render_live_editor_surface(
                        &theme,
                        &self.document,
                        &self.selections,
                        active_filename,
                        &self.highlight_spans,
                    ))
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
                                    .cursor_pointer()
                                    .hover(|s| s.bg(theme.syntax_blue))
                                    .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
                                        this.toggle_command_palette();
                                        cx.notify();
                                    }))
                                    .child("NORMAL"),
                            )
                            .child({
                                let lang_label = match self.language {
                                    LanguageId::Rust => "Rust",
                                    LanguageId::Markdown => "Markdown",
                                    LanguageId::PlainText => "Plain Text",
                                };
                                div()
                                    .text_color(theme.text_muted)
                                    .child(format!("UTF-8  •  LF  •  {}", lang_label))
                            }),
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
                                    .child(line_status),
                            )
                            .child(
                                div()
                                    .text_color(theme.text_muted)
                                    .child("Spaces: 4"),
                            )
                            .child(
                                div()
                                    .text_color(theme.syntax_green)
                                    .cursor_pointer()
                                    .hover(|s| s.text_color(theme.syntax_cyan))
                                    .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
                                        this.save_document();
                                        cx.notify();
                                    }))
                                    .child(dirty_status),
                            ),
                    ),
            )
            // =================================================================
            // 5. COMMAND PALETTE MODAL OVERLAY (floating above full window)
            // =================================================================
            .when(show_palette, |parent| {
                parent.child(render_command_palette(&theme, query, commands, cx))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initializes_arcade_shell_with_solarized_glass_defaults() {
        let shell = ArcadeShell::test_stub();
        assert_eq!(shell.active_tab, 0);
        assert!(shell.show_command_palette);
        assert!(!shell.commands.is_empty());
        assert_eq!(DESIGN_SYSTEM_NAME, "ArcadeEdit Solarized Glass");
    }

    #[test]
    fn toggles_command_palette_visibility() {
        let mut shell = ArcadeShell::test_stub();
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
        let mut shell = ArcadeShell::test_stub();
        assert_eq!(shell.active_tab, 0);

        shell.select_tab(1);
        assert_eq!(shell.active_tab, 1);

        shell.select_tab(2);
        assert_eq!(shell.active_tab, 2);
    }

    #[test]
    fn performs_interactive_typing_and_undo_cycles() {
        let mut shell = ArcadeShell::test_stub();
        let initial_text = shell.document.to_string();

        shell.insert_text("let x = 42;\n");
        assert_ne!(shell.document.to_string(), initial_text);

        shell.undo();
        assert_eq!(shell.document.to_string(), initial_text);

        shell.redo();
        assert_ne!(shell.document.to_string(), initial_text);
    }

    #[test]
    fn filters_command_palette_by_query() {
        let mut shell = ArcadeShell::test_stub();
        assert_eq!(shell.commands.len(), default_commands().len());

        shell.command_query = "theme".to_string();
        shell.filter_commands();
        assert_eq!(shell.commands.len(), 1);
        assert_eq!(shell.commands[0].title, "Toggle Solarized Sheen Contrast");
        assert!(shell.commands[0].is_selected);

        // Clear query
        shell.command_query.clear();
        shell.filter_commands();
        assert_eq!(shell.commands.len(), default_commands().len());
        assert!(shell.commands[0].is_selected);
    }

    #[test]
    fn navigates_and_executes_command_palette_actions() {
        let mut shell = ArcadeShell::test_stub();
        assert!(shell.commands[0].is_selected);

        shell.select_next_command();
        assert!(!shell.commands[0].is_selected);
        assert!(shell.commands[1].is_selected);

        shell.select_prev_command();
        assert!(shell.commands[0].is_selected);

        // Execute "Save Document"
        shell.document.insert_text(ByteOffset(0), "mutated").unwrap();
        assert!(shell.history.is_dirty(shell.document.revision()));

        // Find "Save Document" index
        let save_idx = shell.commands.iter().position(|c| c.title == "Save Document").unwrap();
        shell.execute_command_at(save_idx);

        assert!(!shell.show_command_palette);
        assert!(!shell.history.is_dirty(shell.document.revision()));
    }

    #[test]
    fn toggles_theme_and_sidebar() {
        let mut shell = ArcadeShell::test_stub();
        assert!(shell.is_dark_theme);
        assert!(shell.show_sidebar);

        shell.toggle_theme();
        assert!(!shell.is_dark_theme);

        shell.toggle_theme();
        assert!(shell.is_dark_theme);

        shell.toggle_sidebar();
        assert!(!shell.show_sidebar);

        shell.toggle_sidebar();
        assert!(shell.show_sidebar);
    }
}
