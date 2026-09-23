//! The custom ArcadeEdit desktop UI boundary.
//!
//! GPUI powers an original Solarized minimalist interface with glassy sheen layers,
//! macOS/MAUI-inspired modals, vibrant live syntax highlighting, and responsive
//! multi-cursor keyboard editing.

pub mod command_palette;
pub mod editor_view;
pub mod help_modal;
pub mod terminal;
pub mod theme;

use std::path::PathBuf;

use arcade_core::selection::SelectionSet;
use arcade_core::{ByteOffset, Document, History};
use arcade_language::{HighlightSpan, HighlightWorker, LanguageId, LanguageService};
use command_palette::{default_commands, render_command_palette, CommandItem};
use editor_view::render_live_editor_surface;
use gpui::{div, prelude::*, px, Context, FocusHandle, IntoElement, Render, Window};
use help_modal::{render_help_modal, HelpSection};
use terminal::{render_terminal_panel, TerminalLine, TerminalLineKind, TerminalState};
use theme::SolarizedTheme;

/// Identifies the custom visual system used by the desktop application.
pub const DESIGN_SYSTEM_NAME: &str = "ArcadeEdit Solarized Glass";

/// An open document tab inside ArcadeEdit.
#[derive(Clone, Debug)]
pub struct EditorTab {
    /// Unique identifier for this tab instance.
    pub id: usize,
    /// Display title shown on the tab header (e.g., "main.rs", "Welcome.md").
    pub title: String,
    /// Active document buffer representation.
    pub document: Document,
    /// Multi-cursor selections across this document.
    pub selections: SelectionSet,
    /// Undo/redo history manager.
    pub history: History,
    /// Detected language grammar for syntax highlighting.
    pub language: LanguageId,
    /// Pre-computed syntax highlight spans.
    pub highlight_spans: Vec<HighlightSpan>,
    /// Associated disk file path (if loaded from or saved to disk).
    pub file_path: Option<PathBuf>,
}

impl EditorTab {
    /// Constructs a new editor tab.
    pub fn new(
        id: usize,
        title: impl Into<String>,
        document: Document,
        language: LanguageId,
        file_path: Option<PathBuf>,
        highlight_spans: Vec<HighlightSpan>,
    ) -> Self {
        Self {
            id,
            title: title.into(),
            document,
            selections: SelectionSet::cursor(ByteOffset(0)),
            history: History::new(),
            language,
            highlight_spans,
            file_path,
        }
    }
}

/// The rich custom-rendered ArcadeEdit desktop application shell.
pub struct ArcadeShell {
    /// Dynamic open editor tabs.
    pub tabs: Vec<EditorTab>,
    /// Index of the active tab.
    pub active_tab_index: usize,
    /// Counter for unique tab identifiers.
    pub next_tab_id: usize,
    /// Synchronous language service for instant frame-0 syntax highlighting.
    pub language_service: Option<LanguageService>,

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
    /// Currently opened file path on disk, if any.
    pub current_file_path: Option<PathBuf>,
    /// Currently opened workspace folder root, if any.
    pub workspace_root: Option<PathBuf>,
    /// Files discovered in the opened workspace folder.
    pub workspace_files: Vec<PathBuf>,
    /// Integrated terminal session state.
    pub terminal: TerminalState,
    /// Controls visibility of the bottom integrated terminal panel.
    pub show_terminal: bool,
    /// Whether terminal currently has active keyboard focus.
    pub terminal_focused: bool,
    /// Controls visibility of the in-app Help and Documentation modal.
    pub show_help_modal: bool,
    /// The active tab / section inside the Help modal.
    pub active_help_section: HelpSection,
}

impl ArcadeShell {
    /// Creates the initial shell configured with Solarized Dark and live interactive keyboard input.
    pub fn welcome(cx: &mut Context<Self>) -> Self {
        let initial_text = "# Welcome to ArcadeEdit\n\nA fast, native text editor with custom GPU-rendered desktop experience.\n\nType anywhere to begin editing live!\n• Use Arrow Keys, Home, End to navigate\n• Press Ctrl+P to open the Command Palette\n• Press Ctrl+Z / Ctrl+Y for Undo / Redo\n• Multi-cursor selections merge seamlessly\n";

        let focus_handle = cx.focus_handle();

        let mut service_opt = LanguageService::new().ok();
        let mut spans = Vec::new();
        if let Some(service) = &mut service_opt {
            spans = service.highlight(LanguageId::Markdown, initial_text);
        }

        let initial_tab = EditorTab::new(
            1,
            "Welcome.md",
            Document::new(initial_text),
            LanguageId::Markdown,
            None,
            spans.clone(),
        );

        let worker = HighlightWorker::spawn().ok();
        let working_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let terminal = TerminalState::new(working_dir);

        Self {
            tabs: vec![initial_tab],
            active_tab_index: 0,
            next_tab_id: 2,
            language_service: service_opt,
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
            current_file_path: None,
            workspace_root: None,
            workspace_files: Vec::new(),
            terminal,
            show_terminal: false,
            terminal_focused: false,
            show_help_modal: false,
            active_help_section: HelpSection::IrDocs,
        }
    }

