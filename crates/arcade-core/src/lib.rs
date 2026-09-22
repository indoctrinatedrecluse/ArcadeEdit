//! UI-independent text storage and edit transactions for ArcadeEdit.
//!
//! This crate deliberately has no dependency on GPUI, a terminal, a display
//! server, or a container runtime. It is shared by desktop and headless modes.

pub mod history;
pub mod io;
pub mod search;
pub mod selection;

use std::cmp::min;
use ropey::Rope;

pub use history::{History, Transaction};
pub use io::{has_external_change, load_from_file, save_to_file, FileMetadata, PersistenceError};
pub use search::{find_all, find_next, find_prev, replace_all};
pub use selection::{Selection, SelectionSet};

/// A UTF-8 byte offset in a document.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct ByteOffset(pub usize);

/// A half-open range of UTF-8 byte offsets.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TextRange {
    /// The first byte in the range.
    pub start: ByteOffset,
    /// The first byte after the range.
    pub end: ByteOffset,
}

impl TextRange {
    /// Creates a range when its endpoints are ordered.
    pub fn new(start: ByteOffset, end: ByteOffset) -> Result<Self, EditError> {
        if start > end {
            return Err(EditError::InvertedRange { start, end });
        }
        Ok(Self { start, end })
    }

    /// Returns the length of this range in bytes.
    pub fn len_bytes(&self) -> usize {
        self.end.0 - self.start.0
    }

    /// Returns `true` if this range represents an empty point.
    pub fn is_empty(&self) -> bool {
        self.start.0 == self.end.0
    }
}

/// A replacement expressed against the document's current revision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TextEdit {
    /// The UTF-8 byte range to replace.
    pub range: TextRange,
    /// The UTF-8 text inserted in its place.
    pub replacement: String,
}

/// A monotonically increasing document revision.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct Revision(pub u64);

/// The result of applying one edit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppliedEdit {
    /// The revision before the edit.
    pub previous_revision: Revision,
    /// The revision after the edit.
    pub revision: Revision,
    /// The edit as applied to the prior document.
    pub edit: TextEdit,
}

/// An error returned when an edit cannot be applied safely.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EditError {
    /// The range endpoints were supplied in reverse order.
    InvertedRange {
        /// The supplied range start.
        start: ByteOffset,
        /// The supplied range end.
        end: ByteOffset,
    },
    /// An endpoint was outside the document or split a UTF-8 character.
    InvalidByteOffset(ByteOffset),
}

impl std::fmt::Display for EditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvertedRange { start, end } => {
                write!(f, "Inverted range: {} > {}", start.0, end.0)
            }
            Self::InvalidByteOffset(offset) => {
                write!(f, "Invalid or character-splitting byte offset: {}", offset.0)
            }
        }
    }
}

impl std::error::Error for EditError {}

/// A Ropey-backed UTF-8 text document.
#[derive(Clone, Debug)]
pub struct Document {
    text: Rope,
    revision: Revision,
}

impl Document {
    /// Creates a document from UTF-8 text.
    pub fn new(text: &str) -> Self {
        Self {
            text: Rope::from_str(text),
            revision: Revision::default(),
        }
    }

    /// Returns the document's current revision.
    pub fn revision(&self) -> Revision {
        self.revision
    }

    /// Returns the UTF-8 byte length of the document.
    pub fn len_bytes(&self) -> usize {
        self.text.len_bytes()
    }

    /// Returns the number of physical lines in the document.
    pub fn len_lines(&self) -> usize {
        self.text.len_lines()
    }

    /// Returns the backing rope for read-only consumers such as parsers.
    pub fn rope(&self) -> &Rope {
        &self.text
    }

    /// Materializes the document for small callers such as tests and CLI output.
    pub fn to_string(&self) -> String {
        self.text.to_string()
    }

