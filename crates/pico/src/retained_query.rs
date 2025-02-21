use dashmap::Entry;

use crate::{Database, DerivedNodeId, MemoRef};

/// When you call [`db.retain(&result_of_calling_memoized_function)`][Database::retain],
/// you receive a [RetainedQuery]. As long as this RetainedQuery exists, the memoized
/// function and its dependencies will not be garbage collected from the database.
///
/// You can call [`db.clear_retain`][Database::clear_retain] to stop retaining
/// the query, and you can call
/// [`retained_query.forget`][RetainedQuery::permanently_retain_query] to
/// cause the underlying memoized call to never be garbage collected.
///
/// RetainedQuery will panic if dropped without either having been passed to
/// `permanently_retain_query` or `clear_retain`.
pub struct RetainedQuery {
    pub derived_node_id: DerivedNodeId,
    pub cleared: bool,
}

impl RetainedQuery {
    /// This causes the query to be permanently retained in the database.
    pub fn permanently_retain_query(mut self) {
        // set cleared to true so that we don't panic when dropping the RetainedQuery
        self.cleared = true;
    }
}

impl std::ops::Drop for RetainedQuery {
    fn drop(&mut self) {
        if !self.cleared {
            panic!(
                "RetainedQuery dropped while still retained. If this is intentional, \
                consume the RetainedQuery by calling retained_query.forget()."
            )
        }
    }
}

impl Database {
    pub fn retain<'db, T>(&'db self, memo_ref: &MemoRef<'db, T>) -> RetainedQuery {
        debug_assert!(std::ptr::eq(self, memo_ref.db));
        match self.retained_calls.entry(memo_ref.derived_node_id) {
            Entry::Occupied(mut occupied_entry) => {
                (*occupied_entry.get_mut()) += 1;
            }
            Entry::Vacant(vacant_entry) => {
                vacant_entry.insert(1);
            }
        };
        RetainedQuery {
            derived_node_id: memo_ref.derived_node_id,
            cleared: false,
        }
    }

    pub fn clear_retain(&self, mut retained_query: RetainedQuery) {
        match self.retained_calls.entry(retained_query.derived_node_id) {
            Entry::Occupied(mut occupied_entry) => {
                (*occupied_entry.get_mut()) -= 1;
                if occupied_entry.get() == &0 {
                    occupied_entry.remove();
                }
            }
            Entry::Vacant(_) => {
                panic!("RetainedQuery not found in databse. This is indicative of a bug in Pico.")
            }
        }
        retained_query.cleared = true;
    }
}
