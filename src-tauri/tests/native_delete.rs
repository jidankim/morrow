use morrow_lib::native_bridge::{
    DeleteCleanupAction, DeleteMorrowDataRequest, FakeNativeBridge, NativeBridgeState,
    TokenLookupRequest, TokenWriteRequest, MORROW_KEYCHAIN_SERVICE, MORROW_TOKEN_KIND,
};
use morrow_storage::{CandidateDraft, CandidateKind, Store};

#[path = "native_delete/diagnostics.rs"]
mod diagnostics;
#[path = "native_delete/provider_credentials.rs"]
mod provider_credentials;

#[test]
fn delete_morrow_data_request_accepts_ui_oauth_field_casing() -> Result<(), String> {
    let request = serde_json::from_str::<DeleteMorrowDataRequest>(
        r#"{
          "confirmation": "DELETE MORROW DATA",
          "cleanupProposedItems": true,
          "deleteEmptyProposalContainers": false,
          "revokeProviderOAuth": true
        }"#,
    )
    .map_err(|error| error.to_string())?;

    assert_eq!(
        request,
        DeleteMorrowDataRequest::new("DELETE MORROW DATA", true, false, true)
    );
    let serialized = serde_json::to_value(&request).map_err(|error| error.to_string())?;
    assert_eq!(
        serialized.get("revokeProviderOAuth"),
        Some(&serde_json::json!(true))
    );
    assert!(serialized.get("revokeProviderOauth").is_none());
    Ok(())
}

#[test]
fn delete_morrow_data_command_deletes_configured_store_and_returns_cleanup_receipt(
) -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let db_path = dir.path().join("native-delete.sqlite");
    let store = Store::open(&db_path).map_err(|error| error.to_string())?;
    store
        .create_candidate(CandidateDraft {
            kind: CandidateKind::CalendarEvent,
            chat_guid: "chat-native-delete".to_owned(),
            anchor_message_guid: "msg-native-delete".to_owned(),
            title: "Delete me".to_owned(),
            confidence_millis: 900,
            normalized_time: "2026-07-15T10:00:00Z".to_owned(),
            evidence_excerpt: "meet on July 15".to_owned(),
            observed_at: 1_783_000_000,
        })
        .map_err(|error| error.to_string())?;
    let state =
        NativeBridgeState::with_bridge(FakeNativeBridge::with_morrow_store_path(db_path.clone()));
    state
        .create_morrow_token(TokenWriteRequest::new(
            MORROW_KEYCHAIN_SERVICE,
            MORROW_TOKEN_KIND,
            "fixture-redacted-token-value",
        ))
        .map_err(|error| error.to_string())?;

    let receipt = state
        .delete_morrow_data(DeleteMorrowDataRequest::new(
            "DELETE MORROW DATA",
            true,
            false,
            true,
        ))
        .map_err(|error| error.to_string())?;

    assert!(receipt.database_deleted);
    assert!(!db_path.exists());
    assert!(!receipt.approved_external_items_deleted);
    assert!(receipt.provider_oauth_delete_requested);
    assert!(receipt.provider_oauth_deleted);
    assert!(!receipt.provider_oauth_delete_failed);
    assert!(receipt.provider_oauth_delete_error.is_none());
    let serialized_receipt = serde_json::to_value(&receipt).map_err(|error| error.to_string())?;
    assert_eq!(
        serialized_receipt.get("providerOAuthDeleteRequested"),
        Some(&serde_json::json!(true))
    );
    assert_eq!(
        serialized_receipt.get("providerOAuthDeleted"),
        Some(&serde_json::json!(true))
    );
    assert_eq!(
        serialized_receipt.get("providerOAuthDeleteFailed"),
        Some(&serde_json::json!(false))
    );
    assert!(serialized_receipt.get("providerOAuthDeleteError").is_none());
    assert!(serialized_receipt
        .get("providerOauthDeleteRequested")
        .is_none());
    assert!(serialized_receipt.get("providerOauthDeleted").is_none());
    assert!(serialized_receipt
        .get("providerOauthDeleteFailed")
        .is_none());
    assert_eq!(
        receipt.cleanup_plan.proposed_items,
        DeleteCleanupAction::Completed
    );
    assert_eq!(receipt.cleanup_plan.proposed_calendar_items_deleted, 0);
    assert_eq!(receipt.cleanup_plan.proposed_reminder_items_deleted, 0);
    assert_eq!(
        receipt.cleanup_plan.empty_proposal_containers,
        DeleteCleanupAction::SkippedByUser
    );
    Ok(())
}

#[test]
fn delete_morrow_data_command_succeeds_when_store_directory_is_absent() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let db_path = dir.path().join("missing-app-data").join("morrow.sqlite");
    let state =
        NativeBridgeState::with_bridge(FakeNativeBridge::with_morrow_store_path(db_path.clone()));

    let receipt = state
        .delete_morrow_data(DeleteMorrowDataRequest::new(
            "DELETE MORROW DATA",
            true,
            false,
            true,
        ))
        .map_err(|error| error.to_string())?;

    assert!(!receipt.database_deleted);
    assert!(!db_path.exists());
    assert!(receipt.provider_oauth_delete_requested);
    assert!(!receipt.provider_oauth_deleted);
    assert!(!receipt.provider_oauth_delete_failed);
    assert!(receipt.provider_oauth_delete_error.is_none());
    assert_eq!(
        receipt.cleanup_plan.proposed_items,
        DeleteCleanupAction::Completed
    );
    assert_eq!(
        receipt.cleanup_plan.empty_proposal_containers,
        DeleteCleanupAction::SkippedByUser
    );
    Ok(())
}

#[test]
fn invalid_confirmation_does_not_delete_store_or_token() -> Result<(), String> {
    // Given
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let db_path = dir.path().join("native-delete-invalid.sqlite");
    let store = Store::open(&db_path).map_err(|error| error.to_string())?;
    store
        .create_candidate(CandidateDraft {
            kind: CandidateKind::CalendarEvent,
            chat_guid: "chat-native-invalid-delete".to_owned(),
            anchor_message_guid: "msg-native-invalid-delete".to_owned(),
            title: "Keep me".to_owned(),
            confidence_millis: 900,
            normalized_time: "2026-07-15T10:00:00Z".to_owned(),
            evidence_excerpt: "meet on July 15".to_owned(),
            observed_at: 1_783_000_000,
        })
        .map_err(|error| error.to_string())?;
    let state =
        NativeBridgeState::with_bridge(FakeNativeBridge::with_morrow_store_path(db_path.clone()));
    let lookup = TokenLookupRequest::new(MORROW_KEYCHAIN_SERVICE, MORROW_TOKEN_KIND);
    state
        .create_morrow_token(TokenWriteRequest::new(
            MORROW_KEYCHAIN_SERVICE,
            MORROW_TOKEN_KIND,
            "fixture-redacted-token-value",
        ))
        .map_err(|error| error.to_string())?;

    // When
    let result = state.delete_morrow_data(DeleteMorrowDataRequest::new(
        "delete morrow data",
        true,
        false,
        true,
    ));

    // Then
    assert!(result.is_err());
    assert!(db_path.exists());
    let token = state
        .read_morrow_token(lookup)
        .map_err(|error| error.to_string())?;
    assert!(token.present);
    assert_eq!(token.token.as_deref(), Some("fixture-redacted-token-value"));
    Ok(())
}
