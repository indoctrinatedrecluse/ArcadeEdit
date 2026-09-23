# Changelog

All notable changes to the ArcadeEdit project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [1.2.0] - 2026-09-23

### ⚡ Interactive ConPTY / POSIX Terminal, Live TUI Screen Grid, and Containerized Modes

ArcadeEdit v1.2.0 brings a major upgrade to the integrated terminal drawer: a full cross-platform Pseudo-Terminal (PTY) engine powered by `portable-pty` and `vte`, supporting complex interactive TUI applications (such as `ir pmon`, `ir nettop`, `ir dua`, `ir fm`), bidirectional real-time key navigation, in-place 2D screen grid rendering, and first-class Linux/Containerized/Headless deployment configurations.

---

### 🖥️ Native Interactive Pseudo-Terminal Engine (`portable-pty`)

- **ConPTY (Windows) and POSIX PTY (Linux/macOS) Support**:
  - Replaced synchronous pipe execution with native pseudo-terminal allocation via `portable-pty`.
  - Windows applications (`crossterm`, `ratatui`, `term-sys-monitor`, `cmd.exe`, `powershell.exe`) obtain a real console handle via Windows ConPTY (`CreatePseudoConsole`).
  - Dedicated asynchronous background reader thread streams incoming stdout/stderr byte chunks without locking the GPUI UI thread.
  - Automatic detection of interactive TUI commands (`is_tui_command`): `ir pmon`, `ir nettop`, `ir dua`, `ir fm`, `ir edit`, `ir clock`, `ir matrix`, `ir gitv`, `ir sysinfo`, `ir watch`, `ir envv`, `top`, `htop`, `vim`, `nano`, etc.
  - Automatic ANSI DSR (Device Status Report - `\x1b[6n`) cursor query handling, ensuring instant startup and unblocked execution in Windows ConPTY environments.

---

### 📟 In-Place 2D Virtual Terminal Screen Grid (`vte` + `TerminalScreenGrid`)

- **ANSI / VT100 / xterm Parser & Virtual Grid**:
  - Real-time $R \times C$ character matrix with cursor tracking, line clearing (`\x1b[K`), screen clearing (`\x1b[2J`), and cursor positioning (`\x1b[{r};{c}H`).
  - Full SGR color mapping: Solarized 16-color palette (base03, base02, base01, base00, base0, base1, base2, base3, yellow, orange, red, magenta, violet, blue, cyan, green), bold, and dim styles.
  - Live dashboards (such as `ir pmon` and `ir nettop`) refresh in-place without scrolling runaway history or flickering.
  - Automatically transfers captured screen lines into scrollable history upon process exit.

---

### ⌨️ Interactive In-Terminal Key Routing & Session Controls

- **Bidirectional Keystroke Dispatching**:
  - When an interactive session is active, keyboard input routes directly to the child process in real time:
    - Arrow keys (`↑`/`↓`/`←`/`→`) encoded as standard VT100 escape sequences (`\x1b[A`, `\x1b[B`, `\x1b[C`, `\x1b[D`).
    - Navigation keys: `Home`, `End`, `PageUp`, `PageDown`, `Tab`, `Backspace`, `Enter`, and `Space`.
    - Interactive keys: `q`, `c`, `m`, and letter keys passed directly to process stdin.
  - Pressing `q` or `Ctrl+C` cleanly aborts the active interactive session and returns to the persistent `ir ❯ ` prompt.
  - Quick action toolbar buttons: `[ir pmon]` (Live Process Monitor) and `[ir nettop]` (Live Network Monitor) alongside `[ir help]`, `[ir list]`, `[Clear]`, and `[✕]`.

---

### 🐳 Linux, Headless, and Containerized Mode Support

- **Multi-Stage Dockerfile & Docker Compose**:
  - Official multi-stage `Dockerfile`:
    - Stage 1 (`builder`): Compiles `arcade-headless` from source using `rust:1.80-slim-bookworm`.
    - Stage 2 (`runtime`): Minimal `debian:bookworm-slim` image bundling `arcade-headless` and the standalone `ir` companion CLI (v3.8).
  - `docker-compose.yml` for containerized codebase inspection, regex search/replace, and interactive shell sessions.
- **Display-Independent Headless Engine (`arcade-headless`)**:
  - Runs natively on Linux, Docker containers, and CI/CD pipelines with zero X11/Wayland/GPU dependencies.
  - Automated inspection (`inspect`), fast in-memory rope search (`search`), and atomic multi-file search-and-replace (`replace`) with `--dry-run`, `--check` CI validation, and structured `--json` output.

