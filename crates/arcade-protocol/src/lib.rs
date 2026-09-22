//! Versioned command and result types for ArcadeEdit's non-UI boundary.

use std::path::PathBuf;
use serde::{Deserialize, Serialize};

/// The first stable shape of the ArcadeEdit command protocol.
pub const PROTOCOL_VERSION: u32 = 1;

/// A command that can be issued by the headless binary or, later, a service.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Command {
    /// Report basic information about a document without mutating it.
    Inspect {
        /// The document to inspect.
        path: PathBuf,
    },
    /// Find text in a document or workspace.
    Search {
        /// The text to find.
        query: String,
        /// Documents or directories to search.
        paths: Vec<PathBuf>,
    },
    /// Replace text, optionally without writing the result.
    Replace {
        /// The text to replace.
        query: String,
        /// The text inserted for every match.
        replacement: String,
        /// When true, produce the proposed result without writing files.
        dry_run: bool,
        /// Documents or directories to modify.
        paths: Vec<PathBuf>,
    },
}

/// Metadata and structural inspection metrics for a document.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct InspectResult {
    /// File path inspected.
    pub path: PathBuf,
    /// Total byte size of the document.
    pub byte_size: usize,
    /// Total unicode character (scalar) count.
    pub char_count: usize,
    /// Total lines in the document.
    pub line_count: usize,
    /// Character encoding detected (e.g. "UTF-8").
    pub encoding: String,
    /// Detected line ending convention ("LF", "CRLF", or "None").
    pub line_ending: String,
    /// Inferred language grammar (e.g. "Rust", "Markdown", "Plain Text").
    pub language: String,
    /// Whether the file is read-only.
    pub is_read_only: bool,
}

/// A single matched occurrence from a search command.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SearchMatch {
    /// Source file path where match was found.
    pub path: PathBuf,
    /// 1-based line number.
    pub line: usize,
    /// 1-based column number.
    pub column: usize,
    /// 0-based byte offset in the document.
    pub byte_offset: usize,
    /// Matched line text with trailing line break stripped.
    pub line_text: String,
}

/// Consolidated result of a search operation across one or more files.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SearchResult {
    /// The search query term.
    pub query: String,
    /// Total number of matches discovered.
    pub total_matches: usize,
    /// Total number of files inspected.
    pub files_searched: usize,
    /// All match occurrences.
    pub matches: Vec<SearchMatch>,
}

/// Summary of replacements performed on a single file.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FileReplaceSummary {
    /// Path to the modified file.
    pub path: PathBuf,
    /// Number of replacements made in this file.
    pub replacements: usize,
    /// Whether the file was modified on disk (false if dry-run).
    pub modified: bool,
}

/// Consolidated result of a replace operation across one or more files.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReplaceResult {
    /// The target search query replaced.
    pub query: String,
    /// The replacement string used.
    pub replacement: String,
    /// Whether this execution was performed as a dry run.
    pub dry_run: bool,
    /// Total number of replacements made across all files.
    pub total_replacements: usize,
    /// Per-file replacement details.
    pub files: Vec<FileReplaceSummary>,
}

/// A serializable response envelope for a command.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Response<T> {
    /// Protocol revision used to create the response.
    pub protocol_version: u32,
    /// The command result.
    pub result: T,
}

impl<T> Response<T> {
    /// Wraps a result using the current protocol version.
    pub fn new(result: T) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            result,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_and_deserializes_inspect_response() {
        let inspect = InspectResult {
            path: PathBuf::from("src/main.rs"),
            byte_size: 1024,
            char_count: 980,
            line_count: 50,
            encoding: "UTF-8".into(),
            line_ending: "LF".into(),
            language: "Rust".into(),
            is_read_only: false,
        };

        let response = Response::new(inspect);
        let json = serde_json::to_string(&response).expect("serialize");
        let deserialized: Response<InspectResult> = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(deserialized.protocol_version, PROTOCOL_VERSION);
        assert_eq!(deserialized.result.language, "Rust");
        assert_eq!(deserialized.result.line_count, 50);
    }

    #[test]
    fn serializes_and_deserializes_search_and_replace() {
        let search = SearchResult {
            query: "fn main".into(),
            total_matches: 1,
            files_searched: 5,
            matches: vec![SearchMatch {
                path: PathBuf::from("src/main.rs"),
                line: 1,
                column: 1,
                byte_offset: 0,
                line_text: "fn main() {}".into(),
            }],
        };
        let res = Response::new(search);
        let json = serde_json::to_string(&res).expect("serialize");
        assert!(json.contains("fn main"));

        let replace = ReplaceResult {
            query: "foo".into(),
            replacement: "bar".into(),
            dry_run: true,
            total_replacements: 2,
            files: vec![FileReplaceSummary {
                path: PathBuf::from("file.txt"),
                replacements: 2,
                modified: false,
            }],
        };
        let res = Response::new(replace);
        let json = serde_json::to_string(&res).expect("serialize");
        assert!(json.contains("dry_run"));
    }
}