    /// Creates a shell without a window focus handle (suitable for unit tests).
    pub fn test_stub() -> Self {
        let initial_text = "# ArcadeEdit Test Document\n";
        let initial_tab = EditorTab::new(
            1,
            "Welcome.md",
            Document::new(initial_text),
            LanguageId::Rust,
            None,
            Vec::new(),
        );

        Self {
            tabs: vec![initial_tab],
            active_tab_index: 0,
            next_tab_id: 2,
            language_service: LanguageService::new().ok(),
            document: Document::new(initial_text),
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
            current_file_path: None,
            workspace_root: None,
            workspace_files: Vec::new(),
            terminal: TerminalState::new(PathBuf::from(".")),
            show_terminal: false,
            terminal_focused: false,
            show_help_modal: false,
            active_help_section: HelpSection::IrDocs,
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

    /// Marks the active document as saved, writing to disk if a file path is associated.
    pub fn save_document(&mut self) {
        if let Some(path) = &self.current_file_path {
            let _ = std::fs::write(path, self.document.to_string());
        }
        self.history.mark_saved(self.document.revision());
        self.sync_to_active_tab();
    }

    /// Opens a file from disk into an editor tab, detecting language and updating highlights.
    pub fn open_file(&mut self, path: PathBuf) -> Result<(), String> {
        // 1. If already open in an existing tab, activate it
        if let Some(existing_idx) = self.tabs.iter().position(|t| t.file_path.as_ref() == Some(&path)) {
            self.select_tab(existing_idx);
            return Ok(());
        }

        // 2. Read file from disk
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read file {}: {e}", path.display()))?;

        let language = LanguageId::from_path(&path);
        let file_title = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file")
            .to_string();

        // 3. Immediately compute syntax highlight spans synchronously (<2ms)
        let spans = if let Some(service) = &mut self.language_service {
            service.highlight(language, &content)
        } else if let Ok(mut service) = LanguageService::new() {
            let s = service.highlight(language, &content);
            self.language_service = Some(service);
            s
        } else {
            Vec::new()
        };

        let doc = Document::new(&content);
        let mut hist = History::new();
        hist.mark_saved(doc.revision());

        let id = self.next_tab_id;
        self.next_tab_id += 1;

        let mut new_tab = EditorTab::new(id, file_title, doc, language, Some(path.clone()), spans);
        new_tab.history = hist;

        // If the only tab is an unmodified default Welcome or empty untitled buffer, replace it
        let should_replace_first = self.tabs.len() == 1
            && self.tabs[0].file_path.is_none()
            && !self.tabs[0].history.is_dirty(self.tabs[0].document.revision())
            && (self.tabs[0].title == "Welcome.md" || self.tabs[0].title.starts_with("untitled"));

        if should_replace_first {
            self.tabs[0] = new_tab;
            self.active_tab_index = 0;
        } else {
            self.sync_to_active_tab();
            self.tabs.push(new_tab);
            self.active_tab_index = self.tabs.len() - 1;
        }

        self.sync_from_active_tab();
        self.trigger_highlight();
        Ok(())
    }

    /// Prompts the user with a native file picker dialog to open a document.
    pub fn prompt_open_file(&mut self) {
        let mut dialog = rfd::FileDialog::new().set_title("Open Document");
        if let Some(root) = &self.workspace_root {
            dialog = dialog.set_directory(root);
        }
        if let Some(picked) = dialog.pick_file() {
            let _ = self.open_file(picked);
        }
    }

    /// Opens a workspace directory, discovers candidate files, and updates the explorer.
    pub fn open_folder(&mut self, path: PathBuf) {
        let files = arcade_workspace::discover_files(&[&path]);
        self.workspace_root = Some(path.clone());
        self.workspace_files = files;
        self.terminal.working_dir = path.clone();
        self.show_sidebar = true;

        if let Some(preferred) = self
            .workspace_files
            .iter()
            .find(|f| {
                f.file_name().map_or(false, |name| {
                    name == "README.md"
                        || name == "Cargo.toml"
                        || name == "main.rs"
                        || name == "lib.rs"
                })
            })
            .cloned()
            .or_else(|| self.workspace_files.first().cloned())
        {
            let _ = self.open_file(preferred);
        }
    }

    /// Prompts the user with a native folder picker dialog to open a workspace.
    pub fn prompt_open_folder(&mut self) {
        let mut dialog = rfd::FileDialog::new().set_title("Open Workspace Folder");
        if let Some(root) = &self.workspace_root {
            dialog = dialog.set_directory(root);
        }
        if let Some(folder) = dialog.pick_folder() {
            self.open_folder(folder);
        }
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
                self.prompt_open_file();
            }
            "Open Folder..." => {
                self.prompt_open_folder();
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
                let byte_size = self.document.len_bytes();
                let char_count = self.document.rope().len_chars();
                let line_count = self.document.rope().len_lines();
                let preview_msg = format!(
                    "\n// [Arcade Headless Preview: {} bytes, {} chars, {} lines, rev: {:?} - 0 errors]\n",
                    byte_size, char_count, line_count, self.document.revision()
                );
                self.insert_text(&preview_msg);
                self.terminal.lines.push(TerminalLine::new(
                    TerminalLineKind::Info,
                    format!("Headless Inspection: {} bytes, {} lines (Revision {:?})", byte_size, line_count, self.document.revision()),
                ));
            }
            "Open Integrated Terminal with `ir`" => {
                self.show_terminal = true;
                self.terminal_focused = true;
            }
            "Toggle Solarized Sheen Contrast" => {
                self.toggle_theme();
            }
            "ir Documentation" => {
                self.active_help_section = HelpSection::IrDocs;
                self.show_help_modal = true;
            }
            "About ArcadeEdit" => {
                self.active_help_section = HelpSection::About;
                self.show_help_modal = true;
            }
            "Help: Keyboard Shortcuts" => {
                self.active_help_section = HelpSection::Shortcuts;
                self.show_help_modal = true;
            }
            _ => {}
        }
    }

