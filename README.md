# ✨ ArcadeEdit

> A fast, native text editor with a custom GPU-rendered desktop experience and a first-class headless core.

ArcadeEdit is a features-packed editor that stays an **editor**, not an IDE. It aims for the speed, focus, and keyboard-first feel of tools such as Zed, Lapce, and Notepad++—without inheriting their product UI or IDE-first assumptions.

## 🎯 Project brief

ArcadeEdit is built in Rust around one shared editing engine. That engine powers two equal product modes:

- 🖥️ **Rendered mode** — a native desktop editor built with GPUI and an original ArcadeEdit interface.
- ⚙️ **Headless mode** — a deterministic CLI/service runtime for local automation, SSH, CI, and containers; it has no display or GPU requirement.

The desktop application is the rich visual home for writing and editing. Headless mode exposes the same document, search, syntax, and transformation capabilities to scripts and isolated environments.

## 🧭 Product principles

- ✍️ **Text first.** Editing quality, responsiveness, and useful text workflows come before IDE features.
- ⚡ **Fast by design.** Large-file performance comes from a rope document model, incremental work, virtualized layout, and bounded caches—not just GPU drawing.
- 🎨 **Our own interface.** GPUI supplies the application and GPU rendering framework; ArcadeEdit owns all visible UI, interaction patterns, and visual language.
- 🔁 **One core, many surfaces.** Desktop and headless modes use one versioned editor model and command protocol.
- 🔒 **Safe automation.** Preview, dry-run, diff, and explicit write controls are core headless behaviors.
- 🧩 **Extensible later.** The core exposes stable command boundaries first; a public extensions API follows only after those boundaries have earned their shape.

## 🏗️ Architecture

```text
                         🧠 arcade-core
       documents · transactions · selections · undo/redo · search
           Ropey text storage · Tree-sitter syntax · workspace model
                                  │
                     📦 arcade-protocol
       versioned commands · edits · diagnostics · diffs · JSON results
                         ╱                    ╲
          🖥️ arcade-desktop                  ⚙️ arcade-headless
         GPUI custom frontend              CLI / service / containers
         GPU-rendered editor               no display or GPU required
```

`arcade-core` must not depend on GPUI, a windowing system, a terminal emulator, or a container runtime. GPUI belongs exclusively to the desktop surface.

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
- Cursor movement and user-visible selection use character/grapheme-aware operations.
- Every edit produces a precise before/after `TextEdit`, allowing the parser, undo history, views, and headless clients to stay synchronized.
- The layout engine renders only visible visual lines plus a small overscan region.
- Huge-file mode intentionally limits expensive work such as syntax parsing, wrapping, and heavyweight indexing.

## 🖥️ Rendered mode

The desktop app will be a custom GPUI application—not a reskin of Zed or a dependency on Zed CoreUI.

### First editor experience

- 📄 Fast open, edit, save, reload, and external-change handling
- 🗂️ Tabs, split panes, sessions, and workspace folders
- ↕️ Smooth scrolling, line numbers, soft wrap, minimap consideration, and large-file mode
- ✨ Multi-cursor editing, selections, undo/redo, find/replace, and command palette
- 🎨 Themes, typography, keymaps, and an accessible custom design system
- 🌳 Tree-sitter syntax highlighting, structural selection, folds, and outlines
- ⌨️ An integrated terminal surface for deliberate command-line workflows

## ⚙️ Headless and containerized mode

Headless mode is a product capability, not a test-only adaptation of the GUI. `arcade-headless` will use `arcade-core` directly and must work without GPUI, a display server, or GPU access.

### Planned capabilities

- 🔍 Deterministic open, search, structural query, transform, save, and diff workflows
- 🧪 `--check` / dry-run and machine-readable JSON output for CI and scripts
- 👀 Watch mode for repeatable local workflows
- 🐳 An unprivileged container image with an explicitly mounted workspace
- 🔐 Read-only by default; writes require an explicit opt-in
- 🌐 A future local/remote service only after the CLI command and result protocol is stable

Illustrative command shape:

```sh
arcade edit src --find "oldName" --replace "newName" --preview
arcade query README.md --syntax markdown --select heading:Installation
arcade apply recipe.toml --check --json
```

## 🛠️ Integrated `ir` command utility