---

## [1.1.0] - 2026-09-23

### 🚀 Integrated Terminal, Bundled `ir` & `term-sys-monitor`, and In-App Help Release

ArcadeEdit v1.1.0 introduces an integrated terminal surface, native companion utility resolution, bundled pure-Rust `ir` CLI (v3.8) and `term-sys-monitor` hardware resource monitor by `indoctrinatedrecluse`, an in-app acrylic Help & Documentation modal, and release packaging automation.

---

### 📟 Integrated Terminal & Bundled Companion Utilities (`arcade-ui::terminal`)

- **Interactive Persistent Bottom Terminal Panel**:
  - Docked 260px console surface at the window root level (directly above the status bar), spanning the full window width, styled with Solarized Glass and specular top-edge sheen reflection.
  - Decoupled from file tabs and editor surfaces—remains persistent across all tab switches, file openings, or new tab creations.
  - Active interactive prompt (`ir ❯ `) with command history recall (`↑`/`↓`), line abort (`Ctrl+C`), and buffer clear (`Ctrl+L`).
  - Clean session exit: typing `exit` or `quit` prints `[Terminal session closed]` and automatically closes the terminal drawer.
  - Quick action toolbar buttons: `[ir help]`, `[ir list]`, `[Clear]`, and `[✕]` (close panel).
  - Quick toggle button right in the Status Bar (`📟 Terminal`) as well as keyboard shortcut `Ctrl+\`` and Command Palette.
- **Native Self-Resolution of Bundled `ir` & `term-sys-monitor`**:
  - Automatically resolves `ir` commands (`ir ...`) directly to the bundled executable (`bin/ir.exe`, application sibling, `C:\ir\ir.exe`, or standard locations) without requiring users to configure system `PATH`.
  - Dispatches standard shell commands directly within the active workspace working directory, capturing stdout, stderr, and exit codes.
- **Bundled Companion Distribution (`ir` + `term-sys-monitor`)**:
  - Bundles the pure-Rust `ir` companion CLI utility and `term-sys-monitor` (release v3.8 from `indoctrinatedrecluse/ir-cli-utility`) with ArcadeEdit release artifacts and development trees:
    - **Windows**: Bundles `ir.exe` and `term-sys-monitor-windows.exe`.
    - **Linux**: Bundles `ir` and `term-sys-monitor-linux`.

---

### 📖 In-App Help & Documentation Modal (`arcade-ui::help_modal`)

- **Translucent Acrylic Help Modal**:
  - Accessible via titlebar **`[HELP ▾]`** button, keyboard shortcut `F1`, or Command Palette.
  - Outside backdrop click-to-dismiss and `Escape` key navigation.
- **Dedicated "ir documentation" Section**:
  - Comprehensive command reference and usage guide covering file manipulation (`list`, `create`, `remove`, `copy`, `move`), text inspection (`grep`, `sort`, `diff`), system utilities (`pmon`, `nettop`, `dua`, `fastfetch`, `monitor`), and web tools (`scrape`).
  - Clear attribution to `indoctrinatedrecluse` with link to upstream repository.
- **Dedicated "About" Section**:
  - Displays App Name: `ArcadeEdit`, Version: `1.1.0`, Author: `indoctrinatedrecluse`.
  - Architectural breakdown and product design principles.
- **"Keyboard Shortcuts" Section**:
  - Complete cheat sheet for all navigation, editing, terminal, and modal shortcuts.

---

### 🔍 Interactive Find & Replace Bar (`arcade-ui::find_replace`)

- **Floating Glass Toolbar (`Ctrl+F` / `Ctrl+H`)**:
  - Integrated in-editor Find (`Ctrl+F`) and Find & Replace (`Ctrl+H`) floating glass toolbar styled with Solarized acrylic sheen.
  - Live query matching with occurrence counter (`1 of 12`, `0 of 0`, or `No matches`).
  - Next match (`↓` / `Enter`) and previous match (`↑` / `Shift+Enter`) navigation with wrap-around.
  - Multi-cursor **`[All]`** action: selects all matching occurrences across the document simultaneously as multi-cursors!
  - Case-sensitivity toggle button (`[Aa]`).
  - Single replace (`[Replace]`) and Replace All (**`[All]`**) actions with atomic undo/redo history.
  - Live visual occurrence highlighting in the gutter and across matching lines in the editor code surface.

