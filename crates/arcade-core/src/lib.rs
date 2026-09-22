//! UI-independent text storage and edit transactions for ArcadeEdit.
//!
//! This crate deliberately has no dependency on GPUI, a terminal, a display
//! server, or a container runtime. It is shared by desktop and headless modes.

use ropey::Rope;

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
    ///
    /// Rendering and parsing paths should read from [`Self::rope`] instead.
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
    fn applies_a_byte_ranged_edit_and_tracks_revisions() {
        let mut document = Document::new("hello world");
        let edit = TextEdit {
            range: TextRange::new(ByteOffset(6), ByteOffset(11)).unwrap(),
            replacement: "ArcadeEdit".into(),
        };

        let applied = document.apply(edit).unwrap();

        assert_eq!(document.to_string(), "hello ArcadeEdit");
        assert_eq!(applied.previous_revision, Revision(0));
        assert_eq!(applied.revision, Revision(1));
    }

    #[test]
    fn rejects_an_offset_inside_a_utf8_character() {
        let mut document = Document::new("aé");
        let edit = TextEdit {
            range: TextRange::new(ByteOffset(2), ByteOffset(2)).unwrap(),
            replacement: "x".into(),
        };

        assert_eq!(
            document.apply(edit),
            Err(EditError::InvalidByteOffset(ByteOffset(2)))
        );
    }
}