ArcadeEdit will ship alongside [`ir-cli-utility`](https://github.com/indoctrinatedrecluse/ir-cli-utility), the pure-Rust `ir` command-line utility. `ir` provides cross-platform filesystem and shell-style operations through native OS syscalls rather than aliases or aggregation of Windows built-ins.

The built-in terminal will make `ir` immediately available as a companion workflow for file operations, search, inspection, archiving, and other command-line tasks. ArcadeEdit will also include an attributable, version-pinned copy of the relevant `ir` documentation in its in-app help.

### 📚 Integration commitments

- Bundle a tested `ir` binary for each supported ArcadeEdit platform.
- Surface the bundled version and upstream project link in About/Help.
- Include the selected upstream documentation with its required attribution and license notices.
- Keep the terminal a real terminal: `ir` is available to the user, but it is not a hidden replacement for normal editor file operations.
- Resolve exact bundle version, distribution terms, updates, and attribution before packaging.

## ✅ Scope

### 🟢 Foundation / first usable release

- Shared document core, transaction history, and file I/O
- GPUI desktop shell and custom editor view
- Tabs, text editing, multi-cursor, undo/redo, search/replace, themes, and settings
- Tree-sitter highlighting for an initial curated language set
- Headless CLI with preview, diff, JSON output, and safe write behavior
- Integrated terminal and packaged `ir` utility/documentation

### 🟡 Later, after the core is proven

- Workspace-wide search and richer session restoration
- Split panes, folds, outlines, and advanced text transforms
- Container images and a polished automation workflow
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

## 🚧 Status (v1.1.0 Release)

**Interactive Desktop & Versioned Headless CLI Active.** The project currently features:
- 🎨 **Solarized Glass GUI** (`arcade-desktop` / `arcade-ui`): A GPU-rendered desktop interface styled with a Solarized Dark minimalist palette, translucent acrylic glass sheen layers, macOS/MAUI-inspired floating command palette (`Ctrl+P`), live multi-cursor keyboard editing, vertical cursor glow, and dynamic language/revision status bar.
- 📟 **Integrated Terminal & Bundled Companion Utilities** (`arcade-ui::terminal`): Dockable bottom terminal panel with command history, prompt execution, quick actions (`ir help`, `ir list`), and automatic self-resolution of the bundled `ir` and `term-sys-monitor` executables without requiring PATH configuration.
- 📖 **In-App Help & Documentation Modal** (`arcade-ui::help_modal`): Accessible via titlebar `[HELP ▾]` or `F1`, featuring full `ir` command reference documentation, keyboard shortcuts, and About section (`ArcadeEdit` v1.1.0 by `indoctrinatedrecluse`).
- 🌳 **Tree-Sitter Background Syntax Worker** (`arcade-language`): Non-blocking AST-based parsing and query capture execution on a dedicated background worker thread (`tree-sitter = "0.25"` and `tree-sitter-rust = "0.24"`), with markdown scanner and UTF-8 safe line token resolution.
- ⚙️ **Deterministic Headless CLI** (`arcade-headless`): A lightweight CLI core free of GPUI or display dependencies, communicating over `arcade-protocol` (v1) with `inspect`, `search`, and `replace` subcommands supporting `--json`, `--dry-run`, and `--check` modes.
- 🧠 **Multi-Cursor Core Engine** (`arcade-core`): Ropey-backed document buffer with byte offsets, revision tracking, atomic transactions, bounded undo/redo history, atomic file persistence, and in-memory rope search.
- 🗂️ **Workspace Services** (`arcade-workspace`): Recursive file discovery ignoring VCS/build artifacts (`.git`, `target`, `node_modules`).

## ▶️ Build and run

### 📋 Prerequisites

- Stable Rust **1.82+** (`rustup default stable`).
- **Rendered GUI mode**: Windows (DirectX/Direct3D 11 via GPUI), macOS (Metal), or Linux (Vulkan/Wayland/X11).
- **Headless mode**: Linux, macOS, or Windows (no display server, GPU, or graphical driver required).

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

# Atomic search-and-replace with dry-run and CI check modes
cargo run -p arcade-headless -- replace "old_api" "new_api" src/ --dry-run
cargo run -p arcade-headless -- replace "deprecated_fn" "new_fn" src/ --check
```

---

### 🐳 Containerized headless mode

Run ArcadeEdit headless inside an isolated, unprivileged container without a display server or GPU:

```sh
# Build the headless container image
docker build -t arcade-headless -f - . <<EOF
FROM rust:1.82-slim AS builder
WORKDIR /app
COPY . .
RUN cargo build --release -p arcade-headless

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/arcade-headless /usr/local/bin/arcade-headless
ENTRYPOINT ["arcade-headless"]
EOF

# Run commands against a mounted workspace directory
docker run --rm -v "$(pwd):/workspace" -w /workspace arcade-headless inspect Cargo.toml --json
```

---

### 🧪 Fast verification & testing

To verify the entire workspace with sub-second execution times:

```sh
# Fast typecheck across all workspace crates
cargo check --workspace

# Run all unit tests across the workspace (25 tests in <5s)
cargo test --workspace --lib
```
