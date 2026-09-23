//! Interactive In-Editor Find & Replace bar and state management.

use gpui::prelude::*;
use gpui::{div, px, Context, Div};

use arcade_core::{Rope, TextRange};
use crate::theme::SolarizedTheme;
use crate::ArcadeShell;

/// Tracks which input field within the Find & Replace widget currently has keyboard focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindFocus {
    /// Keyboard focus is on the query search input field.
    Query,
    /// Keyboard focus is on the replacement input field.
    Replacement,
}

/// Action to be taken by the parent editor upon processing a find/replace keystroke.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindKeyAction {
    /// No action needed.
    None,
    /// Close the Find & Replace bar.
    Close,
    /// Navigate to the next match.
    FindNext,
    /// Navigate to the previous match.
    FindPrev,
    /// Replace the currently active match.
    ReplaceCurrent,
    /// Replace all occurrences across the document.
    ReplaceAll,
    /// Select all occurrences as multi-cursors.
    SelectAllMatches,
    /// Query text changed; re-run matching.
    QueryChanged,
    /// Replacement text changed.
    ReplacementChanged,
}

/// Active state and matching logic for the floating Find & Replace widget.
#[derive(Debug, Clone)]
pub struct FindReplaceState {
    /// Whether the Find & Replace bar is currently visible.
    pub is_open: bool,
    /// Whether the replacement input row is visible.
    pub show_replace: bool,
    /// Active search query string.
    pub query: String,
    /// Active replacement string.
    pub replacement: String,
    /// Pre-computed text match ranges within the document.
    pub matches: Vec<TextRange>,
    /// Index of the currently highlighted match.
    pub active_match_index: usize,
    /// Whether matching is case-sensitive.
    pub case_sensitive: bool,
    /// Which input field currently has keyboard focus.
    pub focus: FindFocus,
}

impl Default for FindReplaceState {
    fn default() -> Self {
        Self::new()
    }
}

impl FindReplaceState {
    /// Creates a new inactive Find & Replace state.
    pub fn new() -> Self {
        Self {
            is_open: false,
            show_replace: false,
            query: String::new(),
            replacement: String::new(),
            matches: Vec::new(),
            active_match_index: 0,
            case_sensitive: false,
            focus: FindFocus::Query,
        }
    }

    /// Opens the Find bar.
    pub fn open_find(&mut self) {
        self.is_open = true;
        self.focus = FindFocus::Query;
    }

    /// Opens the Find & Replace bar with replacement mode enabled.
    pub fn open_replace(&mut self) {
        self.is_open = true;
        self.show_replace = true;
        self.focus = FindFocus::Query;
    }

    /// Closes the Find & Replace bar.
    pub fn close(&mut self) {
        self.is_open = false;
    }

    /// Toggles replace row visibility.
    pub fn toggle_replace(&mut self) {
        self.show_replace = !self.show_replace;
        if self.show_replace {
            self.focus = FindFocus::Replacement;
        } else {
            self.focus = FindFocus::Query;
        }
    }

    /// Recalculates all occurrences of `query` in the provided document rope.
    pub fn update_matches(&mut self, rope: &Rope) {
        self.matches.clear();
        if self.query.is_empty() {
            self.active_match_index = 0;
            return;
        }

        let full_text = rope.to_string();
        let query_len = self.query.len();

        if self.case_sensitive {
            let mut start = 0;
            while let Some(pos) = full_text[start..].find(&self.query) {
                let m_start = start + pos;
                let m_end = m_start + query_len;
                if let Ok(range) = TextRange::new(arcade_core::ByteOffset(m_start), arcade_core::ByteOffset(m_end)) {
                    self.matches.push(range);
                }
                start = m_start + query_len.max(1);
            }
        } else {
            let lower_doc = full_text.to_lowercase();
            let lower_query = self.query.to_lowercase();
            let mut start = 0;
            while let Some(pos) = lower_doc[start..].find(&lower_query) {
                let m_start = start + pos;
                let m_end = m_start + query_len;
                if let Ok(range) = TextRange::new(arcade_core::ByteOffset(m_start), arcade_core::ByteOffset(m_end)) {
                    self.matches.push(range);
                }
                start = m_start + query_len.max(1);
            }
        }

        if self.matches.is_empty() {
            self.active_match_index = 0;
        } else if self.active_match_index >= self.matches.len() {
            self.active_match_index = 0;
        }
    }

    /// Navigates to the next match, wrapping around to the start.
    pub fn next_match(&mut self) -> Option<TextRange> {
        if self.matches.is_empty() {
            return None;
        }
        self.active_match_index = (self.active_match_index + 1) % self.matches.len();
        self.current_match()
    }

