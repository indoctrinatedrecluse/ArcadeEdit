//! Interactive Integrated Terminal component powered by the pure-Rust `ir` CLI companion utility.
//!
//! Provides in-application command execution, automatic resolution of the bundled `ir` binary
//! without requiring system PATH configuration, command history, and custom Solarized Glass styling.

use crate::theme::SolarizedTheme;
use crate::ArcadeShell;
use gpui::{div, prelude::*, px, Context, IntoElement};
use std::path::PathBuf;
use std::process::Command;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Classification of a line inside the terminal output stream.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TerminalLineKind {
    /// A command input entered by the user.
    Input,
    /// Standard output text produced by a process.
    Output,
    /// Standard error text or execution failure notice.
    Error,
    /// System banner, status notification, or helper tip.
    Info,
}

/// A single rendered line in the terminal console buffer.
#[derive(Clone, Debug)]
pub struct TerminalLine {
    /// Category styling for this line.
    pub kind: TerminalLineKind,
    /// The string payload.
    pub text: String,
}

impl TerminalLine {
    /// Creates a new terminal line.
    pub fn new(kind: TerminalLineKind, text: impl Into<String>) -> Self {
        Self {
            kind,
            text: text.into(),
        }
    }
}

/// Automatically resolves the path to the bundled or installed `ir` executable
/// without requiring the user to add it to their system PATH.
pub fn resolve_ir_binary() -> Option<PathBuf> {
    let exe_name = if cfg!(windows) { "ir.exe" } else { "ir" };

    // 1. Sibling to the current running application executable (Release bundle distribution)
    if let Ok(current) = std::env::current_exe() {
        if let Some(dir) = current.parent() {
            let sibling = dir.join(exe_name);
            if sibling.is_file() {
                return Some(sibling);
            }
            let sibling_bin = dir.join("bin").join(exe_name);
            if sibling_bin.is_file() {
                return Some(sibling_bin);
            }
        }
    }

    // 2. Relative workspace directory (`bin/ir.exe`) during development or direct execution
    let local_bin = PathBuf::from("bin").join(exe_name);
    if local_bin.is_file() {
        return Some(local_bin.canonicalize().unwrap_or(local_bin));
    }

    // 3. Check CARGO_MANIFEST_DIR relative path
    let manifest_bin = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("bin")
        .join(exe_name);
    if manifest_bin.is_file() {
        return Some(manifest_bin.canonicalize().unwrap_or(manifest_bin));
    }

    // 4. Standard known installation directory on Windows
    #[cfg(windows)]
    {
        let standard_c = PathBuf::from("C:\\ir\\ir.exe");
        if standard_c.is_file() {
            return Some(standard_c);
        }
        if let Ok(appdata) = std::env::var("APPDATA") {
            let appdata_ir = PathBuf::from(appdata).join("ir").join("ir.exe");
            if appdata_ir.is_file() {
                return Some(appdata_ir);
            }
        }
    }

    // 5. Fall back to system PATH
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join(exe_name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    None
}

/// State tracking for the integrated terminal drawer and session history.
pub struct TerminalState {
    /// Whether the terminal panel is currently visible.
    pub is_open: bool,
    /// Lines in the terminal output stream.
    pub lines: Vec<TerminalLine>,
    /// Active text being typed at the prompt.
    pub input_buffer: String,
    /// History of executed command strings.
    pub command_history: Vec<String>,
    /// Pointer for cycling through command history with Up/Down arrows.
    pub history_index: Option<usize>,
    /// Execution directory for shell commands.
    pub working_dir: PathBuf,
    /// Path to the detected `ir` binary, if found.
    pub ir_path: Option<PathBuf>,
}

impl TerminalState {
    /// Constructs a new terminal state initialized with welcome messages and `ir` detection.
    pub fn new(working_dir: PathBuf) -> Self {
        let detected_ir = resolve_ir_binary();
        let mut lines = Vec::new();

        lines.push(TerminalLine::new(
            TerminalLineKind::Info,
            "✦ ArcadeEdit Integrated Terminal (v1.1.0)",
        ));
        lines.push(TerminalLine::new(
            TerminalLineKind::Info,
            "Bundled with pure-Rust 'ir' CLI utility by @indoctrinatedrecluse",
        ));

        if let Some(ref path) = detected_ir {
            lines.push(TerminalLine::new(
                TerminalLineKind::Info,
                format!("Resolved 'ir' binary: {}", path.display()),
            ));
        } else {
            lines.push(TerminalLine::new(
                TerminalLineKind::Error,
                "Warning: 'ir' binary not found. Place 'ir.exe' in 'bin/' or install to 'C:\\ir'.",
            ));
        }

        lines.push(TerminalLine::new(
            TerminalLineKind::Info,
            "Type 'ir help' for available actions, or use any standard shell command.\n",
        ));

        Self {
            is_open: false,
            lines,
            input_buffer: String::new(),
            command_history: Vec::new(),
            history_index: None,
            working_dir,
            ir_path: detected_ir,
        }
    }

    /// Clears the terminal output buffer.
    pub fn clear(&mut self) {
        self.lines.clear();
    }

    /// Navigates backward in command history (Up arrow).
    pub fn history_up(&mut self) {
        if self.command_history.is_empty() {
            return;
        }
        let next_idx = match self.history_index {
            None => self.command_history.len().saturating_sub(1),
            Some(i) => i.saturating_sub(1),
        };
        self.history_index = Some(next_idx);
        if let Some(cmd) = self.command_history.get(next_idx) {
            self.input_buffer = cmd.clone();
        }
    }

    /// Navigates forward in command history (Down arrow).
    pub fn history_down(&mut self) {
        if self.command_history.is_empty() {
            return;
        }
        if let Some(i) = self.history_index {
            if i + 1 < self.command_history.len() {
                let next_idx = i + 1;
                self.history_index = Some(next_idx);
                if let Some(cmd) = self.command_history.get(next_idx) {
                    self.input_buffer = cmd.clone();
                }
            } else {
                self.history_index = None;
                self.input_buffer.clear();
            }
        }
    }

    /// Executes a command string, resolving `ir` commands to the bundled executable.
    pub fn execute_command(&mut self, raw: &str) {
        let cmd = raw.trim();
        if cmd.is_empty() {
            return;
        }

        // Record to display history and command recall
        self.lines.push(TerminalLine::new(
            TerminalLineKind::Input,
            format!("ir ❯ {}", cmd),
        ));
        self.command_history.push(cmd.to_string());
        self.history_index = None;
        self.input_buffer.clear();

        // Built-in terminal commands
        if cmd == "clear" || cmd == "cls" {
            self.lines.clear();
            return;
        }
        if cmd == "exit" || cmd == "quit" {
            self.lines.push(TerminalLine::new(
                TerminalLineKind::Info,
                "[Terminal session closed]",
            ));
            self.is_open = false;
            return;
        }

        // Tokenize command tokens (split by whitespace while preserving simple segments)
        let tokens: Vec<&str> = cmd.split_whitespace().collect();
        let first_word = tokens.first().copied().unwrap_or("");

        // Check if command invokes `ir` (either directly as `ir ...` or as an alias if typed directly)
        let is_ir_call = first_word == "ir";

        let result = if is_ir_call {
            let ir_binary = self.ir_path.clone().or_else(resolve_ir_binary);
            if let Some(bin_path) = ir_binary {
                if self.ir_path.is_none() {
                    self.ir_path = Some(bin_path.clone());
                }
                let args = &tokens[1..];
                #[cfg(windows)]
                {
                    Command::new(&bin_path)
                        .args(args)
                        .current_dir(&self.working_dir)
                        .creation_flags(CREATE_NO_WINDOW)
                        .output()
                }
                #[cfg(not(windows))]
                {
                    Command::new(&bin_path)
                        .args(args)
                        .current_dir(&self.working_dir)
                        .output()
                }
            } else {
                self.lines.push(TerminalLine::new(
                    TerminalLineKind::Error,
                    "Error: Bundled 'ir' executable could not be resolved. Please verify bin/ir.exe exists.",
                ));
                return;
            }
        } else {
            // General shell command dispatch
            #[cfg(windows)]
            {
                Command::new("cmd")
                    .args(["/C", cmd])
                    .current_dir(&self.working_dir)
                    .creation_flags(CREATE_NO_WINDOW)
                    .output()
            }
            #[cfg(not(windows))]
            {
                Command::new("sh")
                    .args(["-c", cmd])
                    .current_dir(&self.working_dir)
                    .output()
            }
        };

        match result {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    self.lines.push(TerminalLine::new(TerminalLineKind::Output, line));
                }

                let stderr = String::from_utf8_lossy(&output.stderr);
                for line in stderr.lines() {
                    self.lines.push(TerminalLine::new(TerminalLineKind::Error, line));
                }

                if !output.status.success() {
                    if let Some(code) = output.status.code() {
                        self.lines.push(TerminalLine::new(
                            TerminalLineKind::Error,
                            format!("[Process completed with exit code {}]", code),
                        ));
                    }
                }
            }
            Err(e) => {
                self.lines.push(TerminalLine::new(
                    TerminalLineKind::Error,
                    format!("Failed to execute command: {e}"),
                ));
            }
        }
    }

    /// Handles keyboard input routed to the terminal prompt. Returns true if handled.
    pub fn handle_key(&mut self, key: &str, ctrl: bool, char_input: Option<&str>) -> bool {
        if key == "enter" {
            let to_run = self.input_buffer.clone();
            self.execute_command(&to_run);
            return true;
        }

        if key == "up" {
            self.history_up();
            return true;
        }

        if key == "down" {
            self.history_down();
            return true;
        }

        if key == "backspace" {
            self.input_buffer.pop();
            return true;
        }

        if ctrl && key == "c" {
            self.input_buffer.clear();
            return true;
        }

        if ctrl && key == "l" {
            self.clear();
            return true;
        }

        if let Some(ch) = char_input {
            self.input_buffer.push_str(ch);
            return true;
        }

        // Direct fallbacks for named keys when char_input was not provided
        if key == "space" {
            self.input_buffer.push(' ');
            return true;
        }

        if key == "minus" || key == "hyphen" {
            self.input_buffer.push('-');
            return true;
        }

        if !ctrl && key.chars().count() == 1 {
            self.input_buffer.push_str(key);
            return true;
        }

        false
    }
}

