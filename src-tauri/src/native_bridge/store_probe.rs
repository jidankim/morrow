use std::path::Path;

use morrow_storage::StorageError;

pub(super) fn reconcile_now_at(store_path: &Path) -> Result<(), StorageError> {
    morrow_storage::Store::open(store_path).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::reconcile_now_at;

    #[test]
    fn reconcile_now_creates_missing_app_data_directory_before_probe() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store_path = dir
            .path()
            .join("Application Support")
            .join("dev.morrow.desktop")
            .join("morrow.sqlite");
        let app_data_dir = store_path.parent().expect("store path has parent");

        assert!(!app_data_dir.exists());
        reconcile_now_at(&store_path).expect("sync probe should initialize missing app data store");

        assert!(app_data_dir.is_dir());
        assert!(store_path.is_file());
    }
}
