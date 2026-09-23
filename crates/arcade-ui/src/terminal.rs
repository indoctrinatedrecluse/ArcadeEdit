//! Interactive Integrated Terminal component powered by the pure-Rust `ir` CLI companion utility.
//!
//! Provides in-application command execution, automatic resolution of the bundled `ir` binary
//! without requiring system PATH configuration, command history, and custom Solarized Glass styling.

use crate::theme::SolarizedTheme;
use crate::ArcadeShell;
use gpui::{div, prelude::*, px, Context, IntoElement};
use std::path::PathBuf;
use std::process::Command;

pub mod pty_session;
pub mod screen_grid;

pub use pty_session::PtySession;
pub use screen_grid::{GridCell, GridRow, TerminalScreenGrid};

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

/// Returns true if the command invocation targets an interactive TUI application.
pub fn is_tui_command(first_word: &str, second_word: Option<&str>) -> bool {
    const TUI_COMMANDS: &[&str] = &[
        "pmon", "ptop", "nettop", "ntop", "dua", "ncdu", "fm", "browse",
        "edit", "ed", "clock", "globe", "life", "matrix", "gitv", "dbview",
        "dbv", "request", "req", "hexview", "hexv", "sysinfo", "sys",
        "watch", "envv", "top", "htop", "btop", "vim", "nano", "less",
    ];

    if first_word == "ir" {
        if let Some(sub) = second_word {
            return TUI_COMMANDS.contains(&sub);
        }
    } else {
        return TUI_COMMANDS.contains(&first_word);
    }
    false
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
    /// Active interactive PTY session (e.g. running `ir pmon`, `ir nettop`, `ir fm`, etc.).
    pub active_session: Option<PtySession>,
    /// Live 2D virtual screen grid for interactive TUI rendering.
    pub screen_grid: TerminalScreenGrid,
}

