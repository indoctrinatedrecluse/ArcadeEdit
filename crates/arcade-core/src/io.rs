//! File persistence and safe atomic I/O for documents.

use crate::Document;
use std::fs::{self, OpenOptions};
use std::hash::{Hash, Hasher};
use std::io::{self, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::SystemTime;

/// Errors that can occur during document file persistence operations.
#[derive(Debug)]
pub enum PersistenceError {
    /// An underlying standard I/O error occurred.
    Io(io::Error),
    /// The file contents could not be decoded as valid UTF-8.
    InvalidUtf8,
}

impl std::fmt::Display for PersistenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "I/O error: {}", err),
            Self::InvalidUtf8 => write!(f, "File content is not valid UTF-8"),
        }
    }
}

impl std::error::Error for PersistenceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::InvalidUtf8 => None,
        }
    }
}

impl From<io::Error> for PersistenceError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

/// Metadata recorded when a document is read from or written to disk.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FileMetadata {
    /// Timestamp when the file was last modified on disk.
    pub modified: SystemTime,
    /// Total file length in bytes.
    pub len_bytes: u64,
    /// Content fingerprint to catch edits that preserve size and file timestamp.
    pub content_fingerprint: u64,
}

impl FileMetadata {
    /// Captures metadata for a file path.
    pub fn from_path(path: &Path) -> Result<Self, io::Error> {
        let meta = fs::metadata(path)?;
        let modified = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        let bytes = fs::read(path)?;
        Ok(Self {
            modified,
            len_bytes: meta.len(),
            content_fingerprint: fingerprint(&bytes),
        })
    }
}

/// Loads a UTF-8 document and its disk metadata from a file path.
pub fn load_from_file(path: &Path) -> Result<(Document, FileMetadata), PersistenceError> {
    let bytes = fs::read(path)?;
    let content = String::from_utf8(bytes).map_err(|_| PersistenceError::InvalidUtf8)?;
    let meta = FileMetadata::from_path(path)?;
    let doc = Document::new(&content);
    Ok((doc, meta))
}

/// Atomically saves a document to a file path via an adjacent temporary file.
pub fn save_to_file(doc: &Document, path: &Path) -> Result<FileMetadata, PersistenceError> {
    // Resolve an existing symlink to its target so saving an opened link does not replace
    // the link itself with a regular file.
    let destination = if path.exists() {
        fs::canonicalize(path)?
    } else {
        path.to_path_buf()
    };
    let parent = destination
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;

    // Create a unique adjacent temporary file without following a pre-existing symlink.
    let file_name = destination
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("document");
    static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let (temp_path, mut temp_file) = (0..128)
        .find_map(|_| {
            let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let nonce = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let candidate = parent.join(format!(
                ".{file_name}.arcade_tmp_{}_{}_{nonce}",
                std::process::id(),
                sequence
            ));
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&candidate)
            {
                Ok(file) => Some(Ok((candidate, file))),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => None,
                Err(error) => Some(Err(error)),
            }
        })
        .transpose()?
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::AlreadyExists,
                "could not allocate a unique save file",
            )
        })?;

    let write_result = (|| -> io::Result<()> {
        for chunk in doc.rope().chunks() {
            temp_file.write_all(chunk.as_bytes())?;
        }
        if let Ok(existing) = fs::metadata(&destination) {
            temp_file.set_permissions(existing.permissions())?;
        }
        temp_file.sync_all()
    })();
    drop(temp_file);
    if let Err(error) = write_result {
        let _ = fs::remove_file(&temp_path);
        return Err(error.into());
    }

    // Atomic replacement
    if let Err(error) = fs::rename(&temp_path, &destination) {
        let _ = fs::remove_file(&temp_path);
        return Err(error.into());
    }

    let meta = FileMetadata::from_path(&destination)?;
    Ok(meta)
}

/// Checks if the file on disk has been modified compared to the recorded metadata.
pub fn has_external_change(path: &Path, recorded: &FileMetadata) -> bool {
    let Ok(current) = FileMetadata::from_path(path) else {
        return true;
    };
    current.modified != recorded.modified
        || current.len_bytes != recorded.len_bytes
        || current.content_fingerprint != recorded.content_fingerprint
}

fn fingerprint(bytes: &[u8]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saves_and_loads_documents_atomically() {
        let temp_dir = std::env::temp_dir().join(format!("arcade_test_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);
        let file_path = temp_dir.join("test_save.txt");

        let doc = Document::new("Hello from ArcadeEdit persistence!");
        let saved_meta = save_to_file(&doc, &file_path).unwrap();

        assert_eq!(saved_meta.len_bytes, doc.len_bytes() as u64);
        assert!(!has_external_change(&file_path, &saved_meta));

        let (loaded_doc, loaded_meta) = load_from_file(&file_path).unwrap();
        assert_eq!(loaded_doc.to_string(), doc.to_string());
        assert_eq!(loaded_meta, saved_meta);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn detects_same_length_external_content_changes() {
        let temp_dir =
            std::env::temp_dir().join(format!("arcade_fingerprint_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);
        let file_path = temp_dir.join("same_length.txt");
        fs::write(&file_path, "first").unwrap();
        let recorded = FileMetadata::from_path(&file_path).unwrap();
        fs::write(&file_path, "other").unwrap();
        assert!(has_external_change(&file_path, &recorded));
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
