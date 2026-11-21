# ✨ ArcadeEdit

> A fast, native text editor with a custom GPU-rendered desktop experience and a first-class headless core.

ArcadeEdit is a features-packed editor that stays an **editor**, not an IDE. It aims for the speed, focus, and keyboard-first feel of tools such as Zed, Lapce, and Notepad++—without inheriting their product UI or IDE-first assumptions.

## 🎯 Project brief

ArcadeEdit is built in Rust around one shared editing engine. That engine powers two equal product modes:

- 🖥️ **Rendered mode** — a native desktop editor built with GPUI and an original ArcadeEdit interface.
- ⚙️ **Headless mode** — a deterministic CLI for local automation, SSH, CI, and containers; it has no display or GPU requirement.

The desktop application is the rich visual home for writing and editing. Headless mode exposes the same document, search, syntax, and transformation capabilities to scripts and isolated environments.

## 🧭 Product principles

- ✍️ **Text first.** Editing quality, responsiveness, and useful text workflows come before IDE features.
- ⚡ **Fast by design.** The rope model and background highlighting help keep editing responsive; visible-line virtualization is still planned.
- 🎨 **Our own interface.** GPUI supplies the application and GPU rendering framework; ArcadeEdit owns all visible UI, interaction patterns, and visual language.
- 🔁 **One core, many surfaces.** Desktop and headless modes share the document/search core and versioned JSON result envelopes. The protocol `Command` enum is not yet the CLI dispatch boundary.
- 🔒 **Safe automation.** Preview, dry-run, and explicit write controls are core headless behaviors; diff output remains planned.
- 🧩 **Extensible later.** The typed command protocol is defined, but CLI dispatch is not yet routed through it; a public extensions API remains future work.

## 🏗️ Architecture

```text
                         🧠 arcade-core
       documents · transactions · selections · undo/redo · search
                  Ropey storage · UTF-8 byte offsets
                                  │
              🌳 arcade-language · 📁 arcade-workspace
                syntax services · ignore-aware discovery
                                  │
                     📦 arcade-protocol
            versioned command/result types · JSON envelopes
                         ╱                    ╲
          🖥️ arcade-desktop                  ⚙️ arcade-headless
         GPUI custom frontend              CLI / containers
         GPU-rendered editor               no display or GPU required
```

`arcade-core` must not depend on GPUI, a windowing system, a terminal emulator, or a container runtime. GPUI belongs exclusively to the desktop surface. `arcade-headless` currently depends on the core, language, protocol, and workspace crates, and its dependency graph is checked to exclude GPUI.

## 🧱 Planned stack

| Area | Choice | Why |
| --- | --- | --- |
| 🦀 Language | Stable Rust + Cargo workspace | Native performance, predictable deployment, shared core. |
| 🎨 Desktop UI | GPUI | GPU-backed rendering and native desktop integration, with a fully custom interface. |
| 📜 Document model | `ropey::Rope` (UTF-8) | Efficient large-file edits and byte/character/line conversion. |
| ✂️ Editing model | ArcadeEdit transactions | Multi-cursor edits, undo/redo, previews, and headless reproducibility. |
| 🌳 Syntax | Tree-sitter + highlight queries | Incremental parsing, syntax highlighting, structural selection, and folding. |
| 🧵 Background work | Tokio or a small dedicated worker pool | Parsing, search, file I/O, and workspace scanning away from the UI thread. |
| 🔎 Files and search | `notify`, `ignore`, `walkdir`, and a fast search backend | Workspace discovery, file watching, and project search. |
| ⚙️ Settings | `serde` + TOML | Portable, versioned settings, themes, and session data. |
| 📦 Delivery | Native desktop bundles + a headless binary/container image | Desktop use, CI, SSH, and isolated automation. |

### 📐 Text and rendering rules

- Tree-sitter and syntax spans use **UTF-8 byte offsets**.
- Cursor movement currently advances by Unicode scalar values; grapheme-aware editing remains planned.
- Every edit produces a precise before/after `TextEdit`, allowing the parser, undo history, views, and headless clients to stay synchronized.
- The editor currently renders every line in an opened document. A 50 MiB discovery/open limit is in place; visible-line virtualization and a richer huge-file mode remain planned.

## 🖥️ Rendered mode

The desktop app is a custom GPUI application, not a reskin of Zed or a dependency on Zed CoreUI.

### Implemented editor workflows

- Open, edit, save, and Save As; changed-on-disk files are protected from a straightforward stale-buffer overwrite.
- Tabs with dirty close prompts, workspace folders and ignore-aware explorer, line numbers, and dark/light themes.
- Multi-cursor edits, system clipboard copy/cut/paste, undo/redo, find/replace, and command palette.
- Rust/Markdown Tree-Sitter highlighting and JSON/TOML/YAML scanners on a background worker.
- Read-only current-document preview and an integrated PTY terminal for bundled `ir` commands.

