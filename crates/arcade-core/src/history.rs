//! Transaction tracking and reversible undo/redo history.

use crate::selection::SelectionSet;
use crate::{Document, EditError, Revision, TextEdit};

/// An atomic collection of edits and selection changes applied together.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Transaction {
    /// Edits applied forward to transition from the prior document to the new document.
    pub edits: Vec<TextEdit>,
    /// Inverse edits applied to restore the previous document state from the new state.
    pub inverse_edits: Vec<TextEdit>,
    /// Selection set prior to applying the transaction.
    pub before_selections: SelectionSet,
    /// Resulting selection set after applying the transaction.
    pub after_selections: SelectionSet,
    /// Revision of the document prior to the transaction.
    pub previous_revision: Revision,
    /// Revision of the document after the transaction.
    pub resulting_revision: Revision,
}

impl Transaction {
    /// Constructs a new transaction.
    pub fn new(
        edits: Vec<TextEdit>,
        inverse_edits: Vec<TextEdit>,
        before_selections: SelectionSet,
        after_selections: SelectionSet,
        previous_revision: Revision,
        resulting_revision: Revision,
    ) -> Self {
        Self {
            edits,
            inverse_edits,
            before_selections,
            after_selections,
            previous_revision,
            resulting_revision,
        }
    }

    /// Returns the reverse transaction to undo this transaction.
    pub fn invert(&self) -> Self {
        Self {
            edits: self.inverse_edits.clone(),
            inverse_edits: self.edits.clone(),
            before_selections: self.after_selections.clone(),
            after_selections: self.before_selections.clone(),
            previous_revision: self.resulting_revision,
            resulting_revision: self.previous_revision,
        }
    }
}

/// Manages undo and redo stacks for a document with clean/dirty tracking.
#[derive(Clone, Debug)]
pub struct History {
    undo_stack: Vec<Transaction>,
    redo_stack: Vec<Transaction>,
    saved_revision: Option<Revision>,
    max_entries: usize,
}

impl History {
    /// Creates a new empty history stack with default bounds.
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            saved_revision: Some(Revision(0)),
            max_entries: 500,
        }
    }

    /// Sets the maximum number of undo steps preserved in memory.
    pub fn with_capacity(max_entries: usize) -> Self {
        Self {
            undo_stack: Vec::with_capacity(max_entries),
            redo_stack: Vec::new(),
            saved_revision: Some(Revision(0)),
            max_entries,
        }
    }

    /// Records a forward transaction into the undo stack and clears the redo stack.
    pub fn push(&mut self, transaction: Transaction) {
        if self.undo_stack.len() >= self.max_entries {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(transaction);
        self.redo_stack.clear();
    }

    /// Reverts the most recent transaction on the document and returns the restored selection.
    pub fn undo(&mut self, doc: &mut Document) -> Result<Option<SelectionSet>, EditError> {
        let Some(transaction) = self.undo_stack.pop() else {
            return Ok(None);
        };

        // Apply inverse edits in descending offset order
        let mut inverse_edits = transaction.inverse_edits.clone();
        inverse_edits.sort_by_key(|e| std::cmp::Reverse(e.range.start));

        for edit in inverse_edits {
            doc.apply(edit)?;
        }

        let restored_selections = transaction.before_selections.clone();
        self.redo_stack.push(transaction);

        Ok(Some(restored_selections))
    }

    /// Re-applies the most recently undone transaction and returns the resulting selection.
    pub fn redo(&mut self, doc: &mut Document) -> Result<Option<SelectionSet>, EditError> {
        let Some(transaction) = self.redo_stack.pop() else {
            return Ok(None);
        };

        // Apply forward edits in descending offset order
        let mut edits = transaction.edits.clone();
        edits.sort_by_key(|e| std::cmp::Reverse(e.range.start));

        for edit in edits {
            doc.apply(edit)?;
        }

        let restored_selections = transaction.after_selections.clone();
        self.undo_stack.push(transaction);

        Ok(Some(restored_selections))
    }

    /// Marks the given revision as clean (e.g. freshly loaded or saved to disk).
    pub fn mark_saved(&mut self, revision: Revision) {
        self.saved_revision = Some(revision);
    }

    /// Reports whether the current document revision differs from the saved revision.
    pub fn is_dirty(&self, current_revision: Revision) -> bool {
        self.saved_revision != Some(current_revision)
    }

    /// Returns `true` if there are transactions that can be undone.
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Returns `true` if there are transactions that can be redone.
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }
}

impl Default for History {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::selection::Selection;
    use crate::{ByteOffset, TextRange};

    #[test]
    fn tracks_undo_and_redo_cycles() {
        let mut doc = Document::new("hello");
        let mut history = History::new();

        let edit = TextEdit {
            range: TextRange::new(ByteOffset(5), ByteOffset(5)).unwrap(),
            replacement: " world".into(),
        };
        let inverse = TextEdit {
            range: TextRange::new(ByteOffset(5), ByteOffset(11)).unwrap(),
            replacement: "".into(),
        };

        let tx = Transaction::new(
            vec![edit],
            vec![inverse],
            SelectionSet::cursor(ByteOffset(5)),
            SelectionSet::cursor(ByteOffset(11)),
            doc.revision(),
            Revision(1),
        );

        doc.apply(tx.edits[0].clone()).unwrap();
        history.push(tx);

        assert_eq!(doc.to_string(), "hello world");
        assert!(history.is_dirty(doc.revision()));

        // Undo
        let undone_sel = history.undo(&mut doc).unwrap().unwrap();
        assert_eq!(doc.to_string(), "hello");
        assert_eq!(undone_sel.primary(), Selection::cursor(ByteOffset(5)));

        // Redo
        let redone_sel = history.redo(&mut doc).unwrap().unwrap();
        assert_eq!(doc.to_string(), "hello world");
        assert_eq!(redone_sel.primary(), Selection::cursor(ByteOffset(11)));
    }
}