/// Renders the complete integrated terminal surface as a sleek docked bottom panel.
pub fn render_terminal_panel(
    theme: &SolarizedTheme,
    terminal: &TerminalState,
    cx: &mut Context<ArcadeShell>,
) -> impl IntoElement {
    let cwd_display = terminal
        .working_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("workspace");

    let is_ir_resolved = terminal.ir_path.is_some();

    // Render up to the last 500 terminal output lines
    let rendered_lines: Vec<_> = terminal
        .lines
        .iter()
        .rev()
        .take(500)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|line| {
            let (color, prefix) = match line.kind {
                TerminalLineKind::Input => (theme.syntax_green, ""),
                TerminalLineKind::Output => (theme.text_primary, ""),
                TerminalLineKind::Error => (theme.syntax_orange, "⚠️ "),
                TerminalLineKind::Info => (theme.syntax_cyan, ""),
            };

            div()
                .flex()
                .items_start()
                .text_size(px(11.5))
                .font_family("Consolas, 'Cascadia Code', monospace")
                .text_color(color)
                .child(format!("{}{}", prefix, line.text))
        })
        .collect();

    div()
        .h(px(260.0))
        .w_full()
        .bg(theme.bg_canvas)
        .border_t_1()
        .border_color(theme.border_subtle)
        .border_t_1()
        .border_color(theme.border_specular_top)
        .flex()
        .flex_col()
        .shadow_md()
        // =====================================================================
        // 1. TERMINAL HEADER BAR
        // =====================================================================
        .child(
            div()
                .h(px(32.0))
                .flex()
                .items_center()
                .justify_between()
                .px_4()
                .bg(theme.bg_surface_glass)
                .border_b_1()
                .border_color(theme.border_subtle)
                // Left Title & Badges
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_3()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_1p5()
                                .child(
                                    div()
                                        .text_size(px(12.0))
                                        .text_color(theme.syntax_green)
                                        .child("📟"),
                                )
                                .child(
                                    div()
                                        .text_size(px(11.5))
                                        .font_weight(gpui::FontWeight::BOLD)
                                        .text_color(theme.text_bright)
                                        .child("TERMINAL"),
                                ),
                        )
                        .child(
                            div()
                                .px_2()
                                .py_0p5()
                                .rounded_md()
                                .bg(theme.badge_bg)
                                .border_1()
                                .border_color(theme.border_glass)
                                .text_size(px(10.0))
                                .text_color(if is_ir_resolved {
                                    theme.syntax_cyan
                                } else {
                                    theme.syntax_yellow
                                })
                                .child(if is_ir_resolved {
                                    "ir CLI (v3.8) Bundled"
                                } else {
                                    "ir CLI (Not Found)"
                                }),
                        )
                        .child(
                            div()
                                .text_size(px(10.5))
                                .text_color(theme.text_muted)
                                .child(format!("📁 {}", cwd_display)),
                        ),
                )
                // Right Quick Actions
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1p5()
                        // Run `ir help` button
                        .child(
                            div()
                                .px_2()
                                .py_0p5()
                                .rounded_md()
                                .bg(theme.badge_bg)
                                .border_1()
                                .border_color(theme.border_subtle)
                                .text_size(px(10.5))
                                .text_color(theme.syntax_cyan)
                                .cursor_pointer()
                                .hover(|s| s.bg(theme.bg_hover_glass))
                                .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
                                    this.terminal.execute_command("ir help");
                                    cx.notify();
                                }))
                                .child("ir help"),
                        )
                        // Run `ir list` button
                        .child(
                            div()
                                .px_2()
                                .py_0p5()
                                .rounded_md()
                                .bg(theme.badge_bg)
                                .border_1()
                                .border_color(theme.border_subtle)
                                .text_size(px(10.5))
                                .text_color(theme.syntax_yellow)
                                .cursor_pointer()
                                .hover(|s| s.bg(theme.bg_hover_glass))
                                .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
                                    this.terminal.execute_command("ir list");
                                    cx.notify();
                                }))
                                .child("ir list"),
                        )
                        // Clear button
                        .child(
                            div()
                                .px_2()
                                .py_0p5()
                                .rounded_md()
                                .bg(theme.badge_bg)
                                .border_1()
                                .border_color(theme.border_subtle)
                                .text_size(px(10.5))
                                .text_color(theme.text_secondary)
                                .cursor_pointer()
                                .hover(|s| s.bg(theme.bg_hover_glass))
                                .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
                                    this.terminal.clear();
                                    cx.notify();
                                }))
                                .child("Clear"),
                        )
                        // Close Terminal button
                        .child(
                            div()
                                .ml_2()
                                .px_2()
                                .py_0p5()
                                .rounded_md()
                                .text_color(theme.text_muted)
                                .cursor_pointer()
                                .hover(|s| s.text_color(theme.syntax_magenta).bg(theme.bg_hover_glass))
                                .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
                                    this.show_terminal = false;
                                    this.terminal.is_open = false;
                                    this.terminal_focused = false;
                                    cx.notify();
                                }))
                                .child("✕"),
                        ),
                ),
        )
        // =====================================================================
        // 2. TERMINAL OUTPUT LOG SURFACE
        // =====================================================================
        .child(
            div()
                .id("terminal-output-scroll")
                .flex_1()
                .min_h_0()
                .p_3()
                .overflow_y_scroll()
                .flex()
                .flex_col()
                .gap_0p5()
                .children(rendered_lines),
        )
        // =====================================================================
        // 3. INTERACTIVE INPUT PROMPT
        // =====================================================================
        .child(
            div()
                .h(px(32.0))
                .px_3()
                .flex()
                .items_center()
                .gap_2()
                .bg(theme.bg_input_glass)
                .border_t_1()
                .border_color(theme.border_glass)
                .child(
                    div()
                        .text_size(px(12.0))
                        .font_weight(gpui::FontWeight::BOLD)
                        .text_color(theme.syntax_green)
                        .child("ir ❯"),
                )
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .items_center()
                        .font_family("Consolas, 'Cascadia Code', monospace")
                        .text_size(px(12.0))
                        .child(if terminal.input_buffer.is_empty() {
                            div()
                                .flex()
                                .items_center()
                                .gap_1()
                                .child(
                                    div()
                                        .text_color(theme.text_muted)
                                        .child("Type an 'ir' command (e.g. 'ir list', 'ir sort', 'ir help') or shell command..."),
                                )
                                .child(
                                    div()
                                        .text_color(theme.syntax_cyan)
                                        .child("▌"),
                                )
                        } else {
                            div()
                                .flex()
                                .items_center()
                                .child(
                                    div()
                                        .text_color(theme.text_bright)
                                        .child(terminal.input_buffer.clone()),
                                )
                                .child(
                                    div()
                                        .text_color(theme.syntax_cyan)
                                        .child("▌"),
                                )
                        }),
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
                        .text_color(theme.text_muted)
                        .child("Enter ↵"),
                ),
        )
}

