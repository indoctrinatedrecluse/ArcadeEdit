//! Interactive Integrated Terminal component powered by the pure-Rust `ir` CLI companion utility.
//!
//! Provides in-application command execution, automatic resolution of the bundled `ir` binary
//! without requiring system PATH configuration, command history, and custom Solarized Glass styling.

use crate::theme::SolarizedTheme;
use crate::ArcadeShell;
use gpui::{div, prelude::*, px, Context, IntoElement};
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant};

const MAX_TERMINAL_LINES: usize = 4_000;
const MAX_COMMAND_HISTORY: usize = 500;

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
        "pmon", "ptop", "nettop", "ntop", "monitor", "smon", "dua", "ncdu", "fm", "browse", "edit",
        "ed", "clock", "globe", "life", "matrix", "gitv", "gitinfo", "gin", "dbview", "dbv",
        "request", "req", "hexview", "hexv", "sysinfo", "sys", "watch", "envv", "top", "htop",
        "btop", "vim", "nano", "less",
    ];

    if first_word.eq_ignore_ascii_case("ir") || first_word.eq_ignore_ascii_case("ir.exe") {
        if let Some(sub) = second_word {
            return TUI_COMMANDS.contains(&sub.to_ascii_lowercase().as_str());
        }
    } else {
        return TUI_COMMANDS.contains(&first_word.to_ascii_lowercase().as_str());
    }
    false
}

fn parse_command_words(input: &str) -> Result<Vec<String>, String> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut quote = None;
    let mut started = false;
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if let Some(q) = quote {
            if ch == q {
                quote = None;
            } else if ch == '\\' && q == '"' {
                match chars.peek().copied() {
                    Some(next @ ('"' | '\\')) => {
                        word.push(next);
                        chars.next();
                    }
                    _ => word.push(ch),
                }
            } else {
                word.push(ch);
            }
            started = true;
        } else if ch == '\'' || ch == '"' {
            quote = Some(ch);
            started = true;
        } else if ch == '\\' {
            match chars.peek().copied() {
                Some(next)
                    if next.is_whitespace() || next == '\'' || next == '"' || next == '\\' =>
                {
                    word.push(next);
                    chars.next();
                }
                _ => word.push(ch),
            }
            started = true;
        } else if ch.is_whitespace() {
            if started {
                words.push(std::mem::take(&mut word));
                started = false;
            }
        } else {
            word.push(ch);
            started = true;
        }
    }
    if quote.is_some() {
        return Err("Unclosed quote in command".into());
    }
    if started {
        words.push(word);
    }
    Ok(words)
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
    /// Whether the terminal drawer is currently maximized.
    pub is_maximized: bool,
    last_interrupt: Option<Instant>,
    command_completion_tx: Sender<Result<Output, String>>,
    command_completion_rx: Receiver<Result<Output, String>>,
    pending_command_count: usize,
}

