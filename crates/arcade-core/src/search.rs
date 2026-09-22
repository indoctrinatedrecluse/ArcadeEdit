//! High-performance in-memory search and multi-replace primitives.

use crate::history::Transaction;
use crate::selection::SelectionSet;
use crate::{ByteOffset, Document, EditError, TextEdit, TextRange};
use ropey::Rope;

/// Finds all exact occurrences of a query string across the entire document rope.
pub fn find_all(rope: &Rope, query: &str) -> Vec<TextRange> {
    if query.is_empty() {
        return Vec::new();
    }

    let mut matches = Vec::new();
    let query_len_bytes = query.len();
    let mut current_byte = 0;

    for line in rope.lines() {
        let line_str = line.to_string();
        let mut line_offset = 0;

        while let Some(pos) = line_str[line_offset..].find(query) {
            let match_byte_start = current_byte + line_offset + pos;
            let match_byte_end = match_byte_start + query_len_bytes;

            if let Ok(range) = TextRange::new(ByteOffset(match_byte_start), ByteOffset(match_byte_end)) {
                matches.push(range);
            }
            line_offset += pos + query_len_bytes;
        }

        current_byte += line.len_bytes();
    }

    matches
}

/// Finds the next occurrence of `query` at or after `from_offset`, wrapping around if necessary.
pub fn find_next(rope: &Rope, query: &str, from_offset: ByteOffset) -> Option<TextRange> {
    let all = find_all(rope, query);
    if all.is_empty() {
        return None;
    }

    // Look for first match at or after from_offset
    for m in &all {
        if m.start >= from_offset {
            return Some(*m);
        }
    }

    // Wrap around to the beginning
    Some(all[0])
}

/// Finds the previous occurrence of `query` before `from_offset`, wrapping around if necessary.
pub fn find_prev(rope: &Rope, query: &str, from_offset: ByteOffset) -> Option<TextRange> {
    let all = find_all(rope, query);
    if all.is_empty() {
        return None;
    }

    // Look for last match before from_offset
    for m in all.iter().rev() {
        if m.end <= from_offset {
            return Some(*m);
        }
    }

    // Wrap around to the end
    Some(*all.last()?)
}

/// Replaces all occurrences of `query` in the document atomically, returning the resulting transaction.
pub fn replace_all(
    doc: &mut Document,
    query: &str,
    replacement: &str,
) -> Result<Option<Transaction>, EditError> {
    let matches = find_all(doc.rope(), query);
    if matches.is_empty() {
        return Ok(None);
    }

    let before_rev = doc.revision();
    let before_selections = SelectionSet::cursor(ByteOffset(0));

    // Prepare edits in descending offset order so earlier offsets are not displaced
    let mut edits: Vec<TextEdit> = Vec::with_capacity(matches.len());
    let mut inverse_edits: Vec<TextEdit> = Vec::with_capacity(matches.len());

    let replacement_len_bytes = replacement.len();

    for m in matches.into_iter().rev() {
        let edit = TextEdit {
            range: m,
            replacement: replacement.to_string(),
        };

        // Inverse edit covers the newly inserted replacement range
        let inv_end = ByteOffset(m.start.0 + replacement_len_bytes);
        let inverse = TextEdit {
            range: TextRange::new(m.start, inv_end)?,
            replacement: query.to_string(),
        };

        edits.push(edit);
        inverse_edits.push(inverse);
    }

    for edit in &edits {
        doc.apply(edit.clone())?;
    }

    let resulting_rev = doc.revision();
    let resulting_selections = SelectionSet::cursor(ByteOffset(0));

    let tx = Transaction::new(
        edits,
        inverse_edits,
        before_selections,
        resulting_selections,
        before_rev,
        resulting_rev,
    );

    Ok(Some(tx))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_all_occurrences_across_lines() {
        let rope = Rope::from_str("apple banana\napple orange\npear apple");
        let matches = find_all(&rope, "apple");

        assert_eq!(matches.len(), 3);
        assert_eq!(matches[0], TextRange::new(ByteOffset(0), ByteOffset(5)).unwrap());
        assert_eq!(matches[1], TextRange::new(ByteOffset(13), ByteOffset(18)).unwrap());
        assert_eq!(matches[2], TextRange::new(ByteOffset(31), ByteOffset(36)).unwrap());
    }

    #[test]
    fn wraps_find_next_and_prev() {
        let rope = Rope::from_str("foo bar foo baz foo");
        // Matches at bytes 0..3, 8..11, 16..19

        let next = find_next(&rope, "foo", ByteOffset(5)).unwrap();
        assert_eq!(next.start, ByteOffset(8));

        let wrap_next = find_next(&rope, "foo", ByteOffset(18)).unwrap();
        assert_eq!(wrap_next.start, ByteOffset(0));

        let prev = find_prev(&rope, "foo", ByteOffset(15)).unwrap();
        assert_eq!(prev.start, ByteOffset(8));

        let wrap_prev = find_prev(&rope, "foo", ByteOffset(2)).unwrap();
        assert_eq!(wrap_prev.start, ByteOffset(16));
    }

    #[test]
    fn replaces_all_occurrences_atomically() {
        let mut doc = Document::new("cats and dogs and cats");
        let tx = replace_all(&mut doc, "cats", "birds").unwrap().unwrap();

        assert_eq!(doc.to_string(), "birds and dogs and birds");
        assert_eq!(tx.edits.len(), 2);
    }
}
