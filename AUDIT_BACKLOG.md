# ArcadeEdit Audit and Implementation Backlog

This is the working record for the desktop editor, headless CLI, container runtime, and integrated `ir` terminal. Placeholder behavior is being implemented when it has a useful in-scope meaning; removal is reserved for decorative or misleading behavior without a meaningful implementation.

## Phase 1 — Data safety

- [x] Headless `replace` previews by default and requires `--write` to modify files.
- [x] Report discovery, load, and save errors and use exit status 2 for partial failures.
- [x] Make multi-file replace report exact committed files and structured per-file failures; atomicity remains per file, with no workspace rollback.
- [x] Use collision-resistant, exclusively created adjacent temp files for saves; preserve permissions and clean up after failures.
- [x] Route desktop saves through shared persistence, surface failures, and mark clean only after success.
- [x] Add Save As for untitled buffers and dirty-tab Save/Discard/Cancel prompts.
- [ ] Add reload/compare/overwrite choices for external edits; current save protection blocks overwrite using timestamp, size, and a content fingerprint.
- [x] Run the container as a non-root user and document bind-mount ownership.
- [ ] Close dirty tabs safely on application/window exit as well as per-tab close.

## Phase 2 — Desktop editor

- [x] Remove the sample sidebar tree; show workspace files only after opening a folder.
- [x] Implement the former fake preview action as a read-only active-document preview.
- [x] Replace static workspace/mode/newline/status labels with live state or remove unsupported labels.
- [x] Remove the decorative empty minimap rail.
- [x] Align documented save shortcuts and terminal row counts with actual behavior.
- [x] Preserve the loaded line-ending style when inserting new lines; report the active tab's style.
- [x] Add system clipboard copy, cut, and paste for selections, including multicursor insertion.
- [x] Preserve the primary selection when normalized multi-cursor selections are reordered or merged.
- [x] Move file highlighting onto the background worker and reject files over the current 50 MiB limit.
- [x] Surface workspace scan and preferred-file open failures in the terminal error channel.
- [ ] Render visible lines only; current editor constructs every line, so large documents below the limit can still be expensive.
- [ ] Add grapheme-cluster-aware cursor movement/deletion; core offsets currently move by Unicode scalar.
- [ ] Add a reload command and richer external-file conflict resolution.
- [ ] Improve file-picker cancellation/error reporting and track dirty state on application exit.
- [ ] Implement split panes, session restore, soft wrap, folds, outlines, settings/keymaps, and typography only as separately scoped features.

## Phase 3 — Headless automation

- [x] Honor ignore files, avoid following directory symlinks, and return discovery errors.
- [x] Enforce and report the current 50 MiB discovery/open limit.
- [x] Find queries across line boundaries and keep Unicode byte offsets distinct from character columns.
- [x] Add `--version`, help, safe write flags, and argument behavior checks.
- [ ] Make `arcade-protocol::Command` the actual CLI dispatch boundary; replace JSON replace output now includes collected failures and committed file summaries.
- [ ] Decide/document overlapping matches, multiline result line mapping, binary files, and invalid UTF-8 policy.
- [ ] Add configurable size/depth limits and report paths omitted by depth limits.
- [ ] Add transactional multi-file writes and machine-readable per-file partial failure details.
- [x] Prove headless builds without desktop/GPUI dependencies locally and in CI.

## Phase 4 — Container and delivery

- [x] Pin local Rust, workspace metadata, CI, Docker builder, and docs to Rust 1.98.1; 1.82 could not resolve the locked GPUI/Zed revision.
- [x] Build container as UID 10001 by default and document writable bind mounts.
- [x] Add CI jobs for Linux headless tests, a Docker preview/write/inspect smoke test, and full Windows workspace/ConPTY tests.
- [ ] Observe the new CI jobs pass and address platform-only failures; Docker is unavailable on the current workstation. Release workflow execution will provide remote evidence.
- [ ] Verify downloaded `ir` artifacts with an independently trusted checksum/signature; current GitHub release downloads are version-addressed but not independently integrity-verified.
- [ ] Verify every expected release artifact exists and include upstream licenses/attribution.
- [ ] Publish supported-platform coverage and smoke-test packaged binaries.
- [ ] Confirm the bundled `ir` version/license/help claims against upstream source and make in-app attribution complete.

## Phase 5 — Terminal and TUI reliability

- [x] Parse quoted arguments and Windows paths; recognize the TUI actions and aliases reported by bundled `ir help`.
- [x] Route printable/Unicode text, navigation, paging, delete, function keys, control chords, and escape to active PTYs.
- [x] Send first Ctrl+C as ETX and reserve a quick second Ctrl+C for explicit force termination.
- [x] Bound PTY reader queues, terminal scrollback, and command history; run ordinary shell commands off the UI thread.
- [x] Test bundled `ir pmon` (`q`), `ir nettop` (`Esc`), and `ir edit` typing/navigation/interrupt through ConPTY.
- [ ] Diagnose bundled `ir edit` Ctrl+S / Ctrl+Q behavior: standard control bytes and Kitty CSI-u were tried but did not save/quit in ConPTY; the integration test currently verifies edit typing/navigation and explicit interrupt instead.
- [ ] Derive PTY dimensions from the actual rendered viewport and propagate window resize events; current launch sizes are fixed at 120×20 or 120×30.
- [ ] Test navigation/quit conventions for the broader set of bundled TUIs, including `fm`, `dua`, `watch`, and aliases.
- [ ] Show exit status and distinguish normal completion, startup failure, IO failure, interrupt, and forced termination in the UI.
- [ ] Verify child processes and reader/worker threads are reaped on panel/window closure and application shutdown.
- [ ] Bound ordinary shell command output capture and concurrent command workers; scrollback is bounded after completion, but `Command::output()` currently buffers full process output first.

## Phase 6 — Careful cleanup and verification

- [x] Format touched Rust sources; full workspace tests, Clippy, headless smoke checks, and Windows desktop/headless release builds pass.
- [ ] Review and remove redundant active-tab state and dead compatibility code only after behavior coverage.
- [ ] Review the final diff for accidental behavior changes and observe CI on Linux/container targets; local Linux cross-check lacks a Linux C compiler.

## Audit findings to retain

- The editor view currently materializes all document lines on every render; the 50 MiB cap limits but does not solve this scaling issue.
- Desktop workspace scan honors ignore files; the visible tree is now built from discovered paths to avoid showing ignored entries or following ignored/out-of-workspace directory links.
- Multi-file `replace` is atomic per file but not across a workspace. Failures are now reported, but earlier successful files remain committed.
- External-change detection now compares modification timestamp, length, and content fingerprint; the fingerprint is a change detector, not a cryptographic authenticity mechanism.
- The integrated terminal uses fixed PTY dimensions, and one bundled editor's save/quit controls did not respond in ConPTY; keep this visible as a functional issue.
- Ordinary command output is capped in retained UI history only after `Command::output()` returns; a command that emits a very large stream can still use unbounded worker memory.
- Hardcoded palette colors, ignored directory names, and the 50 MiB cap are intentional design/configuration defaults. Welcome copy and bundled `ir` names are intentional onboarding/integration content, not dynamic workspace data.
- The repository includes a bundled Windows `bin/ir.exe` and monitor binary. Docker and Linux release fetch companion artifacts from GitHub without an independent checksum/signature verification step.
- The Windows desktop auto-checker runs at launch; the in-app installer currently supports Windows x86_64 only. Other desktop platforms can use manual release downloads.
- Go to Line (`Ctrl+G`) supports a one-based line and optional one-based column.
