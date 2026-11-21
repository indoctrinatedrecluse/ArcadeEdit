//! The custom ArcadeEdit desktop UI boundary.
//!
//! GPUI powers an original Solarized minimalist interface with glassy sheen layers,
//! macOS/MAUI-inspired modals, vibrant live syntax highlighting, and responsive
//! multi-cursor keyboard editing.

pub mod command_palette;
pub mod editor_view;
pub mod find_replace;
pub mod help_modal;
pub mod terminal;
pub mod theme;
pub mod tree_view;
pub mod updater;

use std::collections::BTreeSet;
use std::path::PathBuf;

use arcade_core::selection::SelectionSet;
use arcade_core::{
    has_external_change, load_from_file, save_to_file, ByteOffset, Document, FileMetadata, History,
};
use arcade_language::{HighlightSpan, HighlightWorker, LanguageId, LanguageService};
use command_palette::{default_commands, render_command_palette, CommandItem};
use editor_view::render_live_editor_surface;
use find_replace::{render_find_replace_bar, FindFocus, FindKeyAction, FindReplaceState};
use gpui::{
    div, prelude::*, px, ClipboardItem, Context, FocusHandle, IntoElement, Render, ScrollHandle,
    Window,
};
use help_modal::{render_help_modal, HelpSection};
use terminal::{render_terminal_panel, TerminalLine, TerminalLineKind, TerminalState};
use theme::SolarizedTheme;
use tree_view::build_visible_tree;
use updater::UpdateState;

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
    /// File metadata captured on open/save for detecting external edits.
    pub file_metadata: Option<FileMetadata>,
    /// Newline sequence found in the document, defaulting to LF for new files.
    pub line_ending: String,
    /// Human-readable newline profile for status display.
    pub line_ending_label: String,
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
        let text = document.to_string();
        let (line_ending, line_ending_label) = detect_line_ending(&text);
        Self {
            id,
            title: title.into(),
            document,
            selections: SelectionSet::cursor(ByteOffset(0)),
            history: History::new(),
            language,
            highlight_spans,
            file_path,
            file_metadata: None,
            line_ending,
            line_ending_label,
        }
    }
}

