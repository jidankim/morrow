use std::path::{Path, PathBuf};

use crate::store::Store;
use crate::StorageError;

pub const DELETE_ALL_CONFIRMATION_TEXT: &str = "DELETE MORROW DATA";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeleteAllConfirmation {
    _private: (),
}

impl DeleteAllConfirmation {
    pub fn parse(value: &str) -> Result<Self, StorageError> {
        if value == DELETE_ALL_CONFIRMATION_TEXT {
            Ok(Self { _private: () })
        } else {
            Err(StorageError::InvalidInput {
                field: "delete_all_confirmation",
                reason: format!("must exactly match {DELETE_ALL_CONFIRMATION_TEXT}"),
            })
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeleteAllReceipt {
    pub database_deleted: bool,
    pub approved_external_items_deleted: bool,
}

pub(crate) fn remove_morrow_database(db_path: &Path) -> Result<DeleteAllReceipt, StorageError> {
    let database_deleted = remove_if_exists(db_path)?;
    for suffix in ["-wal", "-shm", "-journal"] {
        if let Some(path) = companion_path(db_path, suffix) {
            remove_if_exists(&path)?;
        }
    }

    Ok(DeleteAllReceipt {
        database_deleted,
        approved_external_items_deleted: false,
    })
}

impl Store {
    pub fn delete_all(
        self,
        confirmation: DeleteAllConfirmation,
    ) -> Result<DeleteAllReceipt, StorageError> {
        delete_all_at(&self.db_path, confirmation)
    }
}

pub fn delete_all_at(
    db_path: &Path,
    confirmation: DeleteAllConfirmation,
) -> Result<DeleteAllReceipt, StorageError> {
    let _confirmation = confirmation;
    remove_morrow_database(db_path)
}

fn remove_if_exists(path: &Path) -> Result<bool, StorageError> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(StorageError::Io(error)),
    }
}

fn companion_path(db_path: &Path, suffix: &str) -> Option<PathBuf> {
    let file_name = db_path.file_name()?.to_str()?;
    Some(db_path.with_file_name(format!("{file_name}{suffix}")))
}