---

### 🌳 Expandable Workspace Directory Tree (`arcade-ui::tree_view`)

- **Hierarchical Directory Tree**:
  - Replaced the flat file list with an expandable, collapsible virtualized tree structure (`tree_view::build_visible_tree`).
  - Interactive folder expansion toggles (`▸ 📁` collapsed, `▾ 📂` expanded) with indentation proportional to directory depth.
  - Automatically expands ancestor directories when files are opened.
  - Dedicated smooth scrolling container (`.id("sidebar-tree-scroll").overflow_y_scroll()`) for effortless navigation of large source trees.
  - Preserved filetype badges (`🦀`, `📄`, `⚙️`, `🔒`, `📝`) with click-to-open handlers opening files into dedicated editor tabs.
- **Binary & Executable File Safety**:
  - Automatically identifies executable and binary files (`.exe`, `.dll`, `.so`, `.bin`, `.zip`, `.wasm`, `.tar`, `.gz`, images, etc.).
  - Visually marks binary files with warning red text (`theme.warning_red: #dc322f`), a `⚠️` warning icon, and a `[bin]` badge.
  - Safety click guard: clicking a binary file refuses to load raw binary data into the editor (protecting against freezes/crashes) and instead logs a clear error notification to the integrated terminal.

---

### 📑 Multi-Tab Editor & Document Workspace (`arcade-ui::tabs`)

- **Full Multi-Tab Architecture**:
  - Independent `EditorTab` state tracking per open file (document buffer, selections, undo/redo history, syntax spans, and language).
  - Opening a new file from the workspace explorer, command palette, or `Ctrl+O` creates a dedicated editor tab instead of overwriting existing buffers.
  - Automatically deduplicates opened files—re-opening an already-opened document seamlessly focuses its existing tab.
  - Dynamic tab bar with filetype badges, dirty state indicator (`•`), and per-tab close button (`×`).
  - New tab button (`+`) to spawn empty untitled buffers.
  - Closing a tab gracefully transfers focus to adjacent tabs or resets to a blank document if the final tab is closed.

---

### 📜 Smooth Scrolling, Monospace Typography & Vibrant Syntax Highlighting (`arcade-ui`, `arcade-language`)

- **Scrollable Editor Canvas & Constrained Flex Layout**:
  - Solved the unconstrained flexbox height bug by applying `.min_h_0()` across the entire GPUI layout chain (`Main Workspace` -> `Main Editor Area` -> `Live Editor Surface` -> `#editor-canvas-scroll`).
  - Integrated `gpui::ScrollHandle` on `#editor-canvas-scroll` with automatic cursor tracking (`scroll_cursor_into_view`) on arrow navigation, Enter, Backspace, and Find matches.
  - Separated the glassy Minimap rail into a fixed right-hand column so it remains pinned to the editor edge while code lines scroll smoothly.
  - Added dedicated scroll views across all in-app modals (Help Shortcuts, `ir` Reference, About section) and the integrated terminal output buffer.
- **Persistent Bottom Terminal Dock**:
  - Fixed terminal pane disappearance on file switching: with `.min_h_0()` on the editor container, long documents are properly constrained to the viewport and no longer push the 260px docked terminal panel off-screen.
  - The integrated terminal remains docked, persistent, and accessible across all open file tabs and tab switches.
- **Accurate & Vibrant Syntax Highlighting**:
  - Fixed a critical binary search bug in `resolve_line_tokens` (`arcade-language`) where `partition_point` operated on `end_byte` over a slice sorted by `start_byte`, which previously caused files with $>16$ spans to drop all highlights. Replaced with strictly monotonic partitioning on `start_byte` with backward candidate scanning.
  - Expanded Tree-Sitter capture classifications: mapped `@boolean` to numeric constants, `@property` and `@field` to variables, `@constructor`, `@module`, and `@namespace` to types, `@method` to functions, and `@attribute` / `@label` to attributes.
  - Upgraded editor typography to a dedicated monospace font stack (`Consolas, 'Cascadia Code', 'Fira Code', 'Courier New', monospace`) for both the line number gutter and code lines.
  - Elevated variable token colors to `theme.text_bright` (warm crisp white in Solarized Dark, deep navy in Solarized Light) for striking visual pop against keywords, functions, types, and comments.