impl TerminalState {
    /// Constructs a new terminal state initialized with welcome messages and `ir` detection.
    pub fn new(working_dir: PathBuf) -> Self {
        let detected_ir = resolve_ir_binary();
        let mut lines = Vec::new();

        lines.push(TerminalLine::new(
            TerminalLineKind::Info,
            "✦ ArcadeEdit Integrated Terminal (v1.2.0)",
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
            active_session: None,
            screen_grid: TerminalScreenGrid::new(100, 24),
        }
    }

    /// Polls the active PTY session, draining output bytes into the screen grid.
    /// Returns true if the screen was updated and needs to be repainted.
    pub fn poll_active_session(&mut self) -> bool {
        let mut changed = false;

        if let Some(ref mut session) = self.active_session {
            let chunks = session.try_recv();
            for chunk in chunks {
                if !chunk.is_empty() {
                    changed = true;
                    // Auto-respond to ANSI DSR cursor position request (\x1b[6n) if sent by host ConPTY
                    if chunk.windows(4).any(|w| w == b"\x1b[6n") {
                        let _ = session.write_bytes(
                            format!(
                                "\x1b[{};{}R",
                                self.screen_grid.cursor_row + 1,
                                self.screen_grid.cursor_col + 1
                            )
                            .as_bytes(),
                        );
                    }
                    self.screen_grid.advance_bytes(&chunk);
                }
            }

            if !session.is_alive() {
                // Drain any final chunks
                let final_chunks = session.try_recv();
                for chunk in final_chunks {
                    if !chunk.is_empty() {
                        self.screen_grid.advance_bytes(&chunk);
                    }
                }

                // Transfer non-empty lines from screen grid into history
                for row in &self.screen_grid.rows_data {
                    let text = row.text();
                    if !text.is_empty() {
                        self.lines.push(TerminalLine::new(TerminalLineKind::Output, text));
                    }
                }

                self.lines.push(TerminalLine::new(
                    TerminalLineKind::Info,
                    format!("[Process '{}' completed]", session.command_title),
                ));

                self.active_session = None;
                changed = true;
            }
        }

        changed
    }

    /// Sends an interactive key string or command byte to the active session.
    pub fn send_interactive_key(&mut self, key: &str) {
        if let Some(ref mut session) = self.active_session {
            let _ = session.write_bytes(key.as_bytes());
        }
    }

    /// Terminates any currently running active interactive session.
    pub fn terminate_active_session(&mut self) {
        if let Some(ref mut session) = self.active_session {
            session.kill();
            self.lines.push(TerminalLine::new(
                TerminalLineKind::Info,
                format!("[Process '{}' terminated by user]", session.command_title),
            ));
        }
        self.active_session = None;
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
        let second_word = tokens.get(1).copied();

        // Check if command invokes an interactive full-screen TUI application
        if is_tui_command(first_word, second_word) {
            let is_ir_call = first_word == "ir";
            let (exe_path, args) = if is_ir_call {
                let ir_binary = self.ir_path.clone().or_else(resolve_ir_binary);
                if let Some(bin_path) = ir_binary {
                    if self.ir_path.is_none() {
                        self.ir_path = Some(bin_path.clone());
                    }
                    let args = tokens[1..].iter().map(|s| s.to_string()).collect();
                    (bin_path, args)
                } else {
                    self.lines.push(TerminalLine::new(
                        TerminalLineKind::Error,
                        "Error: Bundled 'ir' executable could not be resolved. Please verify bin/ir.exe exists.",
                    ));
                    return;
                }
            } else {
                #[cfg(windows)]
                {
                    (PathBuf::from("cmd.exe"), vec!["/C".to_string(), cmd.to_string()])
                }
                #[cfg(not(windows))]
                {
                    (PathBuf::from("sh"), vec!["-c".to_string(), cmd.to_string()])
                }
            };

            self.screen_grid.clear();
            match PtySession::spawn(&exe_path, &args, &self.working_dir, 100, 24) {
                Ok(session) => {
                    self.active_session = Some(session);
                }
                Err(e) => {
                    self.lines.push(TerminalLine::new(
                        TerminalLineKind::Error,
                        format!("Failed to spawn interactive PTY session: {e}"),
                    ));
                }
            }
            return;
        }

        // Standard batch shell / ir command dispatch
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
        // 1. When an interactive PTY session is active, route keystrokes directly into its stdin
        if let Some(ref mut session) = self.active_session {
            if ctrl && key == "c" {
                let _ = session.write_bytes(b"\x03");
                session.kill();
                self.lines.push(TerminalLine::new(
                    TerminalLineKind::Info,
                    "[Session terminated by Ctrl+C]",
                ));
                self.active_session = None;
                return true;
            }

            if key == "escape" {
                let _ = session.write_bytes(b"\x1b");
                return true;
            }

            if !ctrl && key == "q" {
                let _ = session.write_bytes(b"q");
                return true;
            }

            // Arrow keys encoded as standard VT100 sequences
            if key == "up" {
                let _ = session.write_bytes(b"\x1b[A");
                return true;
            }
            if key == "down" {
                let _ = session.write_bytes(b"\x1b[B");
                return true;
            }
            if key == "right" {
                let _ = session.write_bytes(b"\x1b[C");
                return true;
            }
            if key == "left" {
                let _ = session.write_bytes(b"\x1b[D");
                return true;
            }
            if key == "home" {
                let _ = session.write_bytes(b"\x1b[H");
                return true;
            }
            if key == "end" {
                let _ = session.write_bytes(b"\x1b[F");
                return true;
            }
            if key == "pageup" {
                let _ = session.write_bytes(b"\x1b[5~");
                return true;
            }
            if key == "pagedown" {
                let _ = session.write_bytes(b"\x1b[6~");
                return true;
            }
            if key == "enter" {
                let _ = session.write_bytes(b"\r");
                return true;
            }
            if key == "backspace" {
                let _ = session.write_bytes(b"\x08");
                return true;
            }
            if key == "tab" {
                let _ = session.write_bytes(b"\t");
                return true;
            }
            if key == "space" {
                let _ = session.write_bytes(b" ");
                return true;
            }

            if let Some(ch) = char_input {
                let _ = session.write_bytes(ch.as_bytes());
                return true;
            }

            if !ctrl && key.chars().count() == 1 {
                let _ = session.write_bytes(key.as_bytes());
                return true;
            }

            return false;
        }

        // 2. Normal command prompt input handling
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

    // Render up to the last 500 terminal output lines for scrollback
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
                                        .child(if terminal.active_session.is_some() {
                                            "INTERACTIVE TUI"
                                        } else {
                                            "TERMINAL"
                                        }),
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
                                .text_color(if terminal.active_session.is_some() {
                                    theme.syntax_green
                                } else if is_ir_resolved {
                                    theme.syntax_cyan
                                } else {
                                    theme.syntax_yellow
                                })
                                .child(if let Some(session) = &terminal.active_session {
                                    format!("● LIVE: {}", session.command_title)
                                } else if is_ir_resolved {
                                    "ir CLI (v3.8) Bundled".to_string()
                                } else {
                                    "ir CLI (Not Found)".to_string()
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
                        .when_some(terminal.active_session.as_ref(), |el, _session| {
                            el.child(
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
                                        this.terminal.send_interactive_key("q");
                                        cx.notify();
                                    }))
                                    .child("Quit [q]"),
                            )
                            .child(
                                div()
                                    .px_2()
                                    .py_0p5()
                                    .rounded_md()
                                    .bg(theme.badge_bg)
                                    .border_1()
                                    .border_color(theme.border_subtle)
                                    .text_size(px(10.5))
                                    .text_color(theme.warning_red)
                                    .cursor_pointer()
                                    .hover(|s| s.bg(theme.bg_hover_glass))
                                    .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
                                        this.terminal.terminate_active_session();
                                        cx.notify();
                                    }))
                                    .child("Stop [Ctrl+C]"),
                            )
                        })
                        .when(terminal.active_session.is_none(), |el| {
                            el.child(
                                // Run `ir pmon` quick button
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
                                        this.terminal.execute_command("ir pmon");
                                        cx.notify();
                                    }))
                                    .child("ir pmon"),
                            )
                            .child(
                                // Run `ir nettop` quick button
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
                                        this.terminal.execute_command("ir nettop");
                                        cx.notify();
                                    }))
                                    .child("ir nettop"),
                            )
                            .child(
                                // Run `ir help` button
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
                                        this.terminal.execute_command("ir help");
                                        cx.notify();
                                    }))
                                    .child("ir help"),
                            )
                            .child(
                                // Clear button
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
                        })
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
                                    this.terminal.terminate_active_session();
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
        // 2. TERMINAL OUTPUT LOG SURFACE / INTERACTIVE SCREEN GRID
        // =====================================================================
        .child(
            if terminal.active_session.is_some() {
                div()
                    .id("terminal-grid-surface")
                    .flex_1()
                    .min_h_0()
                    .px_4()
                    .py_2()
                    .overflow_hidden()
                    .bg(theme.bg_canvas)
                    .flex()
                    .flex_col()
                    .children(terminal.screen_grid.render_rows(theme))
            } else {
                div()
                    .id("terminal-output-scroll")
                    .flex_1()
                    .min_h_0()
                    .p_3()
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .children(rendered_lines)
            }
        )
        // =====================================================================
        // 3. INTERACTIVE INPUT PROMPT / TUI CONTROLS FOOTER
        // =====================================================================
        .child(
            if let Some(session) = &terminal.active_session {
                div()
                    .h(px(32.0))
                    .px_3()
                    .flex()
                    .items_center()
                    .justify_between()
                    .bg(theme.bg_input_glass)
                    .border_t_1()
                    .border_color(theme.border_glass)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .text_size(px(11.5))
                            .text_color(theme.syntax_cyan)
                            .child("⌨️ Interactive TUI Active:")
                            .child(
                                div()
                                    .text_color(theme.text_secondary)
                                    .child("Press 'q' or 'Esc' to exit, arrow keys / hotkeys navigate directly inside."),
                            ),
                    )
                    .child(
                        div()
                            .px_2()
                            .py_0p5()
                            .rounded_md()
                            .bg(theme.bg_active_glass)
                            .border_1()
                            .border_color(theme.border_glass)
                            .text_size(px(10.0))
                            .text_color(theme.syntax_green)
                            .child(format!("PID Live ({})", session.command_title)),
                    )
            } else {
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
                                            .child("Type an 'ir' command (e.g. 'ir pmon', 'ir nettop', 'ir list') or shell command..."),
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
                    )
            }
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn detects_interactive_tui_commands_accurately() {
        assert!(is_tui_command("ir", Some("pmon")));
        assert!(is_tui_command("pmon", None));
        assert!(is_tui_command("top", None));
        assert!(is_tui_command("ir", Some("top")));
        assert!(is_tui_command("ir", Some("htop")));
        assert!(is_tui_command("ir", Some("nettop")));
        assert!(is_tui_command("ir", Some("ntop")));
        assert!(is_tui_command("ir", Some("dua")));
        assert!(is_tui_command("ir", Some("ncdu")));
        assert!(is_tui_command("ir", Some("fm")));
        assert!(is_tui_command("ir", Some("edit")));
        assert!(is_tui_command("ir", Some("clock")));
        assert!(is_tui_command("ir", Some("matrix")));
        assert!(is_tui_command("ir", Some("gitv")));
        assert!(is_tui_command("ir", Some("sysinfo")));
        assert!(is_tui_command("ir", Some("watch")));
        assert!(is_tui_command("ir", Some("envv")));

        // Non-TUI standard CLI commands should NOT be treated as TUI
        assert!(!is_tui_command("ir", Some("list")));
        assert!(!is_tui_command("ir", Some("help")));
        assert!(!is_tui_command("ir", Some("version")));
        assert!(!is_tui_command("ir", Some("uuid")));
        assert!(!is_tui_command("ir", Some("base64")));
        assert!(!is_tui_command("ir", Some("hash")));
        assert!(!is_tui_command("echo", None));
        assert!(!is_tui_command("calc", None));
    }

    #[test]
    fn manages_pty_session_lifecycle_and_key_routing() {
        let mut term = TerminalState::new(PathBuf::from("."));
        assert!(term.active_session.is_none());

        // Test sending key when no session is active (should not panic)
        term.send_interactive_key("q");

        // Test handle_key in interactive mode vs normal mode
        assert!(term.handle_key("a", false, None));
        assert_eq!(term.input_buffer, "a");
        assert!(term.handle_key("b", false, None));
        assert_eq!(term.input_buffer, "ab");

        // Backspace
        assert!(term.handle_key("backspace", false, None));
        assert_eq!(term.input_buffer, "a");

        // Clear input buffer with Ctrl+C
        assert!(term.handle_key("c", true, None));
        assert!(term.input_buffer.is_empty());
    }

    #[test]
    fn allocates_and_runs_native_pty_session() {
        #[cfg(target_os = "windows")]
        let cmd = std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string());
        #[cfg(target_os = "windows")]
        let args = vec!["/C".to_string(), "echo pty_test_output".to_string()];

        #[cfg(not(target_os = "windows"))]
        let (cmd, args) = ("echo".to_string(), vec!["pty_test_output".to_string()]);

        let session_res = PtySession::spawn(Path::new(&cmd), &args, Path::new("."), 80, 24);
        assert!(session_res.is_ok(), "PtySession should successfully spawn on host platform: {:?}", session_res.err());

        let mut session = session_res.unwrap();
        assert!(!session.command_title.is_empty());

        // Wait up to 3 seconds for output chunks to arrive from PTY background reader
        let start = std::time::Instant::now();
        let mut combined = Vec::new();
        while start.elapsed() < std::time::Duration::from_secs(3) {
            let chunks = session.try_recv();
            for c in chunks {
                if c.windows(4).any(|w| w == b"\x1b[6n") {
                    let _ = session.write_bytes(b"\x1b[1;1R");
                }
                combined.extend(c);
            }
            if !combined.is_empty() && String::from_utf8_lossy(&combined).contains("pty_test_output") {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        let text = String::from_utf8_lossy(&combined);
        assert!(
            text.contains("pty_test_output"),
            "Expected PTY output to contain 'pty_test_output', got: {}",
            text
        );
    }
}