fn detect_line_ending(text: &str) -> (String, String) {
    let bytes = text.as_bytes();
    let (mut has_crlf, mut has_lf, mut first_ending) = (false, false, None);
    for (index, byte) in bytes.iter().enumerate() {
        if *byte == b'\n' {
            let is_crlf = index > 0 && bytes[index - 1] == b'\r';
            has_crlf |= is_crlf;
            has_lf |= !is_crlf;
            first_ending.get_or_insert(if is_crlf { "\r\n" } else { "\n" });
        }
    }
    let line_ending = first_ending.unwrap_or("\n").to_string();
    let label = match (has_crlf, has_lf) {
        (true, true) => "Mixed (CRLF + LF)",
        (true, false) => "CRLF",
        (false, true) => "LF",
        (false, false) => "LF (default)",
    };
    (line_ending, label.to_string())
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
    /// Shows a read-only preview of the current document.
    pub show_preview_modal: bool,
    /// Shows the UI-consistent updater modal.
    pub show_updater_modal: bool,
    /// Shows the Go to Line dialog.
    pub show_goto_line_modal: bool,
    /// Text entered into the Go to Line dialog.
    pub goto_line_input: String,
    /// Latest automatic or manual update-check state.
    pub update_state: UpdateState,
    /// Token used to ignore stale concurrent update-check responses.
    pub update_check_generation: u64,
    /// Defers context-bound update checks requested through context-free commands.
    pub pending_update_check: bool,
    /// The active tab / section inside the Help modal.
    pub active_help_section: HelpSection,
    /// Floating Find & Replace widget state.
    pub find_replace: FindReplaceState,
    /// Set of expanded folder paths in the workspace sidebar.
    pub expanded_folders: BTreeSet<PathBuf>,
    /// Scroll handle controlling the main editor canvas viewport.
    pub editor_scroll_handle: ScrollHandle,
    /// Tab awaiting a save/discard decision before closing.
    pub pending_close_tab: Option<usize>,
    /// Application shutdown is waiting on a dirty-document decision.
    pub pending_window_close: bool,
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

        let mut shell = Self {
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
            show_command_palette: false,
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
            show_preview_modal: false,
            show_updater_modal: false,
            show_goto_line_modal: false,
            goto_line_input: String::new(),
            update_state: UpdateState::Idle,
            update_check_generation: 0,
            pending_update_check: false,
            active_help_section: HelpSection::IrDocs,
            find_replace: FindReplaceState::new(),
            expanded_folders: BTreeSet::new(),
            editor_scroll_handle: ScrollHandle::new(),
            pending_close_tab: None,
            pending_window_close: false,
        };
        shell.check_for_updates(cx, false);
        shell
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
            show_command_palette: false,
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
            show_preview_modal: false,
            show_updater_modal: false,
            show_goto_line_modal: false,
            goto_line_input: String::new(),
            update_state: UpdateState::Idle,
            update_check_generation: 0,
            pending_update_check: false,
            active_help_section: HelpSection::IrDocs,
            find_replace: FindReplaceState::new(),
            expanded_folders: BTreeSet::new(),
            editor_scroll_handle: ScrollHandle::new(),
            pending_close_tab: None,
            pending_window_close: false,
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
        if self.current_file_path.is_none() {
            self.save_document_as();
            return;
        }
        self.save_to_path(self.current_file_path.clone().expect("checked above"));
    }

    /// Prompts for a new file path and saves the active document there.
    pub fn save_document_as(&mut self) {
        let mut dialog = rfd::FileDialog::new().set_title("Save Document As");
        if let Some(root) = &self.workspace_root {
            dialog = dialog.set_directory(root);
        }
        let Some(path) = dialog.save_file() else {
            return;
        };
        self.save_to_path(path);
    }

    fn save_to_path(&mut self, path: PathBuf) {
        if self.current_file_path.as_ref() == Some(&path) {
            if let Some(metadata) = self
                .tabs
                .get(self.active_tab_index)
                .and_then(|tab| tab.file_metadata.as_ref())
            {
                if has_external_change(&path, metadata) {
                    self.report_terminal_error(format!(
                        "{} changed on disk; reload it or use Save As to avoid overwriting external edits.",
                        path.display()
                    ));
                    return;
                }
            }
        }

        match save_to_file(&self.document, &path) {
            Ok(metadata) => {
                self.current_file_path = Some(path);
                if let Some(tab) = self.tabs.get_mut(self.active_tab_index) {
                    tab.file_path = self.current_file_path.clone();
                    tab.file_metadata = Some(metadata);
                    (tab.line_ending, tab.line_ending_label) =
                        detect_line_ending(&self.document.to_string());
                }
                self.history.mark_saved(self.document.revision());
                self.sync_to_active_tab();
            }
            Err(error) => self.report_terminal_error(format!("Save failed: {error}")),
        }
    }

    fn report_terminal_error(&mut self, message: String) {
        self.terminal
            .lines
            .push(TerminalLine::new(TerminalLineKind::Error, message));
        self.show_terminal = true;
        self.terminal.is_open = true;
    }

    fn save_all_and_quit(&mut self, cx: &mut Context<Self>) {
        for index in 0..self.tabs.len() {
            if !self.tabs[index]
                .history
                .is_dirty(self.tabs[index].document.revision())
            {
                continue;
            }
            let path = self.tabs[index].file_path.clone().or_else(|| {
                rfd::FileDialog::new()
                    .set_title("Save Document Before Closing")
                    .save_file()
            });
            let Some(path) = path else {
                return;
            };
            if self.tabs[index].file_path.as_ref() == Some(&path)
                && self.tabs[index]
                    .file_metadata
                    .as_ref()
                    .is_some_and(|metadata| has_external_change(&path, metadata))
            {
                self.report_terminal_error(format!(
                    "{} changed on disk; closing was cancelled to protect the external edit.",
                    path.display()
                ));
                self.pending_window_close = false;
                return;
            }
            match save_to_file(&self.tabs[index].document, &path) {
                Ok(metadata) => {
                    let tab = &mut self.tabs[index];
                    tab.title = path
                        .file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "Untitled".to_string());
                    tab.file_path = Some(path);
                    tab.file_metadata = Some(metadata);
                    tab.history.mark_saved(tab.document.revision());
                }
                Err(error) => {
                    self.report_terminal_error(format!("Save before close failed: {error}"));
                    self.pending_window_close = false;
                    return;
                }
            }
        }
        self.pending_window_close = false;
        cx.quit();
    }

    /// Checks for a newer published release. The desktop startup calls this automatically.
    pub fn check_for_updates(&mut self, cx: &mut Context<Self>, show_result: bool) {
        self.update_check_generation = self.update_check_generation.wrapping_add(1);
        let generation = self.update_check_generation;
        self.update_state = UpdateState::Checking;
        if show_result {
            self.show_updater_modal = true;
        }
        let client = cx.http_client();
        cx.spawn(async move |this, cx| {
            let result = updater::check_for_update(&client, env!("CARGO_PKG_VERSION")).await;
            let _ = this.update(cx, |this, cx| {
                if this.update_check_generation != generation {
                    return;
                }
                this.update_state = match result {
                    Ok(Some(info)) => {
                        if !show_result {
                            this.show_updater_modal = true;
                        }
                        UpdateState::Available(info)
                    }
                    Ok(None) => UpdateState::UpToDate,
                    Err(error) => UpdateState::Failed(error),
                };
                cx.notify();
            });
        })
        .detach();
    }

    fn start_pending_update_check(&mut self, cx: &mut Context<Self>) {
        if std::mem::take(&mut self.pending_update_check) {
            self.check_for_updates(cx, true);
        }
    }

    /// Downloads the verified release asset, stages it beside the app, and restarts on Windows.
    pub fn install_update(&mut self, cx: &mut Context<Self>) {
        let UpdateState::Available(info) = self.update_state.clone() else {
            return;
        };
        #[cfg(not(windows))]
        {
            self.update_state = UpdateState::Failed(
                "Automatic installation is currently available for Windows desktop releases only."
                    .into(),
            );
            cx.notify();
            return;
        }

        #[cfg(windows)]
        {
            let Ok(executable) = std::env::current_exe() else {
                self.update_state = UpdateState::Failed(
                    "Could not locate the running ArcadeEdit executable.".into(),
                );
                cx.notify();
                return;
            };
            let client = cx.http_client();
            self.update_state = UpdateState::Downloading(info.version.clone());
            let version = info.version.clone();
            cx.notify();
            cx.spawn(async move |this, cx| {
                let result = match updater::download_and_stage(&client, &info, &executable).await {
                    Ok(staged) => {
                        updater::launch_installer(&staged, &executable).map(|()| version.clone())
                    }
                    Err(error) => Err(error),
                };
                let _ = this.update(cx, |this, cx| match result {
                    Ok(version) => {
                        this.update_state = UpdateState::Restarting(version);
                        this.show_updater_modal = true;
                        cx.notify();
                        cx.quit();
                    }
                    Err(error) => {
                        this.update_state = UpdateState::Failed(error);
                        this.show_updater_modal = true;
                        cx.notify();
                    }
                });
            })
            .detach();
        }
    }

    /// Opens a file from disk into an editor tab, detecting language and updating highlights.
    pub fn open_file(&mut self, path: PathBuf) -> Result<(), String> {
        // 1. Guard against opening binary / executable files
        if tree_view::is_binary_path(&path) {
            return Err(format!(
                "Cannot open binary or executable file: {}",
                path.display()
            ));
        }

        // 2. If already open in an existing tab, activate it
        if let Some(existing_idx) = self
            .tabs
            .iter()
            .position(|t| t.file_path.as_ref() == Some(&path))
        {
            self.select_tab(existing_idx);
            return Ok(());
        }

        let metadata = std::fs::metadata(&path)
            .map_err(|e| format!("Failed to inspect file {}: {e}", path.display()))?;
        if metadata.len() > arcade_workspace::DEFAULT_HUGE_FILE_THRESHOLD_BYTES as u64 {
            return Err(format!(
                "File is larger than the current {} MiB editor limit: {}",
                arcade_workspace::DEFAULT_HUGE_FILE_THRESHOLD_BYTES / (1024 * 1024),
                path.display()
            ));
        }

        // 3. Read file from disk
        let (doc, metadata) = load_from_file(&path)
            .map_err(|e| format!("Failed to read file {}: {e}", path.display()))?;
        let language = LanguageId::from_path(&path);
        let file_title = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file")
            .to_string();

        // Highlighting runs on the worker so opening a large supported file does not
        // block the UI thread while tree-sitter parses it.
        let spans = Vec::new();

        let mut hist = History::new();
        hist.mark_saved(doc.revision());

        let id = self.next_tab_id;
        self.next_tab_id += 1;

        let mut new_tab = EditorTab::new(id, file_title, doc, language, Some(path.clone()), spans);
        new_tab.history = hist;
        new_tab.file_metadata = Some(metadata);

        // If the only tab is an unmodified default Welcome or empty untitled buffer, replace it
        let should_replace_first = self.tabs.len() == 1
            && self.tabs[0].file_path.is_none()
            && !self.tabs[0]
                .history
                .is_dirty(self.tabs[0].document.revision())
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
        let discovery = arcade_workspace::discover_files_with_errors(&[&path]);
        let files = discovery.files;
        self.workspace_root = Some(path.clone());
        self.workspace_files = files;
        self.terminal.working_dir = path.clone();
        self.show_sidebar = true;
        self.expanded_folders.insert(path.clone());

        for error in discovery.errors {
            self.report_terminal_error(format!("Workspace discovery warning: {error}"));
        }

        if let Some(preferred) = self
            .workspace_files
            .iter()
            .find(|f| {
                f.file_name().is_some_and(|name| {
                    name == "README.md"
                        || name == "Cargo.toml"
                        || name == "main.rs"
                        || name == "lib.rs"
                })
            })
            .cloned()
            .or_else(|| self.workspace_files.first().cloned())
        {
            if let Err(error) = self.open_file(preferred) {
                self.report_terminal_error(error);
            }
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
            self.selections
                .add(arcade_core::selection::Selection::cursor(ByteOffset(
                    next_line_start,
                )));
        } else {
            let next_offset = ByteOffset(
                self.selections
                    .primary()
                    .head
                    .0
                    .saturating_add(1)
                    .min(self.document.len_bytes()),
            );
            self.selections
                .add(arcade_core::selection::Selection::cursor(next_offset));
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
        let current_idx = self
            .commands
            .iter()
            .position(|c| c.is_selected)
            .unwrap_or(0);
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
        let current_idx = self
            .commands
            .iter()
            .position(|c| c.is_selected)
            .unwrap_or(0);
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
            "Save Document As..." => {
                self.save_document_as();
            }
            "New Document Tab" => {
                self.new_tab();
            }
            "Close Active Tab" => {
                self.close_tab(self.active_tab_index);
            }
            "Select All" => {
                self.select_all();
            }
            "Multi-Cursor: Add Next Occurrence" => {
                self.add_next_cursor();
            }
            "Toggle Workspace File Explorer" => {
                self.toggle_sidebar();
            }
            "Toggle Terminal Maximized" => {
                self.terminal.toggle_maximized();
            }
            "Preview Current Document" => {
                self.show_preview_modal = true;
            }
            "Open Integrated Terminal with `ir`" => {
                self.show_terminal = true;
                self.terminal.is_open = true;
                self.terminal_focused = true;
            }
            "Toggle Solarized Sheen Contrast" => {
                self.toggle_theme();
            }
            "ir Documentation" => {
                self.active_help_section = HelpSection::IrDocs;
                self.show_help_modal = true;
            }
            "Check for Updates..." => {
                self.pending_update_check = true;
                self.show_updater_modal = true;
            }
            "About ArcadeEdit" => {
                self.active_help_section = HelpSection::About;
                self.show_help_modal = true;
            }
            "Help: Keyboard Shortcuts" => {
                self.active_help_section = HelpSection::Shortcuts;
                self.show_help_modal = true;
            }
            "Find in Document" => {
                self.open_find();
            }
            "Go to Line..." => {
                self.goto_line_input.clear();
                self.show_goto_line_modal = true;
            }
            "Replace in Document" => {
                self.open_replace();
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
        let tab = &self.tabs[index];
        if tab.history.is_dirty(tab.document.revision()) {
            self.pending_close_tab = Some(index);
            return;
        }
        self.close_tab_without_prompt(index);
    }

    fn close_tab_without_prompt(&mut self, index: usize) {
        self.pending_close_tab = None;
        if self.tabs.len() == 1 {
            let id = self.next_tab_id;
            self.next_tab_id += 1;
            let initial = Document::new("");
            self.tabs[0] = EditorTab::new(
                id,
                "untitled.rs",
                initial,
                LanguageId::Rust,
                None,
                Vec::new(),
            );
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

    fn confirm_discard_close(&mut self) {
        if let Some(index) = self.pending_close_tab {
            self.close_tab_without_prompt(index);
        }
    }

    fn confirm_save_close(&mut self) {
        let Some(index) = self.pending_close_tab else {
            return;
        };
        if index != self.active_tab_index {
            self.select_tab(index);
        }
        self.save_document();
        if !self.tabs[index]
            .history
            .is_dirty(self.tabs[index].document.revision())
        {
            self.close_tab_without_prompt(index);
        }
    }

    /// Opens a new empty untitled document tab.
    pub fn new_tab(&mut self) {
        self.sync_to_active_tab();
        let id = self.next_tab_id;
        self.next_tab_id += 1;
        let tab_num = self.tabs.len() + 1;
        let title = format!("untitled-{}.rs", tab_num);
        let tab = EditorTab::new(
            id,
            title,
            Document::new(""),
            LanguageId::Rust,
            None,
            Vec::new(),
        );
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

    /// Ensures that the primary cursor position is scrolled into view.
    pub fn scroll_cursor_into_view(&self) {
        let primary_head = self.selections.primary().head;
        let line_idx = self.document.line_of_byte(primary_head);
        self.editor_scroll_handle.scroll_to_item(line_idx);
    }

    /// Moves the primary cursor to a one-based line and optional one-based column.
    pub fn go_to_line(&mut self, line: usize, column: usize) {
        let rope = self.document.rope();
        let line_index = line
            .saturating_sub(1)
            .min(rope.len_lines().saturating_sub(1));
        let start_char = rope.line_to_char(line_index);
        let line_slice = rope.line(line_index);
        let content_len = line_slice
            .to_string()
            .trim_end_matches(['\n', '\r'])
            .chars()
            .count();
        let char_index = start_char + column.saturating_sub(1).min(content_len);
        let byte_index = rope.char_to_byte(char_index);
        self.selections = SelectionSet::cursor(ByteOffset(byte_index));
        self.sync_to_active_tab();
        self.scroll_cursor_into_view();
    }

    /// Opens the in-editor Find bar.
    pub fn open_find(&mut self) {
        self.find_replace.open_find();
        self.find_replace.update_matches(self.document.rope());
        if let Some(m) = self.find_replace.current_match() {
            self.selections = SelectionSet::cursor(m.start);
            self.sync_to_active_tab();
        }
    }

    /// Opens the in-editor Find & Replace bar with replacement mode enabled.
    pub fn open_replace(&mut self) {
        self.find_replace.open_replace();
        self.find_replace.update_matches(self.document.rope());
        if let Some(m) = self.find_replace.current_match() {
            self.selections = SelectionSet::cursor(m.start);
            self.sync_to_active_tab();
        }
    }

    /// Closes the in-editor Find & Replace bar.
    pub fn close_find(&mut self) {
        self.find_replace.close();
    }

    /// Cycles to the next match and places a cursor at its start.
    pub fn find_next_match(&mut self) {
        if let Some(m) = self.find_replace.next_match() {
            self.selections = SelectionSet::cursor(m.start);
            self.sync_to_active_tab();
        }
    }

    /// Cycles to the previous match and places a cursor at its start.
    pub fn find_prev_match(&mut self) {
        if let Some(m) = self.find_replace.prev_match() {
            self.selections = SelectionSet::cursor(m.start);
            self.sync_to_active_tab();
        }
    }

    /// Selects all occurrences of the query across the document as multi-cursors!
    pub fn select_all_find_matches(&mut self) {
        if self.find_replace.matches.is_empty() {
            return;
        }
        let first = self.find_replace.matches[0];
        let mut new_selections = SelectionSet::cursor(first.start);
        for m in &self.find_replace.matches {
            new_selections.add(arcade_core::selection::Selection::new(m.start, m.end));
        }
        self.selections = new_selections;
        self.sync_to_active_tab();
    }

    /// Replaces the currently active match with the replacement string.
    pub fn replace_current_find_match(&mut self) {
        if let Some(m) = self.find_replace.current_match() {
            let edit_sels =
                SelectionSet::from_selections(vec![arcade_core::Selection::new(m.start, m.end)], 0);
            if let Ok((tx, new_sels)) = self
                .document
                .insert_text_at_selections(&edit_sels, &self.find_replace.replacement)
            {
                self.history.push(tx);
                self.selections = new_sels;
                self.sync_to_active_tab();
                self.find_replace.update_matches(self.document.rope());
                self.trigger_highlight();
            }
        }
    }

    /// Replaces all occurrences of the find query across the document.
    pub fn replace_all_find_matches(&mut self) {
        if self.find_replace.query.is_empty() {
            return;
        }
        if let Ok(Some(tx)) = arcade_core::replace_all(
            &mut self.document,
            &self.find_replace.query,
            &self.find_replace.replacement,
        ) {
            self.history.push(tx);
            self.sync_to_active_tab();
            self.find_replace.update_matches(self.document.rope());
            self.trigger_highlight();
        }
    }

    /// Toggles folder expansion in the workspace directory tree.
    pub fn toggle_folder_expansion(&mut self, path: PathBuf) {
        if self.expanded_folders.contains(&path) {
            self.expanded_folders.remove(&path);
        } else {
            self.expanded_folders.insert(path);
        }
    }

    /// Inserts a string at all current cursor positions.
    pub fn insert_text(&mut self, text: &str) {
        if let Ok((tx, new_sels)) = self
            .document
            .insert_text_at_selections(&self.selections, text)
        {
            self.history.push(tx);
            self.selections = new_sels;
            self.sync_to_active_tab();
            self.trigger_highlight();
        }
    }

    fn selected_text(&self) -> Option<String> {
        let selected = self
            .selections
            .as_slice()
            .iter()
            .filter(|selection| !selection.is_empty())
            .map(|selection| {
                self.document
                    .rope()
                    .byte_slice(selection.range().start.0..selection.range().end.0)
                    .to_string()
            })
            .collect::<Vec<_>>();
        (!selected.is_empty()).then(|| selected.join("\n"))
    }

    fn copy_selection(&self, cx: &mut Context<Self>) -> bool {
        let Some(text) = self.selected_text() else {
            return false;
        };
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        true
    }

    /// Deletes backward (Backspace) across all current cursor selections.
    pub fn delete_backward(&mut self) {
        if let Ok((tx, new_sels)) = self
            .document
            .delete_backward_at_selections(&self.selections)
        {
            self.history.push(tx);
            self.selections = new_sels;
            self.sync_to_active_tab();
            self.trigger_highlight();
        }
    }

    /// Deletes forward (Delete key) across all current cursor selections.
    pub fn delete_forward(&mut self) {
        if let Ok((tx, new_sels)) = self.document.delete_forward_at_selections(&self.selections) {
            self.history.push(tx);
            self.selections = new_sels;
            self.sync_to_active_tab();
            self.trigger_highlight();
        }
    }

    /// Selects the entire document content.
    pub fn select_all(&mut self) {
        let total_bytes = self.document.len_bytes();
        self.selections = SelectionSet::single(arcade_core::Selection::new(
            ByteOffset(0),
            ByteOffset(total_bytes),
        ));
        self.sync_to_active_tab();
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
        "minus" | "hyphen" => {
            return Some(if shift {
                "_".to_string()
            } else {
                "-".to_string()
            })
        }
        "equal" | "equals" => {
            return Some(if shift {
                "+".to_string()
            } else {
                "=".to_string()
            })
        }
        "comma" => {
            return Some(if shift {
                "<".to_string()
            } else {
                ",".to_string()
            })
        }
        "period" => {
            return Some(if shift {
                ">".to_string()
            } else {
                ".".to_string()
            })
        }
        "slash" => {
            return Some(if shift {
                "?".to_string()
            } else {
                "/".to_string()
            })
        }
        "backslash" => {
            return Some(if shift {
                "|".to_string()
            } else {
                "\\".to_string()
            })
        }
        "semicolon" => {
            return Some(if shift {
                ":".to_string()
            } else {
                ";".to_string()
            })
        }
        "quote" => {
            return Some(if shift {
                "\"".to_string()
            } else {
                "'".to_string()
            })
        }
        "backquote" | "grave" => {
            return Some(if shift {
                "~".to_string()
            } else {
                "`".to_string()
            })
        }
        "bracketleft" => {
            return Some(if shift {
                "{".to_string()
            } else {
                "[".to_string()
            })
        }
        "bracketright" => {
            return Some(if shift {
                "}".to_string()
            } else {
                "]".to_string()
            })
        }
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
        window.on_window_should_close(cx, |window, cx| {
            let Some(shell) = window.root::<ArcadeShell>().flatten() else {
                return true;
            };
            let dirty = shell
                .read(cx)
                .tabs
                .iter()
                .any(|tab| tab.history.is_dirty(tab.document.revision()));
            if dirty {
                shell.update(cx, |shell, cx| {
                    shell.pending_window_close = true;
                    cx.notify();
                });
                false
            } else {
                true
            }
        });
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

        // Poll any active interactive pseudo-terminal session (e.g. ir pmon, ir nettop)
        if self.show_terminal {
            let changed = self.terminal.poll_active_session();
            if changed || self.terminal.active_session.is_some() {
                cx.notify();
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
        let show_preview_modal = self.show_preview_modal;
        let show_updater_modal = self.show_updater_modal;
        let show_goto_line_modal = self.show_goto_line_modal;
        let goto_line_input = self.goto_line_input.clone();
        let update_state = self.update_state.clone();
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
            .unwrap_or_else(|| "No Workspace".to_string());
        let line_ending_label = self
            .tabs
            .get(self.active_tab_index)
            .map(|tab| tab.line_ending_label.as_str())
            .unwrap_or("LF (default)");
        let pending_close_tab = self.pending_close_tab;

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
                if this.pending_window_close {
                    if key == "escape" {
                        this.pending_window_close = false;
                        cx.notify();
                    }
                    return;
                }
                if this.show_preview_modal {
                    if key == "escape" {
                        this.show_preview_modal = false;
                        cx.notify();
                    }
                    return;
                }
                if this.show_updater_modal {
                    if key == "escape"
                        && !matches!(
                            this.update_state,
                            UpdateState::Downloading(_) | UpdateState::Restarting(_)
                        )
                    {
                        this.show_updater_modal = false;
                        cx.notify();
                    }
                    return;
                }
                if this.show_goto_line_modal {
                    if key == "escape" {
                        this.show_goto_line_modal = false;
                    } else if key == "backspace" {
                        this.goto_line_input.pop();
                    } else if key == "enter" {
                        if let Some((line, column)) = parse_line_column(&this.goto_line_input) {
                            this.go_to_line(line, column);
                            this.show_goto_line_modal = false;
                        }
                    } else if let Some(ch) = resolve_input_character(event) {
                        if ch.chars().all(|c| c.is_ascii_digit() || c == ':') {
                            this.goto_line_input.push_str(&ch);
                        }
                    }
                    cx.notify();
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
                        this.start_pending_update_check(cx);
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

                // When Find & Replace is visible, route keyboard events to find/replace interactions:
                if this.find_replace.is_open {
                    if key == "escape" {
                        this.close_find();
                        cx.notify();
                        return;
                    }
                    if ctrl && key == "f" {
                        this.find_replace.focus = FindFocus::Query;
                        cx.notify();
                        return;
                    }
                    if ctrl && key == "h" {
                        this.find_replace.toggle_replace();
                        cx.notify();
                        return;
                    }
                    let typed_char = resolve_input_character(event);
                    let action = this.find_replace.handle_key(key, ctrl, shift, typed_char.as_deref());
                    match action {
                        FindKeyAction::Close => {
                            cx.notify();
                            return;
                        }
                        FindKeyAction::FindNext => {
                            this.find_next_match();
                            cx.notify();
                            return;
                        }
                        FindKeyAction::FindPrev => {
                            this.find_prev_match();
                            cx.notify();
                            return;
                        }
                        FindKeyAction::SelectAllMatches => {
                            this.select_all_find_matches();
                            cx.notify();
                            return;
                        }
                        FindKeyAction::ReplaceCurrent => {
                            this.replace_current_find_match();
                            cx.notify();
                            return;
                        }
                        FindKeyAction::ReplaceAll => {
                            this.replace_all_find_matches();
                            cx.notify();
                            return;
                        }
                        FindKeyAction::QueryChanged => {
                            this.find_replace.update_matches(this.document.rope());
                            if let Some(m) = this.find_replace.current_match() {
                                this.selections = SelectionSet::cursor(m.start);
                            }
                            cx.notify();
                            return;
                        }
                        FindKeyAction::ReplacementChanged => {
                            cx.notify();
                            return;
                        }
                        FindKeyAction::None => {
                            if key == "tab" || key == "up" || key == "down" || key == "enter" {
                                cx.notify();
                                return;
                            }
                        }
                    }
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
                        this.terminal.is_open = true;
                        this.terminal_focused = true;
                    } else {
                        this.terminal.is_open = false;
                        this.terminal_focused = false;
                    }
                    cx.notify();
                    return;
                }

                // When Terminal is open and focused, route keys to terminal session
                if this.show_terminal && this.terminal_focused {
                    if key == "escape" && this.terminal.active_session.is_none() {
                        this.terminal_focused = false;
                        cx.notify();
                        return;
                    }
                    let typed_char = resolve_input_character(event);
                    if this.terminal.handle_key(key, ctrl, shift, typed_char.as_deref()) {
                        if this.terminal.has_pending_commands() {
                            poll_terminal_commands(cx);
                        }
                        if !this.terminal.is_open {
                            this.show_terminal = false;
                            this.terminal_focused = false;
                        }
                        cx.notify();
                        return;
                    }
                    if !this.terminal.is_open {
                        this.show_terminal = false;
                        this.terminal_focused = false;
                        cx.notify();
                    }
                    return;
                }

                // Normal Editor Keyboard Shortcuts:
                if ctrl && key == "g" {
                    this.goto_line_input.clear();
                    this.show_goto_line_modal = true;
                    cx.notify();
                    return;
                }
                if ctrl && key == "c" {
                    this.copy_selection(cx);
                    cx.notify();
                    return;
                }
                if ctrl && key == "x" {
                    if this.copy_selection(cx) {
                        this.delete_backward();
                        cx.notify();
                    }
                    return;
                }
                if ctrl && key == "v" {
                    if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
                        this.insert_text(&text);
                        cx.notify();
                    }
                    return;
                }
                // Find in Document (Ctrl+F)
                if ctrl && key == "f" {
                    this.open_find();
                    cx.notify();
                    return;
                }

                // Replace in Document (Ctrl+H)
                if ctrl && key == "h" {
                    this.open_replace();
                    cx.notify();
                    return;
                }

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
                if ctrl && shift && key == "s" {
                    this.save_document_as();
                    cx.notify();
                    return;
                }
                if ctrl && key == "s" {
                    this.save_document();
                    cx.notify();
                    return;
                }

                // Select All (Ctrl+A)
                if ctrl && key == "a" {
                    this.select_all();
                    cx.notify();
                    return;
                }

                // New Tab (Ctrl+N)
                if ctrl && key == "n" {
                    this.new_tab();
                    cx.notify();
                    return;
                }

                // Close Tab (Ctrl+W)
                if ctrl && key == "w" {
                    this.close_tab(this.active_tab_index);
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
                    this.scroll_cursor_into_view();
                    cx.notify();
                    return;
                }
                if key == "down" {
                    this.selections = this.selections.move_down(this.document.rope(), shift);
                    this.scroll_cursor_into_view();
                    cx.notify();
                    return;
                }

                // Navigation: Home / End
                if key == "home" {
                    this.selections = this.selections.move_to_line_start(this.document.rope(), shift);
                    this.scroll_cursor_into_view();
                    cx.notify();
                    return;
                }
                if key == "end" {
                    this.selections = this.selections.move_to_line_end(this.document.rope(), shift);
                    this.scroll_cursor_into_view();
                    cx.notify();
                    return;
                }

                // Navigation: PageUp / PageDown
                if key == "pageup" {
                    for _ in 0..15 {
                        this.selections = this.selections.move_up(this.document.rope(), shift);
                    }
                    this.scroll_cursor_into_view();
                    cx.notify();
                    return;
                }
                if key == "pagedown" {
                    for _ in 0..15 {
                        this.selections = this.selections.move_down(this.document.rope(), shift);
                    }
                    this.scroll_cursor_into_view();
                    cx.notify();
                    return;
                }

                // Delete (Forward)
                if key == "delete" {
                    this.delete_forward();
                    this.scroll_cursor_into_view();
                    cx.notify();
                    return;
                }

                // Backspace
                if key == "backspace" {
                    this.delete_backward();
                    this.scroll_cursor_into_view();
                    cx.notify();
                    return;
                }

                // Enter / Return
                if key == "enter" {
                    let newline = this
                        .tabs
                        .get(this.active_tab_index)
                        .map(|tab| tab.line_ending.clone())
                        .unwrap_or_else(|| "\n".into());
                    this.insert_text(&newline);
                    this.scroll_cursor_into_view();
                    cx.notify();
                    return;
                }

                // Tab
                if key == "tab" {
                    this.insert_text("    ");
                    this.scroll_cursor_into_view();
                    cx.notify();
                    return;
                }

                // Text Insertion (Printable characters without Control/Alt)
                if let Some(ch) = resolve_input_character(event) {
                    this.insert_text(&ch);
                    this.scroll_cursor_into_view();
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
                                    .child(workspace_title.clone()),
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
                    .min_h_0()
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
                                                .child(workspace_title.clone()),
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
                                        .when(self.workspace_root.is_some(), |el| {
                                            let root = self.workspace_root.as_ref().unwrap();
                                            let current_opt = self.current_file_path.clone();
                                            let visible_items = build_visible_tree(
                                                root,
                                                &self.expanded_folders,
                                                &self.workspace_files,
                                            );

                                            let tree_nodes: Vec<_> = visible_items.into_iter().map(|item| {
                                                let is_active = !item.is_dir && (current_opt.as_ref() == Some(&item.path));
                                                let path_clone = item.path.clone();
                                                let is_dir = item.is_dir;
                                                let is_bin = item.is_binary();
                                                let icon = item.icon();
                                                let toggle_arrow = if item.is_dir {
                                                    if item.is_expanded { "▾ " } else { "▸ " }
                                                } else {
                                                    "  "
                                                };
                                                let indent = px(item.depth as f32 * 12.0);

                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .gap_1p5()
                                                    .pl(indent)
                                                    .pr_2()
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
                                                    } else if is_bin {
                                                        theme.warning_red
                                                    } else if is_dir {
                                                        theme.syntax_yellow
                                                    } else {
                                                        theme.text_secondary
                                                    })
                                                    .hover(|s| s.bg(theme.bg_hover_glass))
                                                    .cursor_pointer()
                                                    .on_mouse_down(gpui::MouseButton::Left, cx.listener(move |this, _, _, cx| {
                                                        if is_dir {
                                                            this.toggle_folder_expansion(path_clone.clone());
                                                        } else if is_bin {
                                                            this.show_terminal = true;
                                                            this.terminal.is_open = true;
                                                            this.terminal_focused = true;
                                                            this.terminal.lines.push(TerminalLine::new(
                                                                TerminalLineKind::Error,
                                                                format!("Blocked opening binary executable: {}", path_clone.display()),
                                                            ));
                                                        } else {
                                                            let _ = this.open_file(path_clone.clone());
                                                        }
                                                        cx.notify();
                                                    }))
                                                    .child(
                                                        div()
                                                            .text_size(px(10.0))
                                                            .text_color(theme.text_muted)
                                                            .child(toggle_arrow),
                                                    )
                                                    .child(
                                                        div()
                                                            .text_size(px(11.0))
                                                            .child(icon),
                                                    )
                                                    .child(
                                                        div()
                                                            .text_size(px(11.5))
                                                            .font_weight(if is_active || is_dir {
                                                                gpui::FontWeight::MEDIUM
                                                            } else {
                                                                gpui::FontWeight::NORMAL
                                                            })
                                                            .overflow_hidden()
                                                            .child(if is_bin {
                                                                format!("{} [bin]", item.name)
                                                            } else {
                                                                item.name
                                                            }),
                                                    )
                                            }).collect();

                                            el.child(
                                                div()
                                                    .id("sidebar-tree-scroll")
                                                    .flex_1()
                                                    .min_h_0()
                                                    .flex()
                                                    .flex_col()
                                                    .overflow_y_scroll()
                                                    .children(tree_nodes),
                                            )
                                        })
                                        .when(self.workspace_root.is_none(), |el| {
                                            el.child(
                                                div()
                                                    .p_3()
                                                    .flex()
                                                    .flex_col()
                                                    .gap_2()
                                                    .text_size(px(11.5))
                                                    .text_color(theme.text_muted)
                                                    .child("Open a workspace folder to browse its files."),
                                            )
                                        }),
                                ),
                        )
                    })
                    // Main Editor Surface
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_h_0()
                            .min_w_0()
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
                                &workspace_title,
                                &self.highlight_spans,
                                &self.find_replace.matches,
                                self.find_replace.current_match(),
                                &self.editor_scroll_handle,
                                cx,
                            ))
                            .when(self.find_replace.is_open, |p| {
                                p.child(render_find_replace_bar(&theme, &self.find_replace, cx))
                            }),
                    ),
            )
            // =================================================================
            // 4. PERSISTENT DOCKED TERMINAL PANE (Root Window Level)
            // =================================================================
            .when(show_terminal, |p| {
                p.child(
                    div()
                        .w_full()
                        .flex_shrink_0()
                        .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
                            this.terminal_focused = true;
                            cx.notify();
                        }))
                        .child(render_terminal_panel(&theme, &self.terminal, cx)),
                )
            })
            // =================================================================
            // 5. STATUS BAR (Minimalist Glassy Bottom Strip)
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
                                    .child("EDIT"),
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
                                    .child(format!("UTF-8  •  {}  •  {}", line_ending_label, lang_label))
                            })
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .px_2()
                                    .py_0p5()
                                    .rounded_md()
                                    .bg(if show_terminal {
                                        theme.bg_active_glass
                                    } else {
                                        gpui::rgba(0x00000000)
                                    })
                                    .border_1()
                                    .border_color(if show_terminal {
                                        theme.border_glass
                                    } else {
                                        gpui::rgba(0x00000000)
                                    })
                                    .text_color(if show_terminal {
                                        theme.syntax_cyan
                                    } else {
                                        theme.text_muted
                                    })
                                    .cursor_pointer()
                                    .hover(|s| s.bg(theme.bg_hover_glass).text_color(theme.text_bright))
                                    .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
                                        this.show_terminal = !this.show_terminal;
                                        if this.show_terminal {
                                            this.terminal.is_open = true;
                                            this.terminal_focused = true;
                                        } else {
                                            this.terminal.is_open = false;
                                            this.terminal_focused = false;
                                        }
                                        cx.notify();
                                    }))
                                    .child("📟 Terminal"),
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
                                    .child(line_status),
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
            .when(show_preview_modal, |parent| {
                parent.child(render_document_preview(&self.document.to_string(), &theme, cx))
            })
            .when(show_updater_modal, |parent| {
                parent.child(render_updater_modal(&update_state, &theme, cx))
            })
            .when(show_goto_line_modal, |parent| {
                parent.child(render_goto_line_modal(&goto_line_input, &theme))
            })
            .when_some(pending_close_tab, |parent, tab_index| {
                parent.child(
                    div()
                        .absolute()
                        .inset_0()
                        .bg(gpui::rgba(0x001018a0))
                        .flex()
                        .justify_center()
                        .items_center()
                        .child(
                            div()
                                .w(px(420.0))
                                .p_5()
                                .rounded_xl()
                                .bg(theme.bg_card_glass)
                                .border_1()
                                .border_color(theme.border_glass)
                                .flex()
                                .flex_col()
                                .gap_4()
                                .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
                                .child(div().text_color(theme.text_bright).child("Save changes before closing this tab?"))
                                .child(
                                    div()
                                        .flex()
                                        .justify_end()
                                        .gap_2()
                                        .child(close_prompt_button("Cancel", ClosePromptAction::Cancel, &theme, cx))
                                        .child(close_prompt_button("Discard", ClosePromptAction::Discard(tab_index), &theme, cx))
                                        .child(close_prompt_button("Save", ClosePromptAction::Save(tab_index), &theme, cx)),
                                ),
                        ),
                )
            })
            .when(self.pending_window_close, |parent| {
                parent.child(render_window_close_prompt(&theme, cx))
            })
    }
}