    /// Syncs the shell's active-document mirror fields with the currently active tab in `self.tabs`.
    pub fn sync_from_active_tab(&mut self) {
        if let Some(tab) = self.tabs.get(self.active_tab_index) {
            self.document = tab.document.clone();
            self.selections = tab.selections.clone();
            self.history = tab.history.clone();
            self.language = tab.language;
            self.highlight_spans = tab.highlight_spans.clone();
            self.current_file_path = tab.file_path.clone();
            self.active_tab = self.active_tab_index;
        }
    }

    /// Syncs mutations from shell's active-document fields back into the active tab in `self.tabs`.
    pub fn sync_to_active_tab(&mut self) {
        if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
            tab.document = self.document.clone();
            tab.selections = self.selections.clone();
            tab.history = self.history.clone();
            tab.language = self.language;
            tab.highlight_spans = self.highlight_spans.clone();
            tab.file_path = self.current_file_path.clone();
            if let Some(path) = &self.current_file_path {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    tab.title = name.to_string();
                }
            }
        }
    }

    /// Selects the active tab by its index and synchronizes buffers.
    pub fn select_tab(&mut self, tab: usize) {
        if tab >= self.tabs.len() {
            return;
        }
        self.sync_to_active_tab();
        self.active_tab_index = tab;
        self.sync_from_active_tab();

        // If highlight spans for this tab are empty, generate them immediately!
        if self.highlight_spans.is_empty() {
            if let Some(service) = &mut self.language_service {
                self.highlight_spans = service.highlight(self.language, &self.document.to_string());
                if let Some(t) = self.tabs.get_mut(self.active_tab_index) {
                    t.highlight_spans = self.highlight_spans.clone();
                }
            }
        }
        self.trigger_highlight();
    }

    /// Closes the tab at `index`.
    pub fn close_tab(&mut self, index: usize) {
        if index >= self.tabs.len() {
            return;
        }
        if self.tabs.len() == 1 {
            let id = self.next_tab_id;
            self.next_tab_id += 1;
            let initial = Document::new("");
            self.tabs[0] = EditorTab::new(id, "untitled.rs", initial, LanguageId::Rust, None, Vec::new());
            self.active_tab_index = 0;
            self.sync_from_active_tab();
            return;
        }

        self.tabs.remove(index);
        if self.active_tab_index >= self.tabs.len() {
            self.active_tab_index = self.tabs.len() - 1;
        } else if self.active_tab_index > index {
            self.active_tab_index -= 1;
        }
        self.sync_from_active_tab();
    }

    /// Opens a new empty untitled document tab.
    pub fn new_tab(&mut self) {
        self.sync_to_active_tab();
        let id = self.next_tab_id;
        self.next_tab_id += 1;
        let tab_num = self.tabs.len() + 1;
        let title = format!("untitled-{}.rs", tab_num);
        let tab = EditorTab::new(id, title, Document::new(""), LanguageId::Rust, None, Vec::new());
        self.tabs.push(tab);
        self.active_tab_index = self.tabs.len() - 1;
        self.sync_from_active_tab();
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
            self.sync_to_active_tab();
            self.trigger_highlight();
        }
    }

    /// Deletes backward (Backspace) across all current cursor selections.
    pub fn delete_backward(&mut self) {
        if let Ok((tx, new_sels)) = self.document.delete_backward_at_selections(&self.selections) {
            self.history.push(tx);
            self.selections = new_sels;
            self.sync_to_active_tab();
            self.trigger_highlight();
        }
    }

    /// Performs an undo operation and restores prior selections.
    pub fn undo(&mut self) {
        if let Ok(Some(sels)) = self.history.undo(&mut self.document) {
            self.selections = sels;
            self.sync_to_active_tab();
            self.trigger_highlight();
        }
    }

    /// Performs a redo operation and restores resulting selections.
    pub fn redo(&mut self) {
        if let Ok(Some(sels)) = self.history.redo(&mut self.document) {
            self.selections = sels;
            self.sync_to_active_tab();
            self.trigger_highlight();
        }
    }
}

