use std::path::Path;

use morrow_storage::{delete_all_at, DeleteAllConfirmation};

use super::{
    delete_all_protocol::{
        DeleteCleanupAction, DeleteCleanupPlan, DeleteMorrowDataError, DeleteMorrowDataRequest,
        MorrowDataDeleteReceipt, MorrowDataStorageSurface,
    },
    delete_provider_credentials::{
        provider_credential_delete_receipts, provider_oauth_delete_receipt, MorrowTokenDeleter,
    },
    eventkit_cleanup::{ProposedItemCleaner, ProposedItemCleanupReceipt},
};

pub(crate) fn delete_morrow_data_at(
    request: DeleteMorrowDataRequest,
    store_path: &Path,
    token_vault: &impl MorrowTokenDeleter,
    proposed_item_cleaner: &impl ProposedItemCleaner,
) -> Result<MorrowDataDeleteReceipt, DeleteMorrowDataError> {
    let confirmation = DeleteAllConfirmation::parse(&request.confirmation)?;

    let proposed_items = if request.cleanup_proposed_items {
        proposed_item_cleaner.cleanup_proposed_items()?
    } else {
        ProposedItemCleanupReceipt {
            calendar_items_deleted: 0,
            reminder_items_deleted: 0,
        }
    };
    let delete_receipt = delete_all_at(store_path, confirmation)?;
    let provider_credentials =
        provider_credential_delete_receipts(request.revoke_provider_oauth, token_vault);
    let provider_oauth = provider_oauth_delete_receipt(&provider_credentials);

    Ok(MorrowDataDeleteReceipt {
        storage_surface: MorrowDataStorageSurface::MorrowStore,
        database_deleted: delete_receipt.database_deleted,
        approved_external_items_deleted: delete_receipt.approved_external_items_deleted,
        provider_oauth_delete_requested: request.revoke_provider_oauth,
        provider_oauth_deleted: provider_oauth.deleted,
        provider_oauth_delete_failed: provider_oauth.failed,
        provider_oauth_delete_error: provider_oauth.error,
        provider_credential_deletes: provider_credentials,
        cleanup_plan: cleanup_plan(&request, proposed_items),
    })
}

fn cleanup_plan(
    request: &DeleteMorrowDataRequest,
    proposed_items: ProposedItemCleanupReceipt,
) -> DeleteCleanupPlan {
    DeleteCleanupPlan {
        proposed_items: proposed_items_action(request.cleanup_proposed_items),
        empty_proposal_containers: cleanup_action(request.delete_empty_proposal_containers),
        proposed_calendar_items_deleted: proposed_items.calendar_items_deleted,
        proposed_reminder_items_deleted: proposed_items.reminder_items_deleted,
    }
}

const fn cleanup_action(requested: bool) -> DeleteCleanupAction {
    if requested {
        DeleteCleanupAction::AdapterDeferred
    } else {
        DeleteCleanupAction::SkippedByUser
    }
}

const fn proposed_items_action(requested: bool) -> DeleteCleanupAction {
    if requested {
        DeleteCleanupAction::Completed
    } else {
        DeleteCleanupAction::SkippedByUser
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use morrow_storage::StorageError;

    use super::*;
    use crate::native_bridge::eventkit_cleanup::NoopProposedItemCleaner;

    #[derive(Debug)]
    struct FailingTokenVault;

    impl MorrowTokenDeleter for FailingTokenVault {
        fn delete_morrow_token(
            &self,
            _request: TokenLookupRequest,
        ) -> Result<bool, KeychainBridgeError> {
            Err(KeychainBridgeError::storage_unavailable(
                "Morrow Keychain delete failed with OSStatus -60008",
            ))
        }
    }

    #[test]
    fn token_revoke_failure_does_not_block_local_store_delete() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("morrow.sqlite");
        std::fs::write(&db_path, b"db").expect("write db");

        let receipt = delete_morrow_data_at(
            DeleteMorrowDataRequest::new("DELETE MORROW DATA", false, false, true),
            &db_path,
            &FailingTokenVault,
            &NoopProposedItemCleaner,
        )
        .expect("delete should continue after token failure");

        assert!(receipt.database_deleted);
        assert!(!db_path.exists());
        assert!(receipt.provider_oauth_delete_requested);
        assert!(!receipt.provider_oauth_deleted);
        assert!(receipt.provider_oauth_delete_failed);
        assert!(receipt
            .provider_oauth_delete_error
            .as_deref()
            .is_some_and(|message| message.contains("-60008")));
    }

    #[derive(Debug)]
    struct CountingProposedCleaner;

    impl ProposedItemCleaner for CountingProposedCleaner {
        fn cleanup_proposed_items(
            &self,
        ) -> Result<ProposedItemCleanupReceipt, DeleteMorrowDataError> {
            Ok(ProposedItemCleanupReceipt {
                calendar_items_deleted: 2,
                reminder_items_deleted: 3,
            })
        }
    }

    #[test]
    fn requested_proposed_cleanup_is_reflected_in_receipt() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("morrow.sqlite");

        let receipt = delete_morrow_data_at(
            DeleteMorrowDataRequest::new("DELETE MORROW DATA", true, false, false),
            &db_path,
            &FakeMorrowTokenVault::default(),
            &CountingProposedCleaner,
        )
        .expect("delete receipt");

        assert_eq!(
            receipt.cleanup_plan.proposed_items,
            DeleteCleanupAction::Completed
        );
        assert_eq!(receipt.cleanup_plan.proposed_calendar_items_deleted, 2);
        assert_eq!(receipt.cleanup_plan.proposed_reminder_items_deleted, 3);
    }

    #[derive(Debug)]
    struct DeletingProposedCleaner {
        path: PathBuf,
    }

    impl ProposedItemCleaner for DeletingProposedCleaner {
        fn cleanup_proposed_items(
            &self,
        ) -> Result<ProposedItemCleanupReceipt, DeleteMorrowDataError> {
            std::fs::remove_file(&self.path)
                .map_err(|error| DeleteMorrowDataError::from(StorageError::Io(error)))?;
            Ok(ProposedItemCleanupReceipt {
                calendar_items_deleted: 1,
                reminder_items_deleted: 1,
            })
        }
    }

    #[test]
    fn invalid_confirmation_blocks_proposed_cleanup_before_side_effects() {
        let dir = tempfile::tempdir().expect("tempdir");
        let marker_path = dir.path().join("proposed-marker");
        std::fs::write(&marker_path, b"keep").expect("write marker");

        let result = delete_morrow_data_at(
            DeleteMorrowDataRequest::new("delete morrow data", true, false, false),
            &dir.path().join("morrow.sqlite"),
            &FakeMorrowTokenVault::default(),
            &DeletingProposedCleaner {
                path: marker_path.clone(),
            },
        );

        assert!(result.is_err());
        assert!(marker_path.exists());
    }
}
