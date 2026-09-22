//! Character and byte-aware cursor and multi-selection primitives.

use std::cmp::{max, min};
use crate::{ByteOffset, TextRange};
use ropey::Rope;

/// Represents an individual cursor or range selection in a document.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Selection {
    /// The fixed anchor point where the selection began.
    pub anchor: ByteOffset,
    /// The moving head point (caret position).
    pub head: ByteOffset,
}

impl Selection {
    /// Creates a collapsed selection representing a single point cursor.
    pub const fn cursor(offset: ByteOffset) -> Self {
        Self {
            anchor: offset,
            head: offset,
        }
    }

    /// Creates a selection with specified anchor and head.
    pub const fn new(anchor: ByteOffset, head: ByteOffset) -> Self {
        Self { anchor, head }
    }

    /// Returns `true` if the selection is collapsed (cursor with length 0).
    pub const fn is_empty(&self) -> bool {
        self.anchor.0 == self.head.0
    }

    /// Returns `true` if the head is strictly ahead of the anchor.
    pub const fn is_forward(&self) -> bool {
        self.head.0 >= self.anchor.0
    }

    /// Returns the earlier offset in the document.
    pub fn start(&self) -> ByteOffset {
        min(self.anchor, self.head)
    }

    /// Returns the later offset in the document.
    pub fn end(&self) -> ByteOffset {
        max(self.anchor, self.head)
    }

    /// Returns the half-open `TextRange` covered by this selection.
    pub fn range(&self) -> TextRange {
        TextRange {
            start: self.start(),
            end: self.end(),
        }
    }

    /// Collapses this selection to its head (caret).
    pub fn collapse(&self) -> Self {
        Self::cursor(self.head)
    }
}

/// An ordered, non-empty collection of non-overlapping selections.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectionSet {
    selections: Vec<Selection>,
    primary_index: usize,
}

impl SelectionSet {
    /// Creates a selection set containing a single cursor at the beginning of the document.
    pub fn new() -> Self {
        Self::cursor(ByteOffset(0))
    }

    /// Creates a selection set with a single collapsed cursor at the given offset.
    pub fn cursor(offset: ByteOffset) -> Self {
        Self {
            selections: vec![Selection::cursor(offset)],
            primary_index: 0,
        }
    }

    /// Creates a selection set from a single selection.
    pub fn single(selection: Selection) -> Self {
        Self {
            selections: vec![selection],
            primary_index: 0,
        }
    }

    /// Creates and normalizes a selection set from an arbitrary collection of selections.
    pub fn from_selections(mut list: Vec<Selection>, primary: usize) -> Self {
        if list.is_empty() {
            return Self::new();
        }

        // Sort selections by starting position
        list.sort_by_key(|s| (s.start(), s.end()));

        let mut merged: Vec<Selection> = Vec::with_capacity(list.len());
        let mut new_primary = 0;

        for (idx, sel) in list.into_iter().enumerate() {
            if let Some(last) = merged.last_mut() {
                // If touching or overlapping, merge them
                if sel.start() <= last.end() {
                    let new_start = min(last.start(), sel.start());
                    let new_end = max(last.end(), sel.end());

                    // Retain direction of the newer/current selection if it was primary
                    if idx == primary {
                        *last = if sel.is_forward() {
                            Selection::new(new_start, new_end)
                        } else {
                            Selection::new(new_end, new_start)
                        };
                        new_primary = merged.len() - 1;
                    } else {
                        let forward = last.is_forward();
                        *last = if forward {
                            Selection::new(new_start, new_end)
                        } else {
                            Selection::new(new_end, new_start)
                        };
                    }
                    continue;
                }
            }

            if idx == primary {
                new_primary = merged.len();
            }
            merged.push(sel);
        }

        Self {
            selections: merged,
            primary_index: new_primary,
        }
    }

    /// Returns the primary selection.
    pub fn primary(&self) -> Selection {
        self.selections[self.primary_index]
    }

    /// Returns the index of the primary selection.
    pub fn primary_index(&self) -> usize {
        self.primary_index
    }

    /// Returns a slice of all disjoint selections.
    pub fn as_slice(&self) -> &[Selection] {
        &self.selections
    }

    /// Returns the number of selections in the set.
    pub fn len(&self) -> usize {
        self.selections.len()
    }

    /// Returns `true` if there are no selections (in practice always false).
    pub fn is_empty(&self) -> bool {
        self.selections.is_empty()
    }