- **Instant Synchronous Syntax Highlighting**:
  - Computes highlight spans synchronously upon file opening and tab activation (<2ms), eliminating initial rendering delay.
  - Extended native grammars for `TOML` (`Cargo.toml`), `YAML`/`YML` (CI/CD workflows), and `JSON`.

---

## [1.0.0] - 2026-09-22

### 🚀 Initial Proof of Concept (POC) Release

ArcadeEdit v1.0.0 marks the first usable proof-of-concept release featuring a custom GPU-rendered desktop interface (GPUI), a deterministic headless CLI runtime, an asynchronous Tree-Sitter syntax highlighting engine, and a robust multi-cursor text editing core.

---

### 🖥️ Desktop GUI & Solarized Glass Interface (`arcade-ui`, `arcade-desktop`)

- **Solarized Minimalist Aesthetic & Dynamic Theming**:
  - Implemented custom visual theme using Solarized Dark (`#001e26` canvas) and Solarized Light (`#fdf6e3` canvas) with layered translucent acrylic glass panels and specular top-edge sheen reflections.
  - High-contrast syntax tokens: Cyan (keywords), Blue (functions), Yellow (types/headings), Green (strings/code), Orange (numbers/constants), Magenta (operators), and Violet (attributes).
  - Instant theme toggling via Command Palette and titlebar badge.
- **Interactive Keyboard Event Handling & Auto-Focus**:
  - Attached live keystroke handling on the focused window root with immediate startup auto-focus.
  - Supports character typing, `Enter` (newline insertion), `Tab` (4 spaces), and `Backspace` across all active multi-cursor selections.
  - Smooth directional navigation (`Arrow Keys`, `Home`, `End`) with `Shift` selection expansion.
  - Multi-cursor extension shortcut (`Ctrl+D`).
  - Undo and Redo keystroke shortcuts (`Ctrl+Z`, `Ctrl+Y` / `Ctrl+Shift+Z`).
- **Native OS File & Folder Pickers (`rfd`)**:
  - **"Open Document..." (`Ctrl+O` or Command Palette)**: Opens native cross-platform file picker, loads file content into active buffer, infers grammar (`LanguageId::from_path`), and updates tab title.
  - **"Open Folder..." (`Ctrl+K Ctrl+O`, Command Palette, or Explorer Header)**: Opens native folder picker, discovers project candidate files via `arcade_workspace::discover_files`, opens the primary project document, and populates the sidebar.
  - **File-Backed Persistence**: Saving (`Ctrl+S` or "Save Document") writes edits directly back to disk for opened files and updates revision status.
- **Dynamic Workspace File Explorer**:
  - Displays the active workspace directory name.
  - Dynamically lists project files with categorized icons (`🦀` Rust, `📄` Markdown/text, `⚙️` Config/TOML, `🔒` Locks), active row sheen, and click-to-open handlers.
  - Dedicated **"📂 Open Folder..."** quick action button.
- **Silent Windows Execution**:
  - Configured `#![windows_subsystem = "windows"]` on `arcade-desktop` entry point to suppress background command prompt / terminal windows when launching on Windows.
- **macOS / MAUI Floating Command Palette**:
  - Centered glassy modal overlay triggered via `Ctrl+P` or header search pill.
  - Interactive mouse event handling: every command item is clickable, executes immediately, and closes the palette.
  - Outside backdrop click-to-dismiss with event isolation on the modal card.
  - Dedicated `ESC ✕` close button in the header.
  - Dynamic keyboard search with live character filtering, Backspace correction, visual caret (`▌`), and arrow key (`↑`/`↓`) navigation with `Enter` execution.
  - Built-in commands: **"Open Document..."** (`Ctrl+O`), **"Open Folder..."** (`Ctrl+K Ctrl+O`), "Save Document", "Multi-Cursor: Add Next Occurrence", "Toggle Workspace File Explorer", "Arcade Headless: Preview Edits", "Open Integrated Terminal with `ir`", and "Toggle Solarized Sheen Contrast".
- **Responsive Workspace & Interactive Panels**:
  - Interactive tabs (`buffer.rs`, `Welcome.md`, `Cargo.toml`) with dynamic file title, active indicator sheen, dirty indicator (`•`), close buttons (`×`), and Add Tab button (`+`).
  - Interactive file explorer tree with clickable files (`buffer.rs`, `lib.rs`, `Cargo.toml`, `README.md`) to switch active tabs.
  - Collapsible file explorer pane (`Ctrl+B` or header toggle).
  - Status bar with interactive `NORMAL` mode pill (opens palette) and dirty revision badge (click to save).