impl TerminalState {
    /// Constructs a new terminal state initialized with welcome messages and `ir` detection.
    pub fn new(working_dir: PathBuf) -> Self {
        let detected_ir = resolve_ir_binary();
        let mut lines = Vec::new();

        lines.push(TerminalLine::new(
            TerminalLineKind::Info,
            format!(
                "✦ ArcadeEdit Integrated Terminal (v{})",
                env!("CARGO_PKG_VERSION")
            ),
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

        let (command_completion_tx, command_completion_rx) = channel();
        Self {
            is_open: false,
            lines,
            input_buffer: String::new(),
            command_history: Vec::new(),
            history_index: None,
            working_dir,
            ir_path: detected_ir,
            active_session: None,
            screen_grid: TerminalScreenGrid::new(120, 20),
            is_maximized: false,
            last_interrupt: None,
            command_completion_tx,
            command_completion_rx,
            pending_command_count: 0,
        }
    }

    /// Toggles between normal docked height and expanded/maximized height.
    pub fn toggle_maximized(&mut self) {
        self.is_maximized = !self.is_maximized;
        let (cols, rows) = if self.is_maximized {
            (120, 30)
        } else {
            (120, 20)
        };
        self.screen_grid.resize(cols, rows);
        if let Some(ref session) = self.active_session {
            session.resize(cols as u16, rows as u16);
        }
    }

    /// Polls the active PTY session, draining output bytes into the screen grid.
    /// Returns true if the screen was updated and needs to be repainted.
    pub fn poll_active_session(&mut self) -> bool {
        self.enforce_buffer_limits();
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
                        self.lines
                            .push(TerminalLine::new(TerminalLineKind::Output, text));
                    }
                }

                self.lines.push(TerminalLine::new(
                    TerminalLineKind::Info,
                    format!("[Process '{}' completed]", session.command_title),
                ));

                self.active_session = None;
                self.last_interrupt = None;
                changed = true;
            }
        }

        while let Ok(completion) = self.command_completion_rx.try_recv() {
            self.append_command_completion(completion);
            self.pending_command_count = self.pending_command_count.saturating_sub(1);
            changed = true;
        }

        changed
    }

    /// Sends an interactive key string or command byte to the active session.
    pub fn send_interactive_key(&mut self, key: &str) {
        if let Some(ref mut session) = self.active_session {
            let _ = session.write_bytes(key.as_bytes());
        }
    }

    /// Returns whether captured terminal commands still have work in progress.
    pub fn has_pending_commands(&self) -> bool {
        self.pending_command_count > 0
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
        self.last_interrupt = None;
    }

    fn enforce_buffer_limits(&mut self) {
        let extra = self.lines.len().saturating_sub(MAX_TERMINAL_LINES);
        if extra > 0 {
            self.lines.drain(..extra);
        }
        let extra = self
            .command_history
            .len()
            .saturating_sub(MAX_COMMAND_HISTORY);
        if extra > 0 {
            self.command_history.drain(..extra);
        }
    }

    fn append_command_completion(&mut self, output: Result<Output, String>) {
        match output {
            Ok(output) => {
                for line in String::from_utf8_lossy(&output.stdout).lines() {
                    self.lines
                        .push(TerminalLine::new(TerminalLineKind::Output, line));
                }
                for line in String::from_utf8_lossy(&output.stderr).lines() {
                    self.lines
                        .push(TerminalLine::new(TerminalLineKind::Error, line));
                }
                if !output.status.success() {
                    self.lines.push(TerminalLine::new(
                        TerminalLineKind::Error,
                        format!(
                            "[Process completed with exit code {:?}]",
                            output.status.code()
                        ),
                    ));
                }
            }
            Err(error) => self
                .lines
                .push(TerminalLine::new(TerminalLineKind::Error, error)),
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
        self.enforce_buffer_limits();
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
        let tokens = match parse_command_words(cmd) {
            Ok(tokens) => tokens,
            Err(error) => {
                self.lines
                    .push(TerminalLine::new(TerminalLineKind::Error, error));
                return;
            }
        };
        let first_word = tokens.first().map(String::as_str).unwrap_or("");
        let second_word = tokens.get(1).map(String::as_str);

        // Check if command invokes an interactive full-screen TUI application
        if is_tui_command(first_word, second_word) {
            let is_ir_call =
                first_word.eq_ignore_ascii_case("ir") || first_word.eq_ignore_ascii_case("ir.exe");
            let (exe_path, args) = if is_ir_call {
                let ir_binary = self.ir_path.clone().or_else(resolve_ir_binary);
                if let Some(bin_path) = ir_binary {
                    if self.ir_path.is_none() {
                        self.ir_path = Some(bin_path.clone());
                    }
                    let args = tokens[1..].to_vec();
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
                    (
                        PathBuf::from("cmd.exe"),
                        vec!["/C".to_string(), cmd.to_string()],
                    )
                }
                #[cfg(not(windows))]
                {
                    (PathBuf::from("sh"), vec!["-c".to_string(), cmd.to_string()])
                }
            };

            let (cols, rows) = if self.is_maximized {
                (120, 30)
            } else {
                (120, 20)
            };
            self.screen_grid.resize(cols, rows);
            self.screen_grid.clear();
            match PtySession::spawn(
                &exe_path,
                &args,
                &self.working_dir,
                cols as u16,
                rows as u16,
            ) {
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

        // Captured commands run off the UI thread. Interactive/full-screen commands above use
        // the PTY path; ordinary CLI commands can take arbitrarily long to finish.
        let is_ir_call =
            first_word.eq_ignore_ascii_case("ir") || first_word.eq_ignore_ascii_case("ir.exe");
        let (executable, args) = if is_ir_call {
            let Some(binary) = self.ir_path.clone().or_else(resolve_ir_binary) else {
                self.lines.push(TerminalLine::new(
                    TerminalLineKind::Error,
                    "Error: Bundled 'ir' executable could not be resolved. Please verify the companion binary is installed.",
                ));
                return;
            };
            self.ir_path = Some(binary.clone());
            (binary, tokens[1..].to_vec())
        } else {
            #[cfg(windows)]
            {
                (PathBuf::from("cmd.exe"), vec!["/C".into(), cmd.into()])
            }
            #[cfg(not(windows))]
            {
                (PathBuf::from("sh"), vec!["-c".into(), cmd.into()])
            }
        };
        let working_dir = self.working_dir.clone();
        let completion_tx = self.command_completion_tx.clone();
        let task = std::thread::Builder::new()
            .name("arcade-terminal-command".into())
            .spawn(move || {
                #[cfg(windows)]
                let result = {
                    let mut command = Command::new(executable);
                    command
                        .args(args)
                        .current_dir(working_dir)
                        .creation_flags(CREATE_NO_WINDOW)
                        .output()
                };
                #[cfg(not(windows))]
                let result = Command::new(executable)
                    .args(args)
                    .current_dir(working_dir)
                    .output();
                let _ = completion_tx
                    .send(result.map_err(|error| format!("Failed to execute command: {error}")));
            });
        match task {
            Ok(_) => self.pending_command_count += 1,
            Err(error) => self.lines.push(TerminalLine::new(
                TerminalLineKind::Error,
                format!("Could not start command worker: {error}"),
            )),
        }
    }

    /// Handles keyboard input routed to the terminal prompt. Returns true if handled.
    pub fn handle_key(
        &mut self,
        key: &str,
        ctrl: bool,
        shift: bool,
        char_input: Option<&str>,
    ) -> bool {
        // 1. When an interactive PTY session is active, route keystrokes directly into its stdin
        if let Some(ref mut session) = self.active_session {
            if ctrl && key == "c" {
                let _ = session.write_bytes(b"\x03");
                let now = Instant::now();
                if self
                    .last_interrupt
                    .is_some_and(|at| now.duration_since(at) <= Duration::from_secs(2))
                {
                    session.kill();
                    self.lines.push(TerminalLine::new(
                        TerminalLineKind::Info,
                        "[Session force-terminated after repeated Ctrl+C]",
                    ));
                    self.active_session = None;
                    self.last_interrupt = None;
                } else {
                    self.last_interrupt = Some(now);
                    self.lines.push(TerminalLine::new(
                        TerminalLineKind::Info,
                        "[Interrupt sent; press Ctrl+C again to force termination]",
                    ));
                }
                return true;
            }

            if key == "escape" {
                let _ = session.write_bytes(b"\x1b");
                return true;
            }

            // Arrow keys encoded as standard VT100 sequences
            let arrow = match key {
                "up" => Some('A'),
                "down" => Some('B'),
                "right" => Some('C'),
                "left" => Some('D'),
                _ => None,
            };
            if let Some(code) = arrow {
                let modifier = match (ctrl, shift) {
                    (true, true) => Some(6),
                    (true, false) => Some(5),
                    (false, true) => Some(2),
                    (false, false) => None,
                };
                let sequence = if let Some(modifier) = modifier {
                    format!("\x1b[1;{modifier}{code}")
                } else {
                    format!("\x1b[{code}")
                };
                let _ = session.write_bytes(sequence.as_bytes());
                return true;
            }
            if key == "home" {
                let sequence = if ctrl {
                    b"\x1b[1;5H".as_slice()
                } else {
                    b"\x1b[H".as_slice()
                };
                let _ = session.write_bytes(sequence);
                return true;
            }
            if key == "end" {
                let sequence = if ctrl {
                    b"\x1b[1;5F".as_slice()
                } else {
                    b"\x1b[F".as_slice()
                };
                let _ = session.write_bytes(sequence);
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

            if key == "delete" {
                let _ = session.write_bytes(b"\x1b[3~");
                return true;
            }

            // Function keys
            match key {
                "f1" => {
                    let _ = session.write_bytes(b"\x1bOP");
                    return true;
                }
                "f2" => {
                    let _ = session.write_bytes(b"\x1bOQ");
                    return true;
                }
                "f3" => {
                    let _ = session.write_bytes(b"\x1bOR");
                    return true;
                }
                "f4" => {
                    let _ = session.write_bytes(b"\x1bOS");
                    return true;
                }
                "f5" => {
                    let _ = session.write_bytes(b"\x1b[15~");
                    return true;
                }
                "f6" => {
                    let _ = session.write_bytes(b"\x1b[17~");
                    return true;
                }
                "f7" => {
                    let _ = session.write_bytes(b"\x1b[18~");
                    return true;
                }
                "f8" => {
                    let _ = session.write_bytes(b"\x1b[19~");
                    return true;
                }
                "f9" => {
                    let _ = session.write_bytes(b"\x1b[20~");
                    return true;
                }
                "f10" => {
                    let _ = session.write_bytes(b"\x1b[21~");
                    return true;
                }
                "f11" => {
                    let _ = session.write_bytes(b"\x1b[23~");
                    return true;
                }
                "f12" => {
                    let _ = session.write_bytes(b"\x1b[24~");
                    return true;
                }
                _ => {}
            }

            // Ctrl + ASCII letter key forwarding (Ctrl+A -> \x01, Ctrl+D -> \x04, etc.)
            if ctrl && key.len() == 1 {
                let b = key.as_bytes()[0];
                if b.is_ascii_alphabetic() {
                    let code = b.to_ascii_lowercase() - b'a' + 1;
                    let _ = session.write_bytes(&[code]);
                    return true;
                }
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

    let panel_height = if terminal.is_maximized {
        px(500.0)
    } else {
        px(330.0)
    };

    div()
        .h(panel_height)
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
                        // Maximize / Restore Terminal button
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
                                .hover(|s| s.bg(theme.bg_hover_glass).text_color(theme.text_bright))
                                .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
                                    this.terminal.toggle_maximized();
                                    cx.notify();
                                }))
                                .child(if terminal.is_maximized { "❐ Restore" } else { "⛶ Maximize" }),
                        )
                        // Close Terminal button
                        .child(
                            div()
                                .ml_1()
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
                    .overflow_y_scroll()
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
        assert!(is_tui_command("IR.EXE", Some("GITINFO")));
        assert!(is_tui_command("ir", Some("monitor")));
        for alias in [
            "ptop", "smon", "ntop", "ncdu", "browse", "ed", "gin", "dbv", "req", "hexv", "sys",
        ] {
            assert!(
                is_tui_command("ir", Some(alias)),
                "missing TUI alias {alias}"
            );
        }

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
    fn parses_quoted_paths_and_windows_paths() {
        assert_eq!(
            parse_command_words(r#"ir edit "C:\Users\me\my file.txt""#).unwrap(),
            vec!["ir", "edit", r#"C:\Users\me\my file.txt"#]
        );
        assert!(parse_command_words("ir edit \"unfinished").is_err());
    }

    #[test]
    fn manages_pty_session_lifecycle_and_key_routing() {
        let mut term = TerminalState::new(PathBuf::from("."));
        assert!(term.active_session.is_none());

        // Test sending key when no session is active (should not panic)
        term.send_interactive_key("q");

        // Test handle_key in interactive mode vs normal mode
        assert!(term.handle_key("a", false, false, None));
        assert_eq!(term.input_buffer, "a");
        assert!(term.handle_key("b", false, false, None));
        assert_eq!(term.input_buffer, "ab");

        // Backspace
        assert!(term.handle_key("backspace", false, false, None));
        assert_eq!(term.input_buffer, "a");

        // Clear input buffer with Ctrl+C
        assert!(term.handle_key("c", true, false, None));
        assert!(term.input_buffer.is_empty());

        // Test toggle_maximized
        assert!(!term.is_maximized);
        term.toggle_maximized();
        assert!(term.is_maximized);
        assert_eq!(term.screen_grid.rows, 30);
        term.toggle_maximized();
        assert!(!term.is_maximized);
        assert_eq!(term.screen_grid.rows, 20);
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
        assert!(
            session_res.is_ok(),
            "PtySession should successfully spawn on host platform: {:?}",
            session_res.err()
        );

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
            if !combined.is_empty()
                && String::from_utf8_lossy(&combined).contains("pty_test_output")
            {
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

    #[cfg(windows)]
    #[test]
    fn bundled_ir_edit_accepts_typing_navigation_and_interrupt_over_pty() {
        let binary = resolve_ir_binary().expect("bundled ir.exe must resolve in this checkout");
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let file = std::env::temp_dir().join(format!("arcade edit {nonce}.txt"));
        std::fs::write(&file, "").unwrap();

        let mut terminal = TerminalState::new(std::env::temp_dir());
        terminal.ir_path = Some(binary);
        terminal.execute_command(&format!("ir edit \"{}\"", file.display()));
        assert!(
            terminal.active_session.is_some(),
            "edit must launch in a PTY"
        );

        std::thread::sleep(Duration::from_millis(250));
        terminal.poll_active_session();
        assert!(terminal.handle_key("a", false, false, Some("a")));
        assert!(terminal.handle_key("left", false, false, None));
        assert!(terminal.handle_key("right", false, false, None));
        std::thread::sleep(Duration::from_millis(300));
        terminal.poll_active_session();
        let rendered = terminal
            .screen_grid
            .rows_data
            .iter()
            .map(|row| row.text())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            rendered.contains('a'),
            "typed text not rendered: {rendered}"
        );

        // First Ctrl+C is delivered as an interrupt; a quick second one is the explicit force quit.
        assert!(terminal.handle_key("c", true, false, None));
        assert!(terminal.active_session.is_some());
        assert!(terminal.last_interrupt.is_some());
        assert!(terminal.handle_key("c", true, false, None));
        assert!(terminal.active_session.is_none());
        let _ = std::fs::remove_file(file);
    }

    #[cfg(windows)]
    #[test]
    fn bundled_ir_monitors_quit_from_keyboard_over_pty() {
        let binary = resolve_ir_binary().expect("bundled ir.exe must resolve in this checkout");
        for (command, key) in [("ir pmon", "q"), ("ir nettop", "escape")] {
            let mut terminal = TerminalState::new(std::env::temp_dir());
            terminal.ir_path = Some(binary.clone());
            terminal.execute_command(command);
            assert!(
                terminal.active_session.is_some(),
                "{command} must launch in a PTY"
            );
            std::thread::sleep(Duration::from_millis(250));
            terminal.poll_active_session();
            terminal.handle_key(key, false, false, Some(if key == "q" { "q" } else { "" }));

            let deadline = Instant::now() + Duration::from_secs(8);
            while terminal.active_session.is_some() && Instant::now() < deadline {
                terminal.poll_active_session();
                std::thread::sleep(Duration::from_millis(25));
            }
            assert!(
                terminal.active_session.is_none(),
                "{command} should exit with {key}"
            );
        }
    }
}
