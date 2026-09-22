//! File persistence and safe atomic I/O for documents.

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;
use std::time::SystemTime;
use crate::Document;

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
}

impl FileMetadata {
    /// Captures metadata for a file path.
    pub fn from_path(path: &Path) -> Result<Self, io::Error> {
        let meta = fs::metadata(path)?;
        let modified = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        Ok(Self {
            modified,
            len_bytes: meta.len(),
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
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;

    // Create unique adjacent temporary file
    let file_name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("document");
    let temp_name = format!(".{}.arcade_tmp_{}", file_name, std::process::id());
    let temp_path = parent.join(temp_name);

    {
        let mut temp_file = File::create(&temp_path)?;
        for chunk in doc.rope().chunks() {
            temp_file.write_all(chunk.as_bytes())?;
        }
        temp_file.sync_all()?;
    }

    // Atomic replacement
    fs::rename(&temp_path, path)?;

    let meta = FileMetadata::from_path(path)?;
    Ok(meta)
}

/// Checks if the file on disk has been modified compared to the recorded metadata.
pub fn has_external_change(path: &Path, recorded: &FileMetadata) -> bool {
    let Ok(current) = FileMetadata::from_path(path) else {
        return true;
    };
    current.modified != recorded.modified || current.len_bytes != recorded.len_bytes
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
}