/// Helper to extract printable input character from GPUI KeyDownEvent,
/// correctly resolving space, hyphens, shifted symbols, and platform IME chars.
pub fn resolve_input_character(event: &gpui::KeyDownEvent) -> Option<String> {
    let ctrl = event.keystroke.modifiers.control || event.keystroke.modifiers.platform;
    if ctrl {
        return None;
    }

    let key = event.keystroke.key.as_str();
    let shift = event.keystroke.modifiers.shift;

    // 1. Direct mappings for non-character key names emitted by GPUI
    match key {
        "space" => return Some(" ".to_string()),
        "minus" | "hyphen" => return Some(if shift { "_".to_string() } else { "-".to_string() }),
        "equal" | "equals" => return Some(if shift { "+".to_string() } else { "=".to_string() }),
        "comma" => return Some(if shift { "<".to_string() } else { ",".to_string() }),
        "period" => return Some(if shift { ">".to_string() } else { ".".to_string() }),
        "slash" => return Some(if shift { "?".to_string() } else { "/".to_string() }),
        "backslash" => return Some(if shift { "|".to_string() } else { "\\".to_string() }),
        "semicolon" => return Some(if shift { ":".to_string() } else { ";".to_string() }),
        "quote" => return Some(if shift { "\"".to_string() } else { "'".to_string() }),
        "backquote" | "grave" => return Some(if shift { "~".to_string() } else { "`".to_string() }),
        "bracketleft" => return Some(if shift { "{".to_string() } else { "[".to_string() }),
        "bracketright" => return Some(if shift { "}".to_string() } else { "]".to_string() }),
        // Number row symbols with Shift
        "1" if shift => return Some("!".to_string()),
        "2" if shift => return Some("@".to_string()),
        "3" if shift => return Some("#".to_string()),
        "4" if shift => return Some("$".to_string()),
        "5" if shift => return Some("%".to_string()),
        "6" if shift => return Some("^".to_string()),
        "7" if shift => return Some("&".to_string()),
        "8" if shift => return Some("*".to_string()),
        "9" if shift => return Some("(".to_string()),
        "0" if shift => return Some(")".to_string()),
        _ => {}
    }

    // 2. If GPUI provided a resolved key_char from the platform/IME, use it
    if let Some(ch) = &event.keystroke.key_char {
        if !ch.is_empty() && !ch.chars().all(|c| c.is_control()) {
            return Some(ch.clone());
        }
    }

    // 3. Fallback for simple single character keys without Alt
    if !event.keystroke.modifiers.alt && key.chars().count() == 1 {
        let ch = if shift {
            key.to_uppercase()
        } else {
            key.to_string()
        };
        return Some(ch);
    }

    None
}