    /// Navigates to the previous match, wrapping around to the end.
    pub fn prev_match(&mut self) -> Option<TextRange> {
        if self.matches.is_empty() {
            return None;
        }
        if self.active_match_index == 0 {
            self.active_match_index = self.matches.len() - 1;
        } else {
            self.active_match_index -= 1;
        }
        self.current_match()
    }

    /// Returns the currently active match range, if any.
    pub fn current_match(&self) -> Option<TextRange> {
        self.matches.get(self.active_match_index).copied()
    }

    /// Handles keyboard events routed to the Find & Replace widget.
    pub fn handle_key(
        &mut self,
        key: &str,
        ctrl: bool,
        shift: bool,
        typed_char: Option<&str>,
    ) -> FindKeyAction {
        if key == "escape" {
            self.close();
            return FindKeyAction::Close;
        }

        // Tab: switch focus between Query and Replacement
        if key == "tab" {
            if self.show_replace {
                self.focus = match self.focus {
                    FindFocus::Query => FindFocus::Replacement,
                    FindFocus::Replacement => FindFocus::Query,
                };
            }
            return FindKeyAction::None;
        }

        // Enter: Next / Prev match or Replace
        if key == "enter" {
            match self.focus {
                FindFocus::Query => {
                    if shift {
                        return FindKeyAction::FindPrev;
                    } else {
                        return FindKeyAction::FindNext;
                    }
                }
                FindFocus::Replacement => {
                    if ctrl {
                        return FindKeyAction::ReplaceAll;
                    } else {
                        return FindKeyAction::ReplaceCurrent;
                    }
                }
            }
        }

        // Up / Down arrow keys cycle matches while query is focused
        if key == "up" {
            return FindKeyAction::FindPrev;
        }
        if key == "down" {
            return FindKeyAction::FindNext;
        }

        // Backspace handling
        if key == "backspace" {
            match self.focus {
                FindFocus::Query => {
                    if !self.query.is_empty() {
                        self.query.pop();
                        return FindKeyAction::QueryChanged;
                    }
                }
                FindFocus::Replacement => {
                    if !self.replacement.is_empty() {
                        self.replacement.pop();
                        return FindKeyAction::ReplacementChanged;
                    }
                }
            }
            return FindKeyAction::None;
        }

        // Typing characters
        if let Some(ch) = typed_char {
            match self.focus {
                FindFocus::Query => {
                    self.query.push_str(ch);
                    return FindKeyAction::QueryChanged;
                }
                FindFocus::Replacement => {
                    self.replacement.push_str(ch);
                    return FindKeyAction::ReplacementChanged;
                }
            }
        }

        FindKeyAction::None
    }
}

