//! Workspace services shared by desktop and headless applications.

use std::fs;
use std::path::{Path, PathBuf};

/// A conservative default used until configurable huge-file policies land.
pub const DEFAULT_HUGE_FILE_THRESHOLD_BYTES: usize = 50 * 1024 * 1024;

/// Directory names skipped during recursive workspace discovery.
pub const DEFAULT_IGNORED_DIRS: &[&str] = &[
    ".git",
    "target",
    "node_modules",
    ".idea",
    ".vscode",
    "dist",
    "build",
];

/// Recursively collects all candidate document files from the supplied paths.
///
/// Files are added directly; directories are traversed excluding common build/VCS
/// directories and files exceeding [`DEFAULT_HUGE_FILE_THRESHOLD_BYTES`].
pub fn discover_files<P: AsRef<Path>>(roots: &[P]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for root in roots {
        collect_recursive(root.as_ref(), &mut files, 0);
    }
    files.sort();
    files.dedup();
    files
}

fn collect_recursive(path: &Path, files: &mut Vec<PathBuf>, depth: usize) {
    if depth > 16 || !path.exists() {
        return;
    }

    if path.is_file() {
        if let Ok(meta) = fs::metadata(path) {
            if meta.len() <= DEFAULT_HUGE_FILE_THRESHOLD_BYTES as u64 {
                files.push(path.to_path_buf());
            }
        }
        return;
    }

    if path.is_dir() {
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if DEFAULT_IGNORED_DIRS.contains(&name) {
                return;
            }
        }

        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                collect_recursive(&entry.path(), files, depth + 1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovers_files_in_workspace() {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let files = discover_files(&[manifest_dir]);
        assert!(!files.is_empty());
        assert!(files.iter().any(|f| f.ends_with("Cargo.toml")));
    }
}