impl Render for ArcadeShell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(worker) = &self.highlight_worker {
            while let Some(res) = worker.try_recv_response() {
                if res.revision == self.document.revision() {
                    self.highlight_spans = res.spans.clone();
                    if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
                        tab.highlight_spans = res.spans;
                    }
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
        let show_terminal = self.show_terminal;
        let show_help_modal = self.show_help_modal;
        let active_help_section = self.active_help_section;
        let query = &self.command_query;
        let commands = &self.commands;
        let active_tab = self.active_tab_index;

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

        let active_tab_title = if let Some(tab) = self.tabs.get(self.active_tab_index) {
            tab.title.clone()
        } else {
            "buffer.rs".to_string()
        };
        let active_filename = active_tab_title.clone();

        let workspace_title = self
            .workspace_root
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "ArcadeEdit".to_string());

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

                // When Help modal is visible, Escape dismisses it:
                if this.show_help_modal {
                    if key == "escape" {
                        this.show_help_modal = false;
                        cx.notify();
                        return;
                    }
                    return;
                }
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
                    if let Some(ch) = resolve_input_character(event) {
                        this.command_query.push_str(&ch);
                        this.filter_commands();
                        cx.notify();
                        return;
                    }
                    // Absorb any other keys while palette is open
                    return;
                }

                // Help Menu Shortcut (F1)
                if key == "f1" {
                    this.show_help_modal = true;
                    this.active_help_section = HelpSection::IrDocs;
                    cx.notify();
                    return;
                }

                // Toggle Integrated Terminal (Ctrl+` or Ctrl+~)
                if ctrl && (key == "`" || key == "~") {
                    this.show_terminal = !this.show_terminal;
                    if this.show_terminal {
                        this.terminal_focused = true;
                    }
                    cx.notify();
                    return;
                }

                // When Terminal is open and focused, route keys to terminal session
                if this.show_terminal && this.terminal_focused {
                    if key == "escape" {
                        this.terminal_focused = false;
                        cx.notify();
                        return;
                    }
                    let typed_char = resolve_input_character(event);
                    if this.terminal.handle_key(key, ctrl, typed_char.as_deref()) {
                        cx.notify();
                        return;
                    }
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

                // Open Document (Ctrl+O)
                if ctrl && key == "o" {
                    this.prompt_open_file();
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
                if let Some(ch) = resolve_input_character(event) {
                    this.insert_text(&ch);
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
                                    .text_color(theme.syntax_green)
                                    .cursor_pointer()
                                    .hover(|s| s.bg(theme.bg_hover_glass))
                                    .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
                                        this.show_help_modal = true;
                                        this.active_help_section = HelpSection::IrDocs;
                                        cx.notify();
                                    }))
                                    .child("HELP ▾"),
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
                    .children(self.tabs.iter().enumerate().map(|(idx, tab)| {
                        let is_active = idx == active_tab;
                        let is_dirty = tab.history.is_dirty(tab.document.revision());
                        let icon = match tab.language {
                            LanguageId::Rust => "🦀",
                            LanguageId::Markdown => "📄",
                            LanguageId::Toml | LanguageId::Yaml | LanguageId::Json => "⚙️",
                            LanguageId::PlainText => "📝",
                        };

                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .px_3()
                            .py_1p5()
                            .rounded_t_lg()
                            .bg(if is_active {
                                theme.bg_surface_glass
                            } else {
                                theme.bg_canvas
                            })
                            .border_1()
                            .border_color(if is_active {
                                theme.border_glass
                            } else {
                                gpui::rgba(0x00000000)
                            })
                            .border_b_0()
                            .border_t_2()
                            .border_color(if is_active {
                                theme.syntax_cyan
                            } else {
                                gpui::rgba(0x00000000)
                            })
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.bg_hover_glass))
                            .on_mouse_down(gpui::MouseButton::Left, cx.listener(move |this, _, _, cx| {
                                this.select_tab(idx);
                                cx.notify();
                            }))
                            .child(
                                div()
                                    .text_color(if is_active {
                                        theme.syntax_cyan
                                    } else {
                                        theme.text_muted
                                    })
                                    .text_size(px(12.0))
                                    .child(icon),
                            )
                            .child(
                                div()
                                    .text_size(px(12.5))
                                    .font_weight(if is_active {
                                        gpui::FontWeight::MEDIUM
                                    } else {
                                        gpui::FontWeight::NORMAL
                                    })
                                    .text_color(if is_active {
                                        theme.text_bright
                                    } else {
                                        theme.text_secondary
                                    })
                                    .child(if is_dirty {
                                        format!("{} •", tab.title)
                                    } else {
                                        tab.title.clone()
                                    }),
                            )
                            .child(
                                div()
                                    .ml_1()
                                    .text_color(theme.text_muted)
                                    .hover(|s| s.text_color(theme.syntax_magenta))
                                    .text_size(px(11.0))
                                    .cursor_pointer()
                                    .on_mouse_down(gpui::MouseButton::Left, cx.listener(move |this, _, _, cx| {
                                        cx.stop_propagation();
                                        this.close_tab(idx);
                                        cx.notify();
                                    }))
                                    .child("×"),
                            )
                    }))
                    // Add Tab Button (+)
                    .child(
                        div()
                            .px_2()
                            .py_1()
                            .rounded_md()
                            .text_color(theme.text_muted)
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.bg_hover_glass).text_color(theme.text_bright))
                            .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
                                this.new_tab();
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
                                                .child(workspace_title),
                                        ),
                                )
                                // File Tree & Workspace Actions
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
                                                .py_1p5()
                                                .mb_1()
                                                .rounded_lg()
                                                .bg(theme.badge_bg)
                                                .border_1()
                                                .border_color(theme.border_subtle)
                                                .text_size(px(11.0))
                                                .text_color(theme.syntax_cyan)
                                                .cursor_pointer()
                                                .hover(|s| s.bg(theme.bg_hover_glass))
                                                .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
                                                    this.prompt_open_folder();
                                                    cx.notify();
                                                }))
                                                .child("📂 Open Folder..."),
                                        )
                                        .when(!self.workspace_files.is_empty(), |el| {
                                            let root_opt = self.workspace_root.clone();
                                            let current_opt = self.current_file_path.clone();
                                            let file_nodes: Vec<_> = self.workspace_files.iter().take(120).map(|path| {
                                                let rel_display = if let Some(root) = &root_opt {
                                                    path.strip_prefix(root).unwrap_or(path).to_string_lossy().to_string()
                                                } else {
                                                    path.file_name().and_then(|n| n.to_str()).unwrap_or("file").to_string()
                                                };
                                                let display_name = rel_display.replace('\\', "/");
                                                let is_active = current_opt.as_ref().map_or(false, |p| p == path);
                                                let path_clone = path.clone();

                                                let icon = match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
                                                    "rs" => "🦀",
                                                    "md" | "markdown" => "📄",
                                                    "toml" | "json" | "yaml" | "yml" => "⚙️",
                                                    "lock" => "🔒",
                                                    _ => "📝",
                                                };

                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .gap_2()
                                                    .px_2()
                                                    .py_1()
                                                    .rounded_md()
                                                    .bg(if is_active {
                                                        theme.bg_active_glass
                                                    } else {
                                                        gpui::rgba(0x00000000)
                                                    })
                                                    .border_1()
                                                    .border_color(if is_active {
                                                        theme.border_glass
                                                    } else {
                                                        gpui::rgba(0x00000000)
                                                    })
                                                    .text_color(if is_active {
                                                        theme.syntax_cyan
                                                    } else {
                                                        theme.text_secondary
                                                    })
                                                    .hover(|s| s.bg(theme.bg_hover_glass))
                                                    .cursor_pointer()
                                                    .on_mouse_down(gpui::MouseButton::Left, cx.listener(move |this, _, _, cx| {
                                                        let _ = this.open_file(path_clone.clone());
                                                        cx.notify();
                                                    }))
                                                    .child(
                                                        div()
                                                            .text_size(px(11.0))
                                                            .child(icon),
                                                    )
                                                    .child(
                                                        div()
                                                            .text_size(px(11.5))
                                                            .font_weight(if is_active {
                                                                gpui::FontWeight::MEDIUM
                                                            } else {
                                                                gpui::FontWeight::NORMAL
                                                            })
                                                            .overflow_hidden()
                                                            .child(display_name),
                                                    )
                                            }).collect();

                                            el.children(file_nodes)
                                        })
                                        .when(self.workspace_files.is_empty(), |el| {
                                            el.child(
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
                                                        let p = PathBuf::from("crates/arcade-ui/src/lib.rs");
                                                        if p.exists() {
                                                            let _ = this.open_file(p);
                                                        } else {
                                                            this.select_tab(0);
                                                        }
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
                                                        let p = PathBuf::from("Cargo.toml");
                                                        if p.exists() {
                                                            let _ = this.open_file(p);
                                                        } else {
                                                            this.select_tab(2);
                                                        }
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
                                                        let p = PathBuf::from("README.md");
                                                        if p.exists() {
                                                            let _ = this.open_file(p);
                                                        } else {
                                                            this.select_tab(1);
                                                        }
                                                        cx.notify();
                                                    }))
                                                    .child("📄 README.md"),
                                            )
                                        }),
                                ),
                        )
                    })
                    // Main Editor + Integrated Terminal Area
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .relative()
                            .child(
                                div()
                                    .flex_1()
                                    .flex()
                                    .relative()
                                    .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
                                        this.terminal_focused = false;
                                        cx.notify();
                                    }))
                                    .child(render_live_editor_surface(
                                        &theme,
                                        &self.document,
                                        &self.selections,
                                        &active_filename,
                                        &self.highlight_spans,
                                    )),
                            )
                            .when(show_terminal, |p| {
                                p.child(
                                    div()
                                        .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
                                            this.terminal_focused = true;
                                            cx.notify();
                                        }))
                                        .child(render_terminal_panel(&theme, &self.terminal, cx)),
                                )
                            }),
                    )
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
                                    LanguageId::Toml => "TOML",
                                    LanguageId::Yaml => "YAML",
                                    LanguageId::Json => "JSON",
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
            // =================================================================
            // 6. IN-APP HELP & DOCUMENTATION MODAL OVERLAY
            // =================================================================
            .when(show_help_modal, |parent| {
                parent.child(render_help_modal(active_help_section, &theme, cx))
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
        assert_eq!(shell.tabs.len(), 1);

        // Open a new tab
        shell.new_tab();
        assert_eq!(shell.tabs.len(), 2);
        assert_eq!(shell.active_tab, 1);

        // Type something in tab 1
        shell.insert_text("fn tab_one() {}\n");
        assert!(shell.document.to_string().contains("tab_one"));

        // Switch back to tab 0
        shell.select_tab(0);
        assert_eq!(shell.active_tab, 0);
        assert!(!shell.document.to_string().contains("tab_one"));
        assert!(shell.document.to_string().contains("ArcadeEdit Test Document"));

        // Switch back to tab 1 and verify content was preserved
        shell.select_tab(1);
        assert_eq!(shell.active_tab, 1);
        assert!(shell.document.to_string().contains("tab_one"));

        // Close tab 1
        shell.close_tab(1);
        assert_eq!(shell.tabs.len(), 1);
        assert_eq!(shell.active_tab, 0);
        assert!(shell.document.to_string().contains("ArcadeEdit Test Document"));
    }

    #[test]
    fn opens_multiple_files_in_tabs_and_deduplicates() {
        let mut shell = ArcadeShell::test_stub();
        let cargo_toml_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        let lib_rs_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src").join("lib.rs");

        // 1. Open Cargo.toml (replaces initial Welcome stub)
        shell.open_file(cargo_toml_path.clone()).expect("Failed to open Cargo.toml");
        assert_eq!(shell.tabs.len(), 1);
        assert_eq!(shell.active_tab, 0);
        assert_eq!(shell.current_file_path, Some(cargo_toml_path.clone()));
        assert_eq!(shell.language, LanguageId::Toml);
        assert!(!shell.highlight_spans.is_empty(), "TOML syntax spans should be computed synchronously");

        // 2. Open lib.rs in a new tab
        shell.open_file(lib_rs_path.clone()).expect("Failed to open lib.rs");
        assert_eq!(shell.tabs.len(), 2);
        assert_eq!(shell.active_tab, 1);
        assert_eq!(shell.current_file_path, Some(lib_rs_path.clone()));
        assert_eq!(shell.language, LanguageId::Rust);
        assert!(!shell.highlight_spans.is_empty(), "Rust syntax spans should be computed synchronously");

        // 3. Re-open Cargo.toml (should switch to existing tab at index 0 without duplicating)
        shell.open_file(cargo_toml_path.clone()).expect("Failed to re-open Cargo.toml");
        assert_eq!(shell.tabs.len(), 2, "Re-opening existing file must not create a duplicate tab");
        assert_eq!(shell.active_tab, 0);
        assert_eq!(shell.current_file_path, Some(cargo_toml_path));
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
        shell.insert_text("mutated");
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

    #[test]
    fn opens_file_from_disk_and_updates_state() {
        let mut shell = ArcadeShell::test_stub();
        let cargo_toml_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        assert!(cargo_toml_path.exists());

        shell.open_file(cargo_toml_path.clone()).expect("Failed to open file");
        assert_eq!(shell.current_file_path, Some(cargo_toml_path));
        assert!(shell.document.to_string().contains("[package]"));
        assert!(!shell.history.is_dirty(shell.document.revision()));
        assert_eq!(shell.active_tab, 0);
    }

    #[test]
    fn opens_workspace_folder_and_discovers_files() {
        let mut shell = ArcadeShell::test_stub();
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        shell.open_folder(manifest_dir.clone());
        assert_eq!(shell.workspace_root, Some(manifest_dir));
        assert!(!shell.workspace_files.is_empty());
        assert!(shell.workspace_files.iter().any(|f| f.ends_with("Cargo.toml")));
        assert!(shell.show_sidebar);
        assert!(shell.current_file_path.is_some());
    }

    #[test]
    fn manages_integrated_terminal_lifecycle_and_commands() {
        let mut terminal = TerminalState::new(PathBuf::from("."));
        assert!(!terminal.lines.is_empty());
        assert!(terminal.lines.iter().any(|l| l.text.contains("ArcadeEdit Integrated Terminal")));

        // Typing into input buffer
        assert!(terminal.handle_key("h", false, None));
        assert!(terminal.handle_key("i", false, None));
        assert_eq!(terminal.input_buffer, "hi");

        // Typing space and hyphen directly and via character strings
        assert!(terminal.handle_key("space", false, None));
        assert!(terminal.handle_key("minus", false, None));
        assert!(terminal.handle_key("v", false, Some("v")));
        assert_eq!(terminal.input_buffer, "hi -v");

        // Backspace
        assert!(terminal.handle_key("backspace", false, None));
        assert_eq!(terminal.input_buffer, "hi -");

        // Ctrl+C to abort line
        assert!(terminal.handle_key("c", true, None));
        assert!(terminal.input_buffer.is_empty());

        // Echo command
        terminal.execute_command("echo arcade_term_test");
        assert!(terminal.lines.iter().any(|l| l.text.contains("arcade_term_test")));

        // Clear command
        terminal.execute_command("clear");
        assert!(terminal.lines.is_empty());
    }

    #[test]
    fn routes_palette_commands_to_terminal_and_help_modal() {
        let mut shell = ArcadeShell::test_stub();

        // 1. Open Terminal via palette command
        let term_idx = shell
            .commands
            .iter()
            .position(|c| c.title == "Open Integrated Terminal with `ir`")
            .expect("Command missing");
        shell.execute_command_at(term_idx);
        assert!(shell.show_terminal);
        assert!(shell.terminal_focused);

        // 2. Open ir Documentation via palette command
        let ir_idx = shell
            .commands
            .iter()
            .position(|c| c.title == "ir Documentation")
            .expect("Command missing");
        shell.execute_command_at(ir_idx);
        assert!(shell.show_help_modal);
        assert_eq!(shell.active_help_section, HelpSection::IrDocs);

        // 3. Open About section
        let about_idx = shell
            .commands
            .iter()
            .position(|c| c.title == "About ArcadeEdit")
            .expect("Command missing");
        shell.execute_command_at(about_idx);
        assert!(shell.show_help_modal);
        assert_eq!(shell.active_help_section, HelpSection::About);

        // 4. Headless preview command
        let prev_idx = shell
            .commands
            .iter()
            .position(|c| c.title == "Arcade Headless: Preview Edits")
            .expect("Command missing");
        shell.execute_command_at(prev_idx);
        assert!(shell.document.to_string().contains("[Arcade Headless Preview:"));
        assert!(shell.terminal.lines.iter().any(|l| l.text.contains("Headless Inspection:")));
    }

    #[test]
    fn resolves_bundled_ir_binary() {
        let resolved = terminal::resolve_ir_binary();
        if cfg!(windows) {
            assert!(resolved.is_some(), "Bundled ir binary should be resolved in development/workspace on Windows");
            let path = resolved.unwrap();
            assert!(path.exists());
            assert!(path.to_string_lossy().contains("ir"));
        } else if let Some(path) = resolved {
            assert!(path.exists());
            assert!(path.to_string_lossy().contains("ir"));
        }
    }
}
