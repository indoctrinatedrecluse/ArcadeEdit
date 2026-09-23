//! Hierarchical workspace directory tree model and virtualized item builder.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use arcade_workspace::DEFAULT_IGNORED_DIRS;

/// Represents a single flattened, visible row in the workspace file tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibleTreeItem {
    /// Full path on disk for this file or directory.
    pub path: PathBuf,
    /// Display name of the file or folder.
    pub name: String,
    /// Whether this tree node is a directory.
    pub is_dir: bool,
    /// Indentation nesting depth (0 for root direct children).
    pub depth: usize,
    /// Whether this directory is currently expanded.
    pub is_expanded: bool,
}

impl VisibleTreeItem {
    /// Returns the appropriate visual icon based on filetype or directory state.
    pub fn icon(&self) -> &'static str {
        if self.is_dir {
            if self.is_expanded {
                "📂"
            } else {
                "📁"
            }
        } else {
            match self.path.extension().and_then(|e| e.to_str()).unwrap_or("") {
                "rs" => "🦀",
                "md" | "markdown" => "📄",
                "toml" | "json" | "yaml" | "yml" => "⚙️",
                "lock" => "🔒",
                _ => "📝",
            }
        }
    }
}

/// Builds a flat list of visible tree rows from the given workspace root,
/// recursively expanding only the directories present in `expanded_dirs`.
pub fn build_visible_tree(root: &Path, expanded_dirs: &BTreeSet<PathBuf>) -> Vec<VisibleTreeItem> {
    let mut items = Vec::new();
    collect_visible_items(root, expanded_dirs, 0, &mut items);
    items
}

fn collect_visible_items(
    dir: &Path,
    expanded_dirs: &BTreeSet<PathBuf>,
    depth: usize,
    out: &mut Vec<VisibleTreeItem>,
) {
    if depth > 16 || !dir.exists() || !dir.is_dir() {
        return;
    }

    let entries = match fs::read_dir(dir) {
        Ok(read_dir) => read_dir,
        Err(_) => return,
    };

    let mut dirs: Vec<PathBuf> = Vec::new();
    let mut files: Vec<PathBuf> = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n,
            None => continue,
        };

        if path.is_dir() {
            if !DEFAULT_IGNORED_DIRS.contains(&name) {
                dirs.push(path);
            }
        } else if path.is_file() {
            files.push(path);
        }
    }

    // Sort alphabetically (case-insensitive for intuitive browsing)
    dirs.sort_by(|a, b| {
        a.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase()
            .cmp(&b.file_name().unwrap_or_default().to_string_lossy().to_lowercase())
    });
    files.sort_by(|a, b| {
        a.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase()
            .cmp(&b.file_name().unwrap_or_default().to_string_lossy().to_lowercase())
    });

    // 1. Process directories first
    for d in dirs {
        let name = d
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("folder")
            .to_string();
        let is_expanded = expanded_dirs.contains(&d);

        out.push(VisibleTreeItem {
            path: d.clone(),
            name,
            is_dir: true,
            depth,
            is_expanded,
        });

        if is_expanded {
            collect_visible_items(&d, expanded_dirs, depth + 1, out);
        }
    }

    // 2. Process files
    for f in files {
        let name = f
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file")
            .to_string();

        out.push(VisibleTreeItem {
            path: f,
            name,
            is_dir: false,
            depth,
            is_expanded: false,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_tree_and_handles_expansion() {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let mut expanded = BTreeSet::new();

        // Initially collapsed: root direct children only
        let collapsed_tree = build_visible_tree(&manifest_dir, &expanded);
        assert!(!collapsed_tree.is_empty());
        assert!(collapsed_tree.iter().any(|item| item.name == "Cargo.toml"));

        // Locate crates directory if present
        let src_dir = manifest_dir.join("src");
        if src_dir.exists() {
            let collapsed_count = collapsed_tree.len();
            expanded.insert(src_dir.clone());
            let expanded_tree = build_visible_tree(&manifest_dir, &expanded);

            assert!(expanded_tree.len() > collapsed_count);
            let src_item = expanded_tree.iter().find(|i| i.path == src_dir).expect("src dir found");
            assert!(src_item.is_expanded);
            assert_eq!(src_item.icon(), "📂");

            let lib_item = expanded_tree.iter().find(|i| i.name == "lib.rs").expect("lib.rs found");
            assert_eq!(lib_item.depth, 1);
            assert_eq!(lib_item.icon(), "🦀");
        }
    }
}
