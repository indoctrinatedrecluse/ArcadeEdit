//! Workspace services shared by desktop and headless applications.

use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

/// A conservative default used until configurable huge-file policies land.
pub const DEFAULT_HUGE_FILE_THRESHOLD_BYTES: usize = 50 * 1024 * 1024;

/// Directory names skipped during recursive workspace discovery.
pub const DEFAULT_IGNORED_DIRS: &[&str] = &[
    ".git",
    ".hg",
    ".svn",
    "target",
    "node_modules",
    ".idea",
    ".vscode",
    "dist",
    "build",
    "vendor",
];

/// Files and traversal errors produced while walking explicit workspace roots.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DiscoveryResult {
    /// Files that passed ignore and size filters.
    pub files: Vec<PathBuf>,
    /// Paths that could not be inspected.
    pub errors: Vec<String>,
}

/// Recursively collects all candidate document files from the supplied paths.
///
/// Files are added directly; directories are traversed excluding common build/VCS
/// directories and files exceeding [`DEFAULT_HUGE_FILE_THRESHOLD_BYTES`].
pub fn discover_files<P: AsRef<Path>>(roots: &[P]) -> Vec<PathBuf> {
    discover_files_with_errors(roots).files
}

/// Recursively collects candidate files while honoring ignore files and reporting walk errors.
pub fn discover_files_with_errors<P: AsRef<Path>>(roots: &[P]) -> DiscoveryResult {
    let mut result = DiscoveryResult::default();
    for root in roots {
        let root = root.as_ref();
        if !root.exists() {
            result
                .errors
                .push(format!("{}: path does not exist", root.display()));
            continue;
        }

        if root.is_file() {
            match root.metadata() {
                Ok(meta) if meta.len() <= DEFAULT_HUGE_FILE_THRESHOLD_BYTES as u64 => {
                    result.files.push(root.to_path_buf());
                }
                Ok(meta) => result.errors.push(format!(
                    "{}: file is {} bytes; limit is {} bytes",
                    root.display(),
                    meta.len(),
                    DEFAULT_HUGE_FILE_THRESHOLD_BYTES
                )),
                Err(error) => result.errors.push(format!("{}: {error}", root.display())),
            }
            continue;
        }

        let walker = WalkBuilder::new(root)
            .hidden(false)
            .follow_links(false)
            .max_depth(Some(17))
            .filter_entry(|entry| {
                entry.file_type().is_none_or(|kind| !kind.is_dir())
                    || entry
                        .file_name()
                        .to_str()
                        .is_none_or(|name| !DEFAULT_IGNORED_DIRS.contains(&name))
            })
            .build();

        for entry in walker {
            match entry {
                Ok(entry) if entry.file_type().is_some_and(|kind| kind.is_file()) => {
                    match entry.metadata() {
                        Ok(meta) if meta.len() <= DEFAULT_HUGE_FILE_THRESHOLD_BYTES as u64 => {
                            result.files.push(entry.into_path());
                        }
                        Ok(meta) => result.errors.push(format!(
                            "{}: file is {} bytes; limit is {} bytes",
                            entry.path().display(),
                            meta.len(),
                            DEFAULT_HUGE_FILE_THRESHOLD_BYTES
                        )),
                        Err(error) => result
                            .errors
                            .push(format!("{}: {error}", entry.path().display())),
                    }
                }
                Ok(_) => {}
                Err(error) => result.errors.push(error.to_string()),
            }
        }
    }
    result.files.sort();
    result.files.dedup();
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn discovers_files_in_workspace() {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let files = discover_files(&[manifest_dir]);
        assert!(!files.is_empty());
        assert!(files.iter().any(|f| f.ends_with("Cargo.toml")));
    }

    #[test]
    fn respects_gitignore_and_reports_missing_roots() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("arcade_workspace_{nonce}"));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir(root.join(".git")).unwrap();
        std::fs::write(root.join(".gitignore"), "ignored.txt\n").unwrap();
        std::fs::write(root.join("visible.txt"), "visible").unwrap();
        std::fs::write(root.join("ignored.txt"), "ignored").unwrap();

        let result = discover_files_with_errors(&[&root]);
        assert!(result
            .files
            .iter()
            .any(|path| path.ends_with("visible.txt")));
        assert!(!result
            .files
            .iter()
            .any(|path| path.ends_with("ignored.txt")));
        assert!(result.errors.is_empty());

        let missing = root.join("does-not-exist");
        assert!(!discover_files_with_errors(&[missing]).errors.is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }
}