fn parse_line_column(input: &str) -> Option<(usize, usize)> {
    let mut parts = input.split(':');
    let line = parts.next()?.parse::<usize>().ok()?;
    let column = parts
        .next()
        .map(str::parse::<usize>)
        .transpose()
        .ok()?
        .unwrap_or(1);
    if parts.next().is_some() || line == 0 || column == 0 {
        return None;
    }
    Some((line, column))
}

fn render_goto_line_modal(input: &str, theme: &SolarizedTheme) -> impl IntoElement {
    div()
        .absolute()
        .inset_0()
        .bg(gpui::rgba(0x001018a0))
        .flex()
        .justify_center()
        .items_center()
        .child(
            div()
                .w(px(400.0))
                .p_5()
                .rounded_xl()
                .bg(theme.bg_card_glass)
                .border_1()
                .border_color(theme.border_glass)
                .flex()
                .flex_col()
                .gap_3()
                .child(div().text_color(theme.text_bright).child("Go to Line"))
                .child(
                    div()
                        .px_3()
                        .py_2()
                        .rounded_lg()
                        .bg(theme.bg_surface_glass)
                        .text_color(theme.syntax_cyan)
                        .child(if input.is_empty() {
                            "Line or line:column (for example 24:8)".to_string()
                        } else {
                            input.to_string()
                        }),
                )
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(theme.text_muted)
                        .child("Enter to move · Esc to cancel"),
                ),
        )
}

