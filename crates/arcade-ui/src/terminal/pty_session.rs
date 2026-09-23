//! Cross-platform Pseudo-Terminal (PTY) session management.
//!
//! Uses `portable-pty` to allocate a native pseudo-terminal (Windows ConPTY / POSIX openpty)
//! with background streaming readers, non-blocking I/O, and bidirectional keystroke transmission.

use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtyPair, PtySize};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;

/// An active interactive session connected to a child process via a Pseudo-Terminal (PTY).
pub struct PtySession {
    /// Command line title for this session (e.g. "ir pmon").
    pub command_title: String,
    /// Handle to write raw input bytes directly to the PTY master.
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    /// Channel receiver for incoming output byte chunks read asynchronously.
    receiver: Receiver<Vec<u8>>,
    /// Active child process handle.
    child: Box<dyn portable_pty::Child + Send + Sync>,
    /// Master PTY handle (kept alive for terminal lifecycle and resizing).
    master: Box<dyn MasterPty + Send>,
}

impl PtySession {
    /// Spawns an executable with arguments inside a native Pseudo-Terminal (ConPTY / POSIX PTY).
    pub fn spawn(
        executable: &Path,
        args: &[String],
        working_dir: &Path,
        cols: u16,
        rows: u16,
    ) -> Result<Self, String> {
        let pty_system = native_pty_system();
        let pair: PtyPair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| format!("Failed to open native PTY: {e}"))?;

        let mut cmd = CommandBuilder::new(executable);
        cmd.args(args);
        cmd.cwd(working_dir);

        // Clone reader and take writer before spawning child so no early output is missed
        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| format!("Failed to clone PTY reader: {e}"))?;

        let writer = pair
            .master
            .take_writer()
            .map_err(|e| format!("Failed to take PTY writer: {e}"))?;

        let (tx, rx) = channel();

        // Background reader thread: continuously pumps stdout/stderr bytes into the channel
        thread::Builder::new()
            .name("arcade-pty-reader".into())
            .spawn(move || {
                let mut buf = [0u8; 8192];
                while let Ok(n) = reader.read(&mut buf) {
                    if n == 0 {
                        break;
                    }
                    if tx.send(buf[..n].to_vec()).is_err() {
                        break;
                    }
                }
            })
            .map_err(|e| format!("Failed to spawn PTY reader thread: {e}"))?;

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| format!("Failed to spawn command in PTY: {e}"))?;

        let command_title = if args.is_empty() {
            executable
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("command")
                .to_string()
        } else {
            format!(
                "{} {}",
                executable.file_name().and_then(|n| n.to_str()).unwrap_or("cmd"),
                args.join(" ")
            )
        };

        Ok(Self {
            command_title,
            writer: Arc::new(Mutex::new(writer)),
            receiver: rx,
            child,
            master: pair.master,
        })
    }

    /// Sends raw bytes into the running process's standard input.
    pub fn write_bytes(&self, bytes: &[u8]) -> Result<(), String> {
        if let Ok(mut w) = self.writer.lock() {
            w.write_all(bytes)
                .map_err(|e| format!("Failed to write to PTY: {e}"))?;
            w.flush()
                .map_err(|e| format!("Failed to flush PTY: {e}"))?;
            Ok(())
        } else {
            Err("Failed to acquire PTY writer lock".into())
        }
    }

    /// Drains all incoming byte chunks currently available in the output channel without blocking.
    pub fn try_recv(&mut self) -> Vec<Vec<u8>> {
        let mut chunks = Vec::new();
        while let Ok(chunk) = self.receiver.try_recv() {
            chunks.push(chunk);
        }
        chunks
    }

    /// Checks if the child process is still actively executing.
    pub fn is_alive(&mut self) -> bool {
        match self.child.try_wait() {
            Ok(Some(_status)) => false,
            Ok(None) => true,
            Err(_) => false,
        }
    }

    /// Resizes the pseudo-terminal window dimensions.
    pub fn resize(&self, cols: u16, rows: u16) {
        let _ = self.master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        });
    }

    /// Terminates the child process immediately.
    pub fn kill(&mut self) {
        let _ = self.child.kill();
    }
}