/// Renders the floating Find & Replace widget at the top-right of the editor canvas.
pub fn render_find_replace_bar(
    theme: &SolarizedTheme,
    state: &FindReplaceState,
    cx: &mut Context<ArcadeShell>,
) -> Div {
    let match_count = state.matches.len();
    let match_display = if state.query.is_empty() {
        "0 of 0".to_string()
    } else if match_count == 0 {
        "No matches".to_string()
    } else {
        format!("{} of {}", state.active_match_index + 1, match_count)
    };

    let is_query_focused = state.focus == FindFocus::Query;
    let is_replace_focused = state.focus == FindFocus::Replacement;
    let show_replace = state.show_replace;
    let case_sensitive = state.case_sensitive;

    div()
        .absolute()
        .top(px(10.0))
        .right(px(24.0))
        .w(px(380.0))
        .rounded_xl()
        .bg(theme.bg_surface_glass)
        .border_1()
        .border_color(theme.border_glass)
        .border_t_1()
        .border_color(theme.border_specular_top)
        .shadow_md()
        .p_2()
        .flex()
        .flex_col()
        .gap_1p5()
        // Row 1: Find Query Row
        .child(
            div()
                .flex()
                .items_center()
                .gap_1p5()
                // Search Input Field
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .items_center()
                        .gap_1p5()
                        .px_2()
                        .py_1()
                        .rounded_lg()
                        .bg(theme.bg_input_glass)
                        .border_1()
                        .border_color(if is_query_focused {
                            theme.syntax_cyan
                        } else {
                            theme.border_subtle
                        })
                        .cursor_pointer()
                        .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this: &mut crate::ArcadeShell, _, _, cx| {
                            this.find_replace.focus = FindFocus::Query;
                            cx.notify();
                        }))
                        .child(
                            div()
                                .text_size(px(11.0))
                                .text_color(theme.syntax_cyan)
                                .child("🔍"),
                        )
                        .child(
                            div()
                                .flex_1()
                                .text_size(px(12.0))
                                .text_color(if state.query.is_empty() {
                                    theme.text_muted
                                } else {
                                    theme.text_bright
                                })
                                .child(if state.query.is_empty() {
                                    "Find in document...".to_string()
                                } else {
                                    state.query.clone()
                                }),
                        )
                        // Match count badge
                        .child(
                            div()
                                .text_size(px(10.0))
                                .text_color(if match_count > 0 {
                                    theme.syntax_green
                                } else if state.query.is_empty() {
                                    theme.text_muted
                                } else {
                                    theme.syntax_yellow
                                })
                                .child(match_display),
                        ),
                )
                // Navigation Prev (↑)
                .child(
                    div()
                        .px_1p5()
                        .py_1()
                        .rounded_md()
                        .bg(theme.badge_bg)
                        .border_1()
                        .border_color(theme.border_subtle)
                        .text_size(px(11.0))
                        .text_color(theme.text_secondary)
                        .cursor_pointer()
                        .hover(|s| s.bg(theme.bg_hover_glass).text_color(theme.text_bright))
                        .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this: &mut crate::ArcadeShell, _, _, cx| {
                            this.find_prev_match();
                            cx.notify();
                        }))
                        .child("↑"),
                )
                // Navigation Next (↓)
                .child(
                    div()
                        .px_1p5()
                        .py_1()
                        .rounded_md()
                        .bg(theme.badge_bg)
                        .border_1()
                        .border_color(theme.border_subtle)
                        .text_size(px(11.0))
                        .text_color(theme.text_secondary)
                        .cursor_pointer()
                        .hover(|s| s.bg(theme.bg_hover_glass).text_color(theme.text_bright))
                        .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this: &mut crate::ArcadeShell, _, _, cx| {
                            this.find_next_match();
                            cx.notify();
                        }))
                        .child("↓"),
                )
                // Find "All" Button (multi-cursor: selects all occurrences!)
                .child(
                    div()
                        .px_2()
                        .py_1()
                        .rounded_md()
                        .bg(theme.badge_bg)
                        .border_1()
                        .border_color(theme.border_subtle)
                        .text_size(px(10.5))
                        .font_weight(gpui::FontWeight::BOLD)
                        .text_color(theme.syntax_cyan)
                        .cursor_pointer()
                        .hover(|s| s.bg(theme.bg_hover_glass).text_color(theme.text_bright))
                        .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this: &mut crate::ArcadeShell, _, _, cx| {
                            this.select_all_find_matches();
                            cx.notify();
                        }))
                        .child("All"),
                )
                // Case Match Toggle (Aa)
                .child(
                    div()
                        .px_1p5()
                        .py_1()
                        .rounded_md()
                        .bg(if case_sensitive {
                            theme.bg_active_glass
                        } else {
                            theme.badge_bg
                        })
                        .border_1()
                        .border_color(if case_sensitive {
                            theme.syntax_cyan
                        } else {
                            theme.border_subtle
                        })
                        .text_size(px(10.5))
                        .font_weight(gpui::FontWeight::BOLD)
                        .text_color(if case_sensitive {
                            theme.syntax_cyan
                        } else {
                            theme.text_muted
                        })
                        .cursor_pointer()
                        .hover(|s| s.bg(theme.bg_hover_glass))
                        .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this: &mut crate::ArcadeShell, _, _, cx| {
                            this.find_replace.case_sensitive = !this.find_replace.case_sensitive;
                            this.find_replace.update_matches(this.document.rope());
                            cx.notify();
                        }))
                        .child("Aa"),
                )
                // Toggle Replace Row (🔁)
                .child(
                    div()
                        .px_1p5()
                        .py_1()
                        .rounded_md()
                        .bg(if show_replace {
                            theme.bg_active_glass
                        } else {
                            theme.badge_bg
                        })
                        .border_1()
                        .border_color(if show_replace {
                            theme.syntax_cyan
                        } else {
                            theme.border_subtle
                        })
                        .text_size(px(11.0))
                        .text_color(theme.text_secondary)
                        .cursor_pointer()
                        .hover(|s| s.bg(theme.bg_hover_glass).text_color(theme.text_bright))
                        .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this: &mut crate::ArcadeShell, _, _, cx| {
                            this.find_replace.toggle_replace();
                            cx.notify();
                        }))
                        .child("🔁"),
                )
                // Close Button (✕)
                .child(
                    div()
                        .px_1p5()
                        .py_1()
                        .rounded_md()
                        .text_size(px(11.0))
                        .text_color(theme.text_muted)
                        .cursor_pointer()
                        .hover(|s| s.bg(theme.bg_hover_glass).text_color(theme.syntax_magenta))
                        .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this: &mut crate::ArcadeShell, _, _, cx| {
                            this.find_replace.close();
                            cx.notify();
                        }))
                        .child("✕"),
                ),
        )
        // Row 2: Replace Row (when show_replace is enabled)
        .when(show_replace, |parent| {
            parent.child(
                div()
                    .flex()
                    .items_center()
                    .gap_1p5()
                    // Replace Input Field
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .items_center()
                            .gap_1p5()
                            .px_2()
                            .py_1()
                            .rounded_lg()
                            .bg(theme.bg_input_glass)
                            .border_1()
                            .border_color(if is_replace_focused {
                                theme.syntax_green
                            } else {
                                theme.border_subtle
                            })
                            .cursor_pointer()
                            .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this: &mut crate::ArcadeShell, _, _, cx| {
                                this.find_replace.focus = FindFocus::Replacement;
                                cx.notify();
                            }))
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(theme.syntax_green)
                                    .child("✏️"),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .text_size(px(12.0))
                                    .text_color(if state.replacement.is_empty() {
                                        theme.text_muted
                                    } else {
                                    theme.text_bright
                                })
                                .child(if state.replacement.is_empty() {
                                    "Replace with...".to_string()
                                } else {
                                    state.replacement.clone()
                                }),
                        ),
                )
                // Replace Current Match
                .child(
                    div()
                        .px_2()
                        .py_1()
                        .rounded_md()
                        .bg(theme.badge_bg)
                        .border_1()
                        .border_color(theme.border_subtle)
                        .text_size(px(11.0))
                        .text_color(theme.syntax_green)
                        .cursor_pointer()
                        .hover(|s| s.bg(theme.bg_hover_glass).text_color(theme.text_bright))
                        .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this: &mut crate::ArcadeShell, _, _, cx| {
                            this.replace_current_find_match();
                            cx.notify();
                        }))
                        .child("Replace"),
                )
                // Replace All Button ("All")
                .child(
                    div()
                        .px_2()
                        .py_1()
                        .rounded_md()
                        .bg(theme.badge_bg)
                        .border_1()
                        .border_color(theme.syntax_green)
                        .text_size(px(11.0))
                        .font_weight(gpui::FontWeight::BOLD)
                        .text_color(theme.syntax_green)
                        .cursor_pointer()
                        .hover(|s| s.bg(theme.bg_hover_glass).text_color(theme.text_bright))
                        .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this: &mut crate::ArcadeShell, _, _, cx| {
                            this.replace_all_find_matches();
                            cx.notify();
                        }))
                        .child("All"),
                ),
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracks_matches_and_cycles() {
        let rope = Rope::from_str("apple banana apple cherry apple");
        let mut state = FindReplaceState::new();

        state.query = "apple".to_string();
        state.update_matches(&rope);
        assert_eq!(state.matches.len(), 3);
        assert_eq!(state.active_match_index, 0);

        let next = state.next_match().expect("Next match");
        assert_eq!(state.active_match_index, 1);
        assert_eq!(next.start.0, 13);

        let next2 = state.next_match().expect("Next match");
        assert_eq!(state.active_match_index, 2);
        assert_eq!(next2.start.0, 26);

        // Wrap around
        let wrap = state.next_match().expect("Wrap match");
        assert_eq!(state.active_match_index, 0);
        assert_eq!(wrap.start.0, 0);

        // Prev match wrap
        let prev = state.prev_match().expect("Prev wrap");
        assert_eq!(state.active_match_index, 2);
        assert_eq!(prev.start.0, 26);
    }

    #[test]
    fn handles_keystrokes_and_focus() {
        let mut state = FindReplaceState::new();
        state.open_find();
        assert!(state.is_open);
        assert_eq!(state.focus, FindFocus::Query);

        // Typing query
        let action = state.handle_key("a", false, false, Some("a"));
        assert_eq!(action, FindKeyAction::QueryChanged);
        assert_eq!(state.query, "a");

        // Open replace and toggle focus
        state.open_replace();
        assert!(state.show_replace);
        let tab_action = state.handle_key("tab", false, false, None);
        assert_eq!(tab_action, FindKeyAction::None);
        assert_eq!(state.focus, FindFocus::Replacement);

        // Typing replacement
        let rep_action = state.handle_key("b", false, false, Some("b"));
        assert_eq!(rep_action, FindKeyAction::ReplacementChanged);
        assert_eq!(state.replacement, "b");

        // Escape closes
        let esc_action = state.handle_key("escape", false, false, None);
        assert_eq!(esc_action, FindKeyAction::Close);
        assert!(!state.is_open);
    }
}
