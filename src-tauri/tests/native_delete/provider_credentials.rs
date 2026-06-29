use morrow_lib::native_bridge::{
    DeleteMorrowDataRequest, FakeNativeBridge, NativeBridgeState, TokenLookupRequest,
    TokenWriteRequest, MORROW_KEYCHAIN_SERVICE, MORROW_PROVIDER_TOKEN_KIND, MORROW_TOKEN_KIND,
};
use morrow_storage::Store;

#[test]
fn delete_all_attempts_owned_and_provider_token_delete() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let db_path = dir.path().join("native-delete-both-tokens.sqlite");
    Store::open(&db_path).map_err(|error| error.to_string())?;
    let state =
        NativeBridgeState::with_bridge(FakeNativeBridge::with_morrow_store_path(db_path.clone()));
    let owned_lookup = TokenLookupRequest::new(MORROW_KEYCHAIN_SERVICE, MORROW_TOKEN_KIND);
    let provider_lookup =
        TokenLookupRequest::new(MORROW_KEYCHAIN_SERVICE, MORROW_PROVIDER_TOKEN_KIND);
    state
        .create_morrow_token(TokenWriteRequest::new(
            MORROW_KEYCHAIN_SERVICE,
            MORROW_TOKEN_KIND,
            "fixture-owned-redacted-token",
        ))
        .map_err(|error| error.to_string())?;
    state
        .create_morrow_token(TokenWriteRequest::new(
            MORROW_KEYCHAIN_SERVICE,
            MORROW_PROVIDER_TOKEN_KIND,
            "fixture-provider-redacted-token",
        ))
        .map_err(|error| error.to_string())?;

    let receipt = state
        .delete_morrow_data(DeleteMorrowDataRequest::new(
            "DELETE MORROW DATA",
            false,
            false,
            true,
        ))
        .map_err(|error| error.to_string())?;

    let owned_after_delete = state
        .read_morrow_token(owned_lookup)
        .map_err(|error| error.to_string())?;
    let provider_after_delete = state
        .read_morrow_token(provider_lookup)
        .map_err(|error| error.to_string())?;
    assert!(receipt.provider_oauth_delete_requested);
    assert!(receipt.provider_oauth_deleted);
    assert!(!receipt.provider_oauth_delete_failed);
    assert!(receipt.diagnostics_artifacts_deleted == false);
    assert_eq!(receipt.provider_credential_deletes.len(), 2);
    assert!(receipt
        .provider_credential_deletes
        .iter()
        .any(|receipt| receipt.token_kind == MORROW_PROVIDER_TOKEN_KIND && receipt.deleted));
    assert!(!owned_after_delete.present);
    assert!(!provider_after_delete.present);
    Ok(())
}
