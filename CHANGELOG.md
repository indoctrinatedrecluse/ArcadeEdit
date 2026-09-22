# Changelog

All notable changes to the ArcadeEdit project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
- **macOS / MAUI Floating Command Palette**:
  - Centered glassy modal overlay triggered via `Ctrl+P` or header search pill.
  - Interactive mouse event handling: every command item is clickable, executes immediately, and closes the palette.
  - Outside backdrop click-to-dismiss with event isolation on the modal card.
  - Dedicated `ESC ✕` close button in the header.
  - Dynamic keyboard search with live character filtering, Backspace correction, visual caret (`▌`), and arrow key (`↑`/`↓`) navigation with `Enter` execution.
  - Built-in commands: **"Open Folder..."** (`Ctrl+K Ctrl+O`), "Open Document...", "Save Document", "Multi-Cursor: Add Next Occurrence", "Toggle Workspace File Explorer", "Arcade Headless: Preview Edits", "Open Integrated Terminal with `ir`", and "Toggle Solarized Sheen Contrast".
- **Responsive Workspace & Interactive Panels**:
  - Interactive tabs (`buffer.rs`, `Welcome.md`, `Cargo.toml`) with active indicator sheen, close buttons (`×`), and Add Tab button (`+`).
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
  - Automated test suite: **25 unit tests passing in <5 seconds**.