    /// Applies one replacement and advances the document revision.
    pub fn apply(&mut self, edit: TextEdit) -> Result<AppliedEdit, EditError> {
        let start = self.char_index(edit.range.start)?;
        let end = self.char_index(edit.range.end)?;
        let previous_revision = self.revision;

        self.text.remove(start..end);
        self.text.insert(start, &edit.replacement);
        self.revision = Revision(self.revision.0.saturating_add(1));

        Ok(AppliedEdit {
            previous_revision,
            revision: self.revision,
            edit,
        })
    }

    /// Inserts text simultaneously across all selections in `selections`.
    pub fn insert_text_at_selections(
        &mut self,
        selections: &SelectionSet,
        text: &str,
    ) -> Result<(Transaction, SelectionSet), EditError> {
        let before_rev = self.revision;
        let before_selections = selections.clone();
        let text_len_bytes = text.len();

        let mut edits = Vec::new();
        let mut inverse_edits = Vec::new();
        let mut resulting_cursors = Vec::new();

        // Calculate offset shift as edits are planned
        let mut cumulative_delta: isize = 0;

        for sel in selections.as_slice() {
            let range = sel.range();
            let removed_len = range.len_bytes();
            let net_change = text_len_bytes as isize - removed_len as isize;

            let new_cursor_pos = (range.start.0 as isize + cumulative_delta + text_len_bytes as isize) as usize;
            resulting_cursors.push(Selection::cursor(ByteOffset(new_cursor_pos)));
            cumulative_delta += net_change;

            let edit = TextEdit {
                range,
                replacement: text.to_string(),
            };

            let removed_text = self.text.byte_slice(range.start.0..range.end.0).to_string();
            let inv_end = ByteOffset(range.start.0 + text_len_bytes);
            let inverse = TextEdit {
                range: TextRange::new(range.start, inv_end)?,
                replacement: removed_text,
            };

            edits.push(edit);
            inverse_edits.push(inverse);
        }

        // Apply edits in descending order so earlier byte offsets stay valid
        for edit in edits.iter().rev() {
            self.apply(edit.clone())?;
        }

        let after_selections = SelectionSet::from_selections(resulting_cursors, selections.primary_index());
        let resulting_rev = self.revision;

        let tx = Transaction::new(
            edits,
            inverse_edits,
            before_selections,
            after_selections.clone(),
            before_rev,
            resulting_rev,
        );

        Ok((tx, after_selections))
    }

    /// Deletes backward (Backspace) at each selection in `selections`.
    pub fn delete_backward_at_selections(
        &mut self,
        selections: &SelectionSet,
    ) -> Result<(Transaction, SelectionSet), EditError> {
        let before_rev = self.revision;
        let before_selections = selections.clone();

        let mut edits = Vec::new();
        let mut inverse_edits = Vec::new();
        let mut resulting_cursors = Vec::new();

        let mut cumulative_delta: isize = 0;

        for sel in selections.as_slice() {
            let delete_range = if !sel.is_empty() {
                sel.range()
            } else if sel.head.0 > 0 {
                let current_char = self.text.byte_to_char(sel.head.0);
                let prev_char = current_char.saturating_sub(1);
                let start_byte = ByteOffset(self.text.char_to_byte(prev_char));
                TextRange::new(start_byte, sel.head)?
            } else {
                // At beginning of file, nothing to delete
                let new_cursor_pos = (sel.head.0 as isize + cumulative_delta) as usize;
                resulting_cursors.push(Selection::cursor(ByteOffset(new_cursor_pos)));
                continue;
            };

            let removed_len = delete_range.len_bytes();
            let new_cursor_pos = (delete_range.start.0 as isize + cumulative_delta) as usize;
            resulting_cursors.push(Selection::cursor(ByteOffset(new_cursor_pos)));
            cumulative_delta -= removed_len as isize;

            let removed_text = self.text.byte_slice(delete_range.start.0..delete_range.end.0).to_string();
            let edit = TextEdit {
                range: delete_range,
                replacement: String::new(),
            };
            let inverse = TextEdit {
                range: TextRange::new(delete_range.start, delete_range.start)?,
                replacement: removed_text,
            };

            edits.push(edit);
            inverse_edits.push(inverse);
        }

        // Apply edits in descending order
        for edit in edits.iter().rev() {
            self.apply(edit.clone())?;
        }

        let after_selections = SelectionSet::from_selections(resulting_cursors, selections.primary_index());
        let resulting_rev = self.revision;

        let tx = Transaction::new(
            edits,
            inverse_edits,
            before_selections,
            after_selections.clone(),
            before_rev,
            resulting_rev,
        );

        Ok((tx, after_selections))
    }