- **Live Multi-Cursor Canvas**:
  - Renders line-by-line text from `Document::rope()`.
  - Active line horizontal sheen with glowing cyan accent edge.
  - Glowing cyan vertical cursors rendered at exact character coordinates.
  - Dynamic status bar displaying editing mode (`NORMAL`), document encoding, line endings, dynamic language grammar badge, and primary cursor position (`Ln X, Col Y`).

---

### 🧠 Core Editing Engine & Multi-Cursor Text Model (`arcade-core`)

- **Multi-Cursor Selections (`selection.rs`)**:
  - Implemented `Selection` and `SelectionSet` supporting arbitrary cursor counts.
  - Automatic normalization, sorting, and seamless merging of adjacent/overlapping selection ranges.
  - UTF-8 character and grapheme boundary navigation (`move_left/right/up/down/line_start/line_end/doc_start/doc_end`).
- **Atomic Transactions & Undo/Redo (`history.rs`)**:
  - `Transaction` holding paired forward and inverse text edits with selection snapshots.
  - Bounded `History` undo/redo stacks with clean/dirty tracking relative to disk state.
- **Safe Atomic Persistence (`io.rs`)**:
  - Atomic file saves using sibling temporary files and filesystem renames.
  - External disk modification detection using timestamps and byte lengths.
- **In-Memory Rope Search (`search.rs`)**:
  - Fast substring search (`find_all`, `find_next`, `find_prev`) with wrap-around support.
  - Atomic `replace_all` executing replacements in reverse offset order to preserve coordinates.

---

### 🌳 Tree-Sitter Background Syntax Highlighting (`arcade-language`)

- **Asynchronous Highlight Worker**:
  - Dedicated background worker thread (`arcade-highlight-worker`) offloading AST parsing from the UI thread, ensuring consistent 60/120 FPS rendering.
  - Non-blocking channel polling with automatic draining of intermediate backlog requests during rapid typing bursts.
- **Tree-Sitter Rust Grammar**:
  - Integrated `tree-sitter = "0.25"` and `tree-sitter-rust = "0.24"` compiling natively with MSVC in ~1s.
  - Maps standard Tree-Sitter AST captures to `HighlightKind`.
- **Markdown Scanner**:
  - Fast document scanner highlighting headings (`#`), fenced code blocks, blockquotes, bullet lists, and inline code.
- **Deterministic Line Token Resolution**:
  - Slices document highlight spans into line-level `LineToken`s with priority-based conflict resolution and 100% UTF-8 boundary safety.

---

### ⚙️ Deterministic Headless CLI & Protocol (`arcade-headless`, `arcade-protocol`, `arcade-workspace`)

- **Versioned JSON Protocol (v1)**:
  - Strongly-typed, versioned response envelopes (`Response<T>`) for CI pipelines, scripts, and external tools.
  - Schemas for `InspectResult`, `SearchResult`, and `ReplaceResult`.
- **Headless CLI Subcommands**:
  - `arcade inspect <file> [--json]`: Document metrics, line counts, character counts, encoding, line ending conventions, and inferred language.
  - `arcade search <query> [paths...] [--json]`: Recursive grep-like search across files with line, column, byte offset, and line text.
  - `arcade replace <query> <replacement> [paths...] [--dry-run] [--check] [--json]`:
    - Atomic multi-file replacement.
    - `--dry-run`: Previews replacement counts without touching disk.
    - `--check`: CI lint mode exiting with 0 if clean and 1 if matches exist.
- **Workspace Services (`arcade-workspace`)**:
  - Recursive file discovery respecting huge-file thresholds (50MB) and automatically skipping `.git`, `target`, `node_modules`, and IDE directories.

---

### 🛠️ Developer Experience & Tooling (`tools/`)

- **Cross-Platform Run Scripts**:
  - Added [`tools/run.ps1`](tools/run.ps1) and [`tools/run.sh`](tools/run.sh) for launching the desktop editor.
  - Added [`tools/run_headless.ps1`](tools/run_headless.ps1) and [`tools/run_headless.sh`](tools/run_headless.sh) for running CLI automation commands.
- **Build Optimization**:
  - Optimized Windows MSVC linking by stripping debug info from external dependencies (`debug = 0`), reducing link times from >2 minutes down to sub-second.
  - Disabled redundant test harness generation on binary crates (`[[bin]] test = false`).
  - Automated test suite: **30 unit tests passing in <5 seconds**.

