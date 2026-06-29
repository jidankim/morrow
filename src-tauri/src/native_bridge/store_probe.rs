use std::path::Path;

use morrow_storage::StorageError;

pub(super) fn reconcile_now_at(store_path: &Path) -> Result<(), StorageError> {
    morrow_storage::Store::open(store_path).map(|_| ())
}
