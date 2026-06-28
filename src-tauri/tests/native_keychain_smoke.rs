use morrow_lib::native_bridge::{
    NativeBridgeState, TokenLookupRequest, TokenStorageSurface, TokenWriteRequest,
    MORROW_KEYCHAIN_SERVICE, MORROW_TOKEN_KIND,
};

#[test]
#[ignore = "touches the user's real macOS Keychain; run only for manual native storage smoke"]
fn production_keychain_persists_across_native_bridge_instances() -> Result<(), String> {
    let lookup = TokenLookupRequest::new(MORROW_KEYCHAIN_SERVICE, MORROW_TOKEN_KIND);
    let cleanup = KeychainSmokeCleanup::new(lookup.clone());
    let writer = NativeBridgeState::default();
    writer
        .delete_morrow_token(lookup.clone())
        .map_err(|error| error.to_string())?;

    writer
        .create_morrow_token(TokenWriteRequest::new(
            MORROW_KEYCHAIN_SERVICE,
            MORROW_TOKEN_KIND,
            "fixture-redacted-token-value",
        ))
        .map_err(|error| error.to_string())?;
    let read_from_new_state = NativeBridgeState::default()
        .read_morrow_token(lookup.clone())
        .map_err(|error| error.to_string())?;
    NativeBridgeState::default()
        .delete_morrow_token(lookup.clone())
        .map_err(|error| error.to_string())?;
    let after_delete = NativeBridgeState::default()
        .read_morrow_token(lookup)
        .map_err(|error| error.to_string())?;

    assert_eq!(
        read_from_new_state.storage_surface,
        TokenStorageSurface::KeychainBridge
    );
    assert!(read_from_new_state.present);
    assert_eq!(
        read_from_new_state.token.as_deref(),
        Some("fixture-redacted-token-value")
    );
    assert!(!after_delete.present);
    drop(cleanup);
    Ok(())
}

struct KeychainSmokeCleanup {
    lookup: TokenLookupRequest,
}

impl KeychainSmokeCleanup {
    const fn new(lookup: TokenLookupRequest) -> Self {
        Self { lookup }
    }
}

impl Drop for KeychainSmokeCleanup {
    fn drop(&mut self) {
        let _ = NativeBridgeState::default().delete_morrow_token(self.lookup.clone());
    }
}
