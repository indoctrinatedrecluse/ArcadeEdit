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