fn render_window_close_prompt(
    theme: &SolarizedTheme,
    cx: &mut Context<ArcadeShell>,
) -> impl IntoElement {
    div()
        .absolute()
        .inset_0()
        .bg(gpui::rgba(0x001018a0))
        .flex()
        .justify_center()
        .items_center()
        .child(
            div()
                .w(px(460.0))
                .p_5()
                .rounded_xl()
                .bg(theme.bg_card_glass)
                .border_1()
                .border_color(theme.border_glass)
                .flex()
                .flex_col()
                .gap_4()
                .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(
                    div()
                        .text_color(theme.text_bright)
                        .child("Save changes before closing ArcadeEdit?"),
                )
                .child(
                    div()
                        .flex()
                        .justify_end()
                        .gap_2()
                        .child(window_close_button("Cancel", false, false, theme, cx))
                        .child(window_close_button("Discard", true, false, theme, cx))
                        .child(window_close_button("Save All", false, true, theme, cx)),
                ),
        )
}

fn window_close_button(
    label: &'static str,
    discard: bool,
    save: bool,
    theme: &SolarizedTheme,
    cx: &mut Context<ArcadeShell>,
) -> impl IntoElement {
    div()
        .px_3()
        .py_2()
        .rounded_lg()
        .bg(theme.bg_surface_glass)
        .border_1()
        .border_color(theme.border_glass)
        .text_color(theme.text_bright)
        .cursor_pointer()
        .on_mouse_down(
            gpui::MouseButton::Left,
            cx.listener(move |this, _, _, cx| {
                this.pending_window_close = false;
                if discard {
                    cx.quit();
                } else if save {
                    this.save_all_and_quit(cx);
                }
                cx.notify();
            }),
        )
        .child(label)
}