    /// Converts a physical byte offset into a character index.
    pub fn byte_to_char(&self, offset: ByteOffset) -> Result<usize, EditError> {
        self.char_index(offset)
    }

    /// Converts a character index to its byte offset.
    pub fn char_to_byte(&self, char_idx: usize) -> ByteOffset {
        let safe_idx = min(char_idx, self.text.len_chars());
        ByteOffset(self.text.char_to_byte(safe_idx))
    }

    /// Returns the 0-based physical line index for a given byte offset.
    pub fn line_of_byte(&self, offset: ByteOffset) -> usize {
        let char_idx = self.text.byte_to_char(offset.0);
        self.text.char_to_line(char_idx)
    }

    /// Returns the starting byte offset of the given line.
    pub fn line_start_byte(&self, line_idx: usize) -> ByteOffset {
        let safe_line = min(line_idx, self.text.len_lines().saturating_sub(1));
        let char_idx = self.text.line_to_char(safe_line);
        ByteOffset(self.text.char_to_byte(char_idx))
    }

    fn char_index(&self, offset: ByteOffset) -> Result<usize, EditError> {
        let character_index = self
            .text
            .try_byte_to_char(offset.0)
            .map_err(|_| EditError::InvalidByteOffset(offset))?;
        let canonical_offset = self
            .text
            .try_char_to_byte(character_index)
            .map_err(|_| EditError::InvalidByteOffset(offset))?;

        (canonical_offset == offset.0)
            .then_some(character_index)
            .ok_or(EditError::InvalidByteOffset(offset))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inserts_text_at_multiple_cursors() {
        let mut doc = Document::new("apple banana cherry");
        let sel1 = Selection::cursor(ByteOffset(0));
        let sel2 = Selection::cursor(ByteOffset(6));
        let sel3 = Selection::cursor(ByteOffset(13));
        let selections = SelectionSet::from_selections(vec![sel1, sel2, sel3], 0);

        let (tx, new_sels) = doc.insert_text_at_selections(&selections, "> ").unwrap();

        assert_eq!(doc.to_string(), "> apple > banana > cherry");
        assert_eq!(new_sels.len(), 3);
        assert_eq!(new_sels.as_slice()[0].head, ByteOffset(2));
        assert_eq!(new_sels.as_slice()[1].head, ByteOffset(10));
        assert_eq!(new_sels.as_slice()[2].head, ByteOffset(19));
        assert_eq!(tx.edits.len(), 3);
    }

    #[test]
    fn deletes_backward_at_multiple_selections() {
        let mut doc = Document::new("apple, banana, cherry");
        let sel1 = Selection::new(ByteOffset(0), ByteOffset(5)); // "apple"
        let sel2 = Selection::new(ByteOffset(7), ByteOffset(13)); // "banana"
        let selections = SelectionSet::from_selections(vec![sel1, sel2], 0);

        let (_tx, new_sels) = doc.delete_backward_at_selections(&selections).unwrap();

        assert_eq!(doc.to_string(), ", , cherry");
        assert_eq!(new_sels.as_slice()[0].head, ByteOffset(0));
        assert_eq!(new_sels.as_slice()[1].head, ByteOffset(2));
    }
}