    /// Adds another selection to this set and re-normalizes.
    pub fn add(&mut self, selection: Selection) {
        let mut list = self.selections.clone();
        list.push(selection);
        *self = Self::from_selections(list, self.selections.len());
    }

    // -------------------------------------------------------------------------
    // Cursor Navigation Primitives
    // -------------------------------------------------------------------------

    /// Moves all cursors one character to the left.
    pub fn move_left(&self, rope: &Rope, extend: bool) -> Self {
        let new_selections = self
            .selections
            .iter()
            .map(|sel| {
                let current_char = rope.byte_to_char(sel.head.0);
                let new_char = current_char.saturating_sub(1);
                let new_head = ByteOffset(rope.char_to_byte(new_char));
                let new_anchor = if extend { sel.anchor } else { new_head };
                Selection::new(new_anchor, new_head)
            })
            .collect();

        Self::from_selections(new_selections, self.primary_index)
    }

    /// Moves all cursors one character to the right.
    pub fn move_right(&self, rope: &Rope, extend: bool) -> Self {
        let total_chars = rope.len_chars();
        let new_selections = self
            .selections
            .iter()
            .map(|sel| {
                let current_char = rope.byte_to_char(sel.head.0);
                let new_char = min(current_char.saturating_add(1), total_chars);
                let new_head = ByteOffset(rope.char_to_byte(new_char));
                let new_anchor = if extend { sel.anchor } else { new_head };
                Selection::new(new_anchor, new_head)
            })
            .collect();

        Self::from_selections(new_selections, self.primary_index)
    }

    /// Moves all cursors up one physical line.
    pub fn move_up(&self, rope: &Rope, extend: bool) -> Self {
        let new_selections = self
            .selections
            .iter()
            .map(|sel| {
                let current_char = rope.byte_to_char(sel.head.0);
                let current_line = rope.char_to_line(current_char);

                if current_line == 0 {
                    // Already on the first line, move to the beginning of the document
                    let new_head = ByteOffset(0);
                    let new_anchor = if extend { sel.anchor } else { new_head };
                    return Selection::new(new_anchor, new_head);
                }

                let target_line = current_line - 1;
                let current_line_start_char = rope.line_to_char(current_line);
                let col_offset = current_char - current_line_start_char;

                let target_line_start_char = rope.line_to_char(target_line);
                let target_line_len = rope.line(target_line).len_chars();
                // Avoid landing on newline characters if any
                let target_col = min(col_offset, target_line_len.saturating_sub(1));
                let new_char = target_line_start_char + target_col;

                let new_head = ByteOffset(rope.char_to_byte(new_char));
                let new_anchor = if extend { sel.anchor } else { new_head };
                Selection::new(new_anchor, new_head)
            })
            .collect();

        Self::from_selections(new_selections, self.primary_index)
    }

    /// Moves all cursors down one physical line.
    pub fn move_down(&self, rope: &Rope, extend: bool) -> Self {
        let total_lines = rope.len_lines();
        let new_selections = self
            .selections
            .iter()
            .map(|sel| {
                let current_char = rope.byte_to_char(sel.head.0);
                let current_line = rope.char_to_line(current_char);

                if current_line + 1 >= total_lines {
                    // Already on the last line, move to the end of the document
                    let new_head = ByteOffset(rope.len_bytes());
                    let new_anchor = if extend { sel.anchor } else { new_head };
                    return Selection::new(new_anchor, new_head);
                }

                let target_line = current_line + 1;
                let current_line_start_char = rope.line_to_char(current_line);
                let col_offset = current_char - current_line_start_char;

                let target_line_start_char = rope.line_to_char(target_line);
                let target_line_len = rope.line(target_line).len_chars();
                let target_col = min(col_offset, target_line_len.saturating_sub(1));
                let new_char = target_line_start_char + target_col;

                let new_head = ByteOffset(rope.char_to_byte(new_char));
                let new_anchor = if extend { sel.anchor } else { new_head };
                Selection::new(new_anchor, new_head)
            })
            .collect();

        Self::from_selections(new_selections, self.primary_index)
    }

    /// Moves all cursors to the beginning of their respective lines.
    pub fn move_to_line_start(&self, rope: &Rope, extend: bool) -> Self {
        let new_selections = self
            .selections
            .iter()
            .map(|sel| {
                let current_char = rope.byte_to_char(sel.head.0);
                let line_idx = rope.char_to_line(current_char);
                let line_start_char = rope.line_to_char(line_idx);
                let new_head = ByteOffset(rope.char_to_byte(line_start_char));
                let new_anchor = if extend { sel.anchor } else { new_head };
                Selection::new(new_anchor, new_head)
            })
            .collect();

        Self::from_selections(new_selections, self.primary_index)
    }