#[derive(Clone, Copy)]
enum UpdaterModalAction {
    Install,
    CheckAgain,
    Close,
}

fn updater_modal_button(
    label: &'static str,
    action: UpdaterModalAction,
    theme: &SolarizedTheme,
    cx: &mut Context<ArcadeShell>,
) -> impl IntoElement {
    div()
        .px_3()
        .py_2()
        .rounded_lg()
        .bg(theme.bg_surface_glass)
        .border_1()
        .border_color(theme.border_glass)
        .text_color(theme.text_bright)
        .cursor_pointer()
        .hover(|style| style.bg(theme.bg_hover_glass))
        .on_mouse_down(
            gpui::MouseButton::Left,
            cx.listener(move |this, _, _, cx| {
                match action {
                    UpdaterModalAction::Install => this.install_update(cx),
                    UpdaterModalAction::CheckAgain => this.check_for_updates(cx, true),
                    UpdaterModalAction::Close => this.show_updater_modal = false,
                }
                cx.notify();
            }),
        )
        .child(label)
}

fn render_updater_modal(
    state: &UpdateState,
    theme: &SolarizedTheme,
    cx: &mut Context<ArcadeShell>,
) -> impl IntoElement {
    let (title, message, notes) = match state {
        UpdateState::Idle => (
            "ArcadeEdit Updates",
            "No update check has run yet.".to_string(),
            None,
        ),
        UpdateState::Checking => (
            "Checking for Updates",
            "Contacting GitHub Releases…".to_string(),
            None,
        ),
        UpdateState::UpToDate => (
            "ArcadeEdit is Up to Date",
            format!("You are running v{}.", env!("CARGO_PKG_VERSION")),
            None,
        ),
        UpdateState::Available(update) => (
            "Update Available",
            format!("ArcadeEdit v{} is ready to install.", update.version),
            Some(update.notes.chars().take(4_000).collect::<String>()),
        ),
        UpdateState::Downloading(version) => (
            "Installing Update",
            format!(
                "Downloading and verifying v{version}. The app will restart when it is staged."
            ),
            None,
        ),
        UpdateState::Restarting(version) => (
            "Restarting ArcadeEdit",
            format!("The verified v{version} update is staged."),
            None,
        ),
        UpdateState::Failed(error) => ("Update Check Failed", error.clone(), None),
    };
    let installing = matches!(
        state,
        UpdateState::Downloading(_) | UpdateState::Restarting(_)
    );

    div()
        .absolute()
        .inset_0()
        .bg(gpui::rgba(0x001018a0))
        .flex()
        .justify_center()
        .items_center()
        .on_mouse_down(
            gpui::MouseButton::Left,
            cx.listener(|this, _, _, cx| {
                if !matches!(
                    this.update_state,
                    UpdateState::Downloading(_) | UpdateState::Restarting(_)
                ) {
                    this.show_updater_modal = false;
                    cx.notify();
                }
            }),
        )
        .child(
            div()
                .w(px(520.0))
                .max_w(px(760.0))
                .max_h(px(600.0))
                .rounded_2xl()
                .bg(theme.bg_card_glass)
                .border_1()
                .border_color(theme.border_glass)
                .shadow_xl()
                .flex()
                .flex_col()
                .overflow_hidden()
                .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(
                    div()
                        .px_5()
                        .py_4()
                        .border_b_1()
                        .border_color(theme.border_subtle)
                        .text_color(theme.text_bright)
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .child(title),
                )
                .child(
                    div()
                        .p_5()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .text_color(theme.text_primary)
                        .child(message)
                        .when_some(notes, |view, notes| {
                            if notes.trim().is_empty() {
                                view
                            } else {
                                view.child(
                                    div()
                                        .id("updater-release-notes")
                                        .max_h(px(300.0))
                                        .overflow_y_scroll()
                                        .p_3()
                                        .rounded_lg()
                                        .bg(theme.bg_surface_glass)
                                        .text_color(theme.text_secondary)
                                        .child(notes),
                                )
                            }
                        })
                        .child(
                            div()
                                .flex()
                                .justify_end()
                                .gap_2()
                                .when(matches!(state, UpdateState::Available(_)), |buttons| {
                                    buttons
                                        .child(updater_modal_button(
                                            "Later",
                                            UpdaterModalAction::Close,
                                            theme,
                                            cx,
                                        ))
                                        .child(updater_modal_button(
                                            "Update and Restart",
                                            UpdaterModalAction::Install,
                                            theme,
                                            cx,
                                        ))
                                })
                                .when(
                                    matches!(state, UpdateState::Failed(_) | UpdateState::UpToDate),
                                    |buttons| {
                                        buttons
                                            .child(updater_modal_button(
                                                "Check Again",
                                                UpdaterModalAction::CheckAgain,
                                                theme,
                                                cx,
                                            ))
                                            .child(updater_modal_button(
                                                "Close",
                                                UpdaterModalAction::Close,
                                                theme,
                                                cx,
                                            ))
                                    },
                                )
                                .when(
                                    !matches!(
                                        state,
                                        UpdateState::Available(_)
                                            | UpdateState::Failed(_)
                                            | UpdateState::UpToDate
                                    ) && !installing,
                                    |buttons| {
                                        buttons.child(updater_modal_button(
                                            "Close",
                                            UpdaterModalAction::Close,
                                            theme,
                                            cx,
                                        ))
                                    },
                                ),
                        ),
                ),
        )
}