Reload/compare conflict choices, grapheme-aware movement, visible-line virtualization, split panes, session restore, soft wrap, folds, outlines, settings/keymaps, and typography remain future work.

## ⚙️ Headless and containerized mode

Headless mode is a product capability, not a test-only adaptation of the GUI. `arcade-headless` uses `arcade-core` directly and works without GPUI, a display server, or GPU access.

### Current capabilities

- 🔍 `inspect`, `search`, and workspace `replace` commands
- 🧪 `--check`, preview-by-default replacement, explicit `--write`, and versioned JSON results
- 🐳 An unprivileged container image with a mounted workspace workflow

The CLI does not currently implement structural queries, diff output, watch mode, or a local/remote service. These remain future work. Supported command examples:

```sh
arcade-headless inspect README.md --json
arcade-headless search "oldName" src/ --json
arcade-headless replace "oldName" "newName" src/ --dry-run
arcade-headless replace "oldName" "newName" src/ --write
```

## 🛠️ Integrated `ir` command utility

ArcadeEdit bundles and resolves [`ir-cli-utility`](https://github.com/indoctrinatedrecluse/ir-cli-utility), a Rust command-line utility. The companion release is separately versioned; packaged artifacts are not yet checksum-verified by this repository.

The built-in PTY terminal makes interactive and ordinary `ir` commands available. Its interactive-command map is based on bundled `ir help`; broader TUI navigation and editor save/quit behavior still need integration coverage.

### 📚 Integration commitments

- Bundle the `ir` artifact for each platform included in the current release workflow.
- Keep the upstream project link in About/Help; verify its version/license text and add complete notices before broad redistribution.
- Keep the terminal a real terminal: `ir` is available to the user, but it is not a hidden replacement for normal editor file operations.
- Resolve exact bundle version, distribution terms, updates, and attribution before packaging.

## ✅ Scope

### 🟢 Foundation / first usable release

- Shared document core, transaction history, and file I/O
- GPUI desktop shell and custom editor view
- Tabs, text editing, multi-cursor, undo/redo, search/replace, and dark/light themes
- Tree-sitter highlighting for an initial curated language set
- Headless CLI with preview, JSON output, and explicit safe write behavior
- Integrated terminal and packaged `ir` utility/documentation

### 🟡 Later, after the core is proven

- Workspace-wide search and richer session restoration
- Split panes, folds, outlines, and advanced text transforms
- More container hardening and broader platform automation coverage
- Optional local/remote headless service
- Extension API and sandboxed extension host

### 🔴 Explicitly out of the first release

- Debugger, build/task orchestration, and IDE project models
- Git client UI
- LSP as a required dependency or product center of gravity
- Collaboration, AI features, and a public extension marketplace

## 🗺️ Delivery sequence

1. ✅ 🧠 Scaffold the Cargo workspace and make `arcade-core` independently testable.
2. ✅ ✍️ Implement Ropey-backed documents, transactions, selections, undo/redo, and file persistence.
3. ✅ 🎨 Add the GPUI desktop shell, multi-cursor editing, and live Solarized Glass canvas.
4. ✅ 🌳 Add Tree-sitter AST parsing/highlighting on background workers (`arcade-language`).
5. ✅ ⚙️ Ship the first `arcade-headless` commands (`inspect`, `search`, `replace`) with versioned JSON protocol.
6. ✅ 🛠️ Add the integrated terminal and a versioned `ir` bundle/documentation pipeline.
7. ⏳ 🐳 Package, benchmark, harden container behavior, and define huge-file limits.

## 🚧 Status (v1.3.0 Release)

**Interactive Desktop, ConPTY Terminal & Versioned Headless CLI Active.** The project currently features:
- 🎨 **Solarized Glass GUI** (`arcade-desktop` / `arcade-ui`): A GPU-rendered desktop interface styled with a Solarized Dark minimalist palette, translucent acrylic glass sheen layers, macOS/MAUI-inspired floating command palette (`Ctrl+P`), live multi-cursor keyboard editing, vertical cursor glow, and dynamic language/revision status bar.
- 📟 **Integrated Terminal & Bundled Companion Utilities** (`arcade-ui::terminal`): Persistent bottom terminal panel powered by native ConPTY/PTY with 256-color and 24-bit Truecolor ANSI screen grid, reverse video, DEC cursor tracking, dynamic height maximization, command history, prompt execution, quick actions (`ir help`, `ir list`), and automatic self-resolution of the bundled `ir` and `term-sys-monitor` executables without requiring PATH configuration.
- 📖 **In-App Help & Documentation Modal** (`arcade-ui::help_modal`): Accessible via titlebar `[HELP ▾]` or `F1`, featuring full `ir` command reference documentation, keyboard shortcuts, and About section (`ArcadeEdit` v1.3.0 by `indoctrinatedrecluse`).
- 🔄 **Release updates**: Desktop checks for a newer stable release at launch and from Help; Windows x86_64 can download, verify, and stage the update for restart.
- 🎯 **Go to Line**: Use `Ctrl+G` or the command palette to navigate to a one-based line and optional column.
- 🌳 **Tree-Sitter Background Syntax Worker** (`arcade-language`): Non-blocking AST-based parsing and query capture execution on a dedicated background worker thread (`tree-sitter = "0.25"` and `tree-sitter-rust = "0.24"`), with markdown scanner and UTF-8 safe line token resolution.
- ⚙️ **Deterministic Headless CLI** (`arcade-headless`): A lightweight CLI core free of GPUI or display dependencies, communicating over `arcade-protocol` (v1) with `inspect`, `search`, and `replace` subcommands supporting `--json`, `--dry-run`, and `--check` modes.
- 🧠 **Multi-Cursor Core Engine** (`arcade-core`): Ropey-backed document buffer with byte offsets, revision tracking, atomic transactions, bounded undo/redo history, atomic file persistence, and in-memory rope search.
- 🗂️ **Workspace Services** (`arcade-workspace`): Recursive file discovery ignoring VCS/build artifacts (`.git`, `target`, `node_modules`).

## ▶️ Build and run

### 📋 Prerequisites

- Rust **1.98.1**, pinned by `rust-toolchain.toml` and shared by local builds, CI, and the headless container builder.
- **Rendered GUI mode**: Windows (DirectX/Direct3D 11 via GPUI) is the locally verified desktop target. macOS (Metal) and Linux (Vulkan/Wayland/X11) builds are not yet covered by this repository's release/CI matrix.
- **Headless mode**: Linux and Windows are covered by CI/release builds; macOS should work with Rust but is not currently in the release matrix. No display server, GPU, or graphical driver is required.

Headless replacement is preview-only unless `--write` is supplied. Docker runs as UID 10001 by default; for writable bind mounts, run with the host user's UID/GID (for example, `docker run --user "$(id -u):$(id -g)" ...`) or grant that container user write access to the mounted directory.

---

### 🖥️ Run rendered GUI mode

Launch the GPU-rendered desktop application with the Solarized Glass interface:

```sh
# Using the helper scripts in tools/
./tools/run.ps1        # Windows PowerShell
./tools/run.sh         # Linux / macOS Bash

# Or directly via cargo
cargo run -p arcade-desktop
```

#### GUI POC Features:
- **Live Multi-Cursor Editing**: Type anywhere on the canvas, navigate with Arrow keys / Home / End, hold `Shift` to extend selections, press `Backspace` / `Enter` / `Tab`, and undo/redo with `Ctrl+Z` / `Ctrl+Y`.
- **macOS / MAUI Style Command Palette**: Press `Ctrl+P` or click the header search pill to toggle the floating glass command palette with categorized commands ("Open Document...", "Open Folder...", "Save Document", etc.).
- **Tree-Sitter Highlighting**: Real-time syntax coloring for Rust and Markdown executed asynchronously on a background worker thread.
- **Glassy Sheen Aesthetic**: Translucent acrylic paneling with specular top-edge sheen highlights, drop shadows, and soft glowing accents.
- **Tabs & Workspace Explorer**: Interactive tab switching (`buffer.rs`, `Welcome.md`, `Cargo.toml`) and hierarchical project tree.

---

### ⚙️ Run headless CLI mode

Headless mode compiles independently with zero graphics or windowing dependencies:

```sh
# Using the helper scripts in tools/
./tools/run_headless.ps1 --help
./tools/run_headless.sh --version

# Inspect file metrics, line counts, encoding, and language
cargo run -p arcade-headless -- inspect Cargo.toml
cargo run -p arcade-headless -- inspect Cargo.toml --json

# Search for patterns across files and directories
cargo run -p arcade-headless -- search "TODO" crates/
cargo run -p arcade-headless -- search "fn main" crates/ --json

# Search-and-replace (atomic per file) with preview, write, and CI check modes
cargo run -p arcade-headless -- replace "old_api" "new_api" src/
cargo run -p arcade-headless -- replace "old_api" "new_api" src/ --write
cargo run -p arcade-headless -- replace "deprecated_fn" "new_fn" src/ --check
```

---

### 🐳 Containerized headless & Linux mode

Run ArcadeEdit headless and companion utilities inside an isolated container without a display server or GPU:

```sh
# Build the official multi-stage container image
docker build -t arcade-edit .

# Inspect files and output structured JSON
docker run --rm -v "$(pwd):/workspace" arcade-edit inspect Cargo.toml --json

# Run codebase search and replacements via Docker Compose
docker compose run --rm arcade search "ArcadeEdit"
docker compose run --rm arcade replace "old_api" "new_api" --dry-run
docker compose run --rm --user "$(id -u):$(id -g)" arcade replace "old_api" "new_api" --write

# Run interactive companion shell in container
docker compose run --rm term -c "ir pmon"
```

---

### 🧪 Verification & testing

Run these from the workspace root:

```sh
# Fast typecheck across all workspace crates
cargo check --workspace

# Run all unit tests across the workspace
cargo test --workspace --lib
```