    /// Moves all cursors to the end of their respective lines.
    pub fn move_to_line_end(&self, rope: &Rope, extend: bool) -> Self {
        let new_selections = self
            .selections
            .iter()
            .map(|sel| {
                let current_char = rope.byte_to_char(sel.head.0);
                let line_idx = rope.char_to_line(current_char);
                let line_slice = rope.line(line_idx);
                let mut line_len_chars = line_slice.len_chars();

                // Exclude newline characters ('\n', '\r') from line end
                if line_len_chars > 0 && line_slice.char(line_len_chars - 1) == '\n' {
                    line_len_chars -= 1;
                    if line_len_chars > 0 && line_slice.char(line_len_chars - 1) == '\r' {
                        line_len_chars -= 1;
                    }
                }

                let line_end_char = rope.line_to_char(line_idx) + line_len_chars;
                let new_head = ByteOffset(rope.char_to_byte(line_end_char));
                let new_anchor = if extend { sel.anchor } else { new_head };
                Selection::new(new_anchor, new_head)
            })
            .collect();

        Self::from_selections(new_selections, self.primary_index)
    }

    /// Moves all cursors to the start of the document.
    pub fn move_to_doc_start(&self, extend: bool) -> Self {
        let new_head = ByteOffset(0);
        let new_anchor = if extend { self.primary().anchor } else { new_head };
        Self::single(Selection::new(new_anchor, new_head))
    }

    /// Moves all cursors to the end of the document.
    pub fn move_to_doc_end(&self, rope: &Rope, extend: bool) -> Self {
        let new_head = ByteOffset(rope.len_bytes());
        let new_anchor = if extend { self.primary().anchor } else { new_head };
        Self::single(Selection::new(new_anchor, new_head))
    }
}

impl Default for SelectionSet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collapses_and_evaluates_selection_ranges() {
        let sel = Selection::new(ByteOffset(10), ByteOffset(5));
        assert!(!sel.is_empty());
        assert!(!sel.is_forward());
        assert_eq!(sel.start(), ByteOffset(5));
        assert_eq!(sel.end(), ByteOffset(10));
        assert_eq!(sel.range(), TextRange::new(ByteOffset(5), ByteOffset(10)).unwrap());

        let collapsed = sel.collapse();
        assert!(collapsed.is_empty());
        assert_eq!(collapsed.head, ByteOffset(5));
        assert_eq!(collapsed.anchor, ByteOffset(5));
    }

    #[test]
    fn normalizes_and_merges_overlapping_selections() {
        let list = vec![
            Selection::new(ByteOffset(0), ByteOffset(5)),
            Selection::new(ByteOffset(4), ByteOffset(10)),
            Selection::new(ByteOffset(15), ByteOffset(20)),
        ];

        let set = SelectionSet::from_selections(list, 0);
        assert_eq!(set.len(), 2);
        assert_eq!(set.as_slice()[0], Selection::new(ByteOffset(0), ByteOffset(10)));
        assert_eq!(set.as_slice()[1], Selection::new(ByteOffset(15), ByteOffset(20)));
    }

    #[test]
    fn moves_cursors_left_and_right() {
        let rope = Rope::from_str("hello world");
        let set = SelectionSet::cursor(ByteOffset(5));

        let left = set.move_left(&rope, false);
        assert_eq!(left.primary().head, ByteOffset(4));

        let right = left.move_right(&rope, false);
        assert_eq!(right.primary().head, ByteOffset(5));
    }

    #[test]
    fn navigates_lines_and_boundaries() {
        let rope = Rope::from_str("first line\nsecond line\nthird line");
        let set = SelectionSet::cursor(ByteOffset(2)); // 'r' in first line

        let down = set.move_down(&rope, false);
        assert_eq!(down.primary().head, ByteOffset(13)); // 'e' in second line

        let line_end = down.move_to_line_end(&rope, false);
        assert_eq!(line_end.primary().head, ByteOffset(22)); // end of 'second line'

        let line_start = line_end.move_to_line_start(&rope, false);
        assert_eq!(line_start.primary().head, ByteOffset(11)); // start of 'second line'
    }
}