fn render_document_preview(
    contents: &str,
    theme: &SolarizedTheme,
    cx: &mut Context<ArcadeShell>,
) -> impl IntoElement {
    let line_count = contents.lines().count();
    let char_count = contents.chars().count();
    div()
        .absolute()
        .inset_0()
        .bg(gpui::rgba(0x001018a0))
        .flex()
        .justify_center()
        .items_center()
        .on_mouse_down(
            gpui::MouseButton::Left,
            cx.listener(|this, _, _, cx| {
                this.show_preview_modal = false;
                cx.notify();
            }),
        )
        .child(
            div()
                .w(px(900.0))
                .max_w(px(1100.0))
                .h(px(600.0))
                .rounded_2xl()
                .bg(theme.bg_card_glass)
                .border_1()
                .border_color(theme.border_glass)
                .shadow_xl()
                .flex()
                .flex_col()
                .overflow_hidden()
                .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(
                    div()
                        .h(px(44.0))
                        .px_4()
                        .flex()
                        .items_center()
                        .justify_between()
                        .bg(theme.bg_surface_glass)
                        .border_b_1()
                        .border_color(theme.border_subtle)
                        .child(
                            div()
                                .text_color(theme.text_bright)
                                .child("Document Preview"),
                        )
                        .child(div().text_color(theme.text_muted).child(format!(
                            "{line_count} lines  ·  {char_count} characters  ·  Read only"
                        ))),
                )
                .child(
                    div()
                        .id("document-preview-scroll")
                        .flex_1()
                        .min_h_0()
                        .overflow_y_scroll()
                        .p_4()
                        .font_family(
                            "Consolas, 'Cascadia Code', 'Fira Code', 'Courier New', monospace",
                        )
                        .text_size(px(12.5))
                        .text_color(theme.text_primary)
                        .child(if contents.is_empty() {
                            "(Empty document)".to_string()
                        } else {
                            contents.to_string()
                        }),
                ),
        )
}

fn poll_terminal_commands(cx: &mut Context<ArcadeShell>) {
    cx.spawn(async move |this, cx| loop {
        cx.background_executor()
            .timer(std::time::Duration::from_millis(40))
            .await;
        let keep_polling = this
            .update(cx, |this, cx| {
                let changed = this.terminal.poll_active_session();
                if changed {
                    cx.notify();
                }
                this.terminal.has_pending_commands()
            })
            .unwrap_or(false);
        if !keep_polling {
            break;
        }
    })
    .detach();
}

#[derive(Clone, Copy)]
enum ClosePromptAction {
    Cancel,
    Discard(usize),
    Save(usize),
}

fn close_prompt_button(
    label: &'static str,
    action: ClosePromptAction,
    theme: &SolarizedTheme,
    cx: &mut Context<ArcadeShell>,
) -> impl IntoElement {
    div()
        .px_3()
        .py_1()
        .rounded_md()
        .bg(theme.badge_bg)
        .border_1()
        .border_color(theme.border_subtle)
        .text_color(theme.text_bright)
        .cursor_pointer()
        .on_mouse_down(
            gpui::MouseButton::Left,
            cx.listener(move |this, _, _, cx| {
                match action {
                    ClosePromptAction::Cancel => this.pending_close_tab = None,
                    ClosePromptAction::Discard(index) => {
                        this.pending_close_tab = Some(index);
                        this.confirm_discard_close();
                    }
                    ClosePromptAction::Save(index) => {
                        this.pending_close_tab = Some(index);
                        this.confirm_save_close();
                    }
                }
                cx.notify();
            }),
        )
        .child(label)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_line_and_optional_column() {
        assert_eq!(parse_line_column("24"), Some((24, 1)));
        assert_eq!(parse_line_column("24:8"), Some((24, 8)));
        assert_eq!(parse_line_column("0"), None);
        assert_eq!(parse_line_column("2:0"), None);
        assert_eq!(parse_line_column("2:3:4"), None);
    }

    #[test]
    fn go_to_line_clamps_to_document_line_and_column() {
        let mut shell = ArcadeShell::test_stub();
        shell.document = Document::new("one\ntwo\n");
        shell.go_to_line(9, 99);
        assert_eq!(
            shell.document.line_of_byte(shell.selections.primary().head),
            2
        );
        assert_eq!(
            shell.selections.primary().head.0,
            shell.document.rope().len_bytes()
        );
    }

    #[test]
    fn initializes_arcade_shell_with_solarized_glass_defaults() {
        let shell = ArcadeShell::test_stub();
        assert_eq!(shell.active_tab, 0);
        assert!(!shell.show_command_palette);
        assert!(!shell.commands.is_empty());
        assert_eq!(DESIGN_SYSTEM_NAME, "ArcadeEdit Solarized Glass");
    }

    #[test]
    fn detects_mixed_newlines_and_uses_the_first_style_for_new_lines() {
        assert_eq!(
            detect_line_ending("one\r\ntwo\nthree"),
            ("\r\n".into(), "Mixed (CRLF + LF)".into())
        );
        assert_eq!(
            detect_line_ending("one\r\ntwo"),
            ("\r\n".into(), "CRLF".into())
        );
        assert_eq!(detect_line_ending("one\ntwo"), ("\n".into(), "LF".into()));
        assert_eq!(detect_line_ending(""), ("\n".into(), "LF (default)".into()));
    }

    #[test]
    fn toggles_command_palette_visibility() {
        let mut shell = ArcadeShell::test_stub();
        assert!(!shell.show_command_palette);

        shell.toggle_command_palette();
        assert!(shell.show_command_palette);

        shell.toggle_command_palette();
        assert!(!shell.show_command_palette);

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
        assert!(shell
            .document
            .to_string()
            .contains("ArcadeEdit Test Document"));

        // Switch back to tab 1 and verify content was preserved
        shell.select_tab(1);
        assert_eq!(shell.active_tab, 1);
        assert!(shell.document.to_string().contains("tab_one"));

        // Close tab 1
        shell.close_tab(1);
        assert_eq!(shell.pending_close_tab, Some(1));
        shell.confirm_discard_close();
        assert_eq!(shell.tabs.len(), 1);
        assert_eq!(shell.active_tab, 0);
        assert!(shell
            .document
            .to_string()
            .contains("ArcadeEdit Test Document"));
    }

    #[test]
    fn opens_multiple_files_in_tabs_and_deduplicates() {
        let mut shell = ArcadeShell::test_stub();
        let cargo_toml_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        let lib_rs_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("lib.rs");

        // 1. Open Cargo.toml (replaces initial Welcome stub)
        shell
            .open_file(cargo_toml_path.clone())
            .expect("Failed to open Cargo.toml");
        assert_eq!(shell.tabs.len(), 1);
        assert_eq!(shell.active_tab, 0);
        assert_eq!(shell.current_file_path, Some(cargo_toml_path.clone()));
        assert_eq!(shell.language, LanguageId::Toml);

        // 2. Open lib.rs in a new tab
        shell
            .open_file(lib_rs_path.clone())
            .expect("Failed to open lib.rs");
        assert_eq!(shell.tabs.len(), 2);
        assert_eq!(shell.active_tab, 1);
        assert_eq!(shell.current_file_path, Some(lib_rs_path.clone()));
        assert_eq!(shell.language, LanguageId::Rust);

        // 3. Re-open Cargo.toml (should switch to existing tab at index 0 without duplicating)
        shell
            .open_file(cargo_toml_path.clone())
            .expect("Failed to re-open Cargo.toml");
        assert_eq!(
            shell.tabs.len(),
            2,
            "Re-opening existing file must not create a duplicate tab"
        );
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
    fn extracts_selected_text_in_document_order_for_clipboard() {
        let mut shell = ArcadeShell::test_stub();
        shell.document = Document::new("alpha beta");
        shell.selections = SelectionSet::from_selections(
            vec![
                arcade_core::Selection::new(ByteOffset(6), ByteOffset(10)),
                arcade_core::Selection::new(ByteOffset(0), ByteOffset(5)),
            ],
            0,
        );
        assert_eq!(shell.selected_text().as_deref(), Some("alpha\nbeta"));
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
        let save_idx = shell
            .commands
            .iter()
            .position(|c| c.title == "Save Document")
            .unwrap();
        let save_path =
            std::env::temp_dir().join(format!("arcade_ui_save_{}.txt", std::process::id()));
        shell.current_file_path = Some(save_path.clone());
        shell.execute_command_at(save_idx);

        assert!(!shell.show_command_palette);
        assert!(!shell.history.is_dirty(shell.document.revision()));
        let _ = std::fs::remove_file(save_path);
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

        shell
            .open_file(cargo_toml_path.clone())
            .expect("Failed to open file");
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
        assert!(shell
            .workspace_files
            .iter()
            .any(|f| f.ends_with("Cargo.toml")));
        assert!(shell.show_sidebar);
        assert!(shell.current_file_path.is_some());
    }

    #[test]
    fn manages_integrated_terminal_lifecycle_and_commands() {
        let mut terminal = TerminalState::new(PathBuf::from("."));
        assert!(!terminal.lines.is_empty());
        assert!(terminal
            .lines
            .iter()
            .any(|l| l.text.contains("ArcadeEdit Integrated Terminal")));

        // Typing into input buffer
        assert!(terminal.handle_key("h", false, false, None));
        assert!(terminal.handle_key("i", false, false, None));
        assert_eq!(terminal.input_buffer, "hi");

        // Typing space and hyphen directly and via character strings
        assert!(terminal.handle_key("space", false, false, None));
        assert!(terminal.handle_key("minus", false, false, None));
        assert!(terminal.handle_key("v", false, false, Some("v")));
        assert_eq!(terminal.input_buffer, "hi -v");

        // Backspace
        assert!(terminal.handle_key("backspace", false, false, None));
        assert_eq!(terminal.input_buffer, "hi -");

        // Ctrl+C to abort line
        assert!(terminal.handle_key("c", true, false, None));
        assert!(terminal.input_buffer.is_empty());

        // Echo command
        terminal.execute_command("echo arcade_term_test");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        while terminal.has_pending_commands() && std::time::Instant::now() < deadline {
            terminal.poll_active_session();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(terminal.lines.iter().any(|line| {
            line.kind == TerminalLineKind::Output && line.text.contains("arcade_term_test")
        }));

        // Exit / quit command cleanly closes session
        terminal.is_open = true;
        assert!(terminal.is_open);
        terminal.execute_command("exit");
        assert!(!terminal.is_open);
        assert!(terminal
            .lines
            .iter()
            .any(|l| l.text.contains("[Terminal session closed]")));

        terminal.is_open = true;
        terminal.execute_command("quit");
        assert!(!terminal.is_open);

        // Clear command
        terminal.execute_command("clear");
        assert!(terminal.lines.is_empty());
    }

    #[test]
    fn refuses_to_open_binary_files() {
        let mut shell = ArcadeShell::test_stub();
        let exe_result = shell.open_file(PathBuf::from("arcade.exe"));
        assert!(exe_result.is_err());
        assert!(exe_result
            .unwrap_err()
            .contains("Cannot open binary or executable file"));

        let dll_result = shell.open_file(PathBuf::from("library.dll"));
        assert!(dll_result.is_err());
    }

    #[test]
    fn terminal_persists_across_tab_switches() {
        let mut shell = ArcadeShell::test_stub();
        shell.show_terminal = true;
        shell.terminal.is_open = true;

        // Switch tabs back and forth
        shell.select_tab(1);
        assert!(
            shell.show_terminal,
            "Terminal must remain open when switching tabs"
        );
        assert!(shell.terminal.is_open);

        shell.select_tab(0);
        assert!(
            shell.show_terminal,
            "Terminal must remain open when switching back"
        );
        assert!(shell.terminal.is_open);

        shell.new_tab();
        assert!(
            shell.show_terminal,
            "Terminal must remain open when creating a new tab"
        );
        assert!(shell.terminal.is_open);
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

        // Preview is a read-only view of the active buffer.
        let before = shell.document.to_string();
        let preview_idx = shell
            .commands
            .iter()
            .position(|c| c.title == "Preview Current Document")
            .expect("Preview Current Document command missing");
        shell.execute_command_at(preview_idx);
        assert!(shell.show_preview_modal);
        assert_eq!(shell.document.to_string(), before);
    }

    #[test]
    fn resolves_bundled_ir_binary() {
        let resolved = terminal::resolve_ir_binary();
        if cfg!(windows) {
            assert!(
                resolved.is_some(),
                "Bundled ir binary should be resolved in development/workspace on Windows"
            );
            let path = resolved.unwrap();
            assert!(path.exists());
            assert!(path.to_string_lossy().contains("ir"));
        } else if let Some(path) = resolved {
            assert!(path.exists());
            assert!(path.to_string_lossy().contains("ir"));
        }
    }

    #[test]
    fn finds_and_replaces_occurrences_in_document() {
        let mut shell = ArcadeShell::test_stub();
        shell.document = Document::new("apple banana apple cherry apple");
        shell.sync_to_active_tab();

        // 1. Open Find bar
        shell.open_find();
        assert!(shell.find_replace.is_open);

        // 2. Set query and compute matches
        shell.find_replace.query = "apple".to_string();
        shell.find_replace.update_matches(shell.document.rope());
        assert_eq!(shell.find_replace.matches.len(), 3);

        // 3. Navigation cycling
        shell.find_next_match();
        assert_eq!(shell.find_replace.active_match_index, 1);
        shell.find_prev_match();
        assert_eq!(shell.find_replace.active_match_index, 0);

        // 4. Select all matches as multi-cursors
        shell.select_all_find_matches();
        assert_eq!(shell.selections.len(), 3);

        // 5. Replace current match
        shell.find_replace.replacement = "orange".to_string();
        shell.replace_current_find_match();
        assert!(shell.document.to_string().starts_with("orange banana"));
        assert_eq!(shell.find_replace.matches.len(), 2);

        // 6. Replace all remaining matches
        shell.replace_all_find_matches();
        assert_eq!(
            shell.document.to_string(),
            "orange banana orange cherry orange"
        );
        assert_eq!(shell.find_replace.matches.len(), 0);

        // 7. Close find bar
        shell.close_find();
        assert!(!shell.find_replace.is_open);
    }

    #[test]
    fn expands_and_navigates_directory_tree() {
        let mut shell = ArcadeShell::test_stub();
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        shell.open_folder(manifest_dir.clone());

        assert_eq!(shell.workspace_root, Some(manifest_dir.clone()));
        assert!(shell.expanded_folders.contains(&manifest_dir));

        let src_dir = manifest_dir.join("src");
        assert!(!shell.expanded_folders.contains(&src_dir));

        // Toggle expand src_dir
        shell.toggle_folder_expansion(src_dir.clone());
        assert!(shell.expanded_folders.contains(&src_dir));

        // Toggle collapse src_dir
        shell.toggle_folder_expansion(src_dir.clone());
        assert!(!shell.expanded_folders.contains(&src_dir));
    }

    #[test]
    fn deletes_forward_and_selects_all() {
        let mut shell = ArcadeShell::test_stub();
        shell.document = Document::new("Hello World");
        shell.selections = SelectionSet::cursor(ByteOffset(0));
        shell.sync_to_active_tab();

        // 1. Forward delete removes character ahead of cursor ('H')
        shell.delete_forward();
        assert_eq!(shell.document.to_string(), "ello World");

        // 2. Select All selects full buffer
        shell.select_all();
        assert_eq!(
            shell.selections.primary().head,
            ByteOffset(shell.document.len_bytes())
        );
        assert_eq!(shell.selections.primary().anchor, ByteOffset(0));

        // 3. Typing over full selection replaces entire text
        shell.insert_text("Arcade");
        assert_eq!(shell.document.to_string(), "Arcade");
    }

    #[test]
    fn executes_new_tab_close_tab_and_terminal_maximize() {
        let mut shell = ArcadeShell::test_stub();
        assert_eq!(shell.tabs.len(), 1);

        // 1. New tab via palette
        let new_tab_idx = shell
            .commands
            .iter()
            .position(|c| c.title == "New Document Tab")
            .expect("New Document Tab command missing");
        shell.execute_command_at(new_tab_idx);
        assert_eq!(shell.tabs.len(), 2);
        assert_eq!(shell.active_tab_index, 1);

        // 2. Close active tab via palette
        let close_tab_idx = shell
            .commands
            .iter()
            .position(|c| c.title == "Close Active Tab")
            .expect("Close Active Tab command missing");
        shell.execute_command_at(close_tab_idx);
        assert_eq!(shell.tabs.len(), 1);
        assert_eq!(shell.active_tab_index, 0);

        // 3. Toggle terminal maximize command
        let max_idx = shell
            .commands
            .iter()
            .position(|c| c.title == "Toggle Terminal Maximized")
            .expect("Toggle Terminal Maximized command missing");
        assert!(!shell.terminal.is_maximized);
        shell.execute_command_at(max_idx);
        assert!(shell.terminal.is_maximized);
        shell.execute_command_at(max_idx);
        assert!(!shell.terminal.is_maximized);
    }
}
