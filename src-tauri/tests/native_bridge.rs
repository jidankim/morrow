use morrow_lib::native_bridge::{
    FakeNativeBridge, KeychainErrorCode, NativeBridgeState, PermissionKind, PermissionOutcome,
    PermissionState, TokenLookupRequest, TokenStorageSurface, TokenWriteRequest,
    MORROW_KEYCHAIN_SERVICE, MORROW_TOKEN_KIND,
};

#[test]
fn permission_status_mapper_covers_supported_states() {
    for kind in PermissionKind::ALL {
        for state in PermissionState::ALL {
            let bridge = FakeNativeBridge::with_permission(kind, state);

            let statuses = bridge.query_permission_statuses();
            let status = statuses.iter().find(|status| status.kind == kind);

            assert!(status.is_some());
            let status = status.unwrap();
            assert_eq!(status.state, state);
            match state {
                PermissionState::Granted => {
                    assert_eq!(status.outcome, PermissionOutcome::Success);
                    assert!(status.warning.is_none());
                }
                PermissionState::Denied => {
                    assert_eq!(status.outcome, PermissionOutcome::Warning);
                    assert!(status.warning.is_some());
                }
                PermissionState::Unavailable => {
                    assert_eq!(status.outcome, PermissionOutcome::Unavailable);
                    assert!(status.warning.is_some());
                }
            }
        }
    }
}

#[test]
fn denied_permissions_return_warnings_not_success() {
    let bridge = FakeNativeBridge::with_all_permissions(PermissionState::Denied);

    let statuses = bridge.query_permission_statuses();

    assert_eq!(statuses.len(), PermissionKind::ALL.len());
    for status in statuses {
        assert_eq!(status.state, PermissionState::Denied);
        assert_eq!(status.outcome, PermissionOutcome::Warning);
        assert!(status.warning.is_some());
    }
}

#[test]
fn keychain_create_read_delete_accepts_only_morrow_owned_token() -> Result<(), String> {
    let bridge = FakeNativeBridge::default();
    let lookup = TokenLookupRequest::new(MORROW_KEYCHAIN_SERVICE, MORROW_TOKEN_KIND);
    let write = TokenWriteRequest::new(
        MORROW_KEYCHAIN_SERVICE,
        MORROW_TOKEN_KIND,
        "fixture-redacted-token-value",
    );

    let saved = bridge
        .create_morrow_token(write)
        .map_err(|error| error.to_string())?;
    let read = bridge
        .read_morrow_token(lookup.clone())
        .map_err(|error| error.to_string())?;
    let deleted = bridge
        .delete_morrow_token(lookup.clone())
        .map_err(|error| error.to_string())?;
    let after_delete = bridge
        .read_morrow_token(lookup)
        .map_err(|error| error.to_string())?;

    assert_eq!(saved.storage_surface, TokenStorageSurface::KeychainBridge);
    assert!(read.present);
    assert_eq!(read.token.as_deref(), Some("fixture-redacted-token-value"));
    assert!(deleted.deleted);
    assert!(!after_delete.present);
    assert_eq!(
        after_delete.storage_surface,
        TokenStorageSurface::KeychainBridge
    );
    Ok(())
}

#[test]
fn keychain_rejects_non_morrow_service_and_unsupported_token_kind() {
    let bridge = FakeNativeBridge::default();
    let non_morrow_service = TokenWriteRequest::new(
        "com.example.not-morrow",
        MORROW_TOKEN_KIND,
        "fixture-redacted-token-value",
    );
    let unsupported_kind = TokenWriteRequest::new(
        MORROW_KEYCHAIN_SERVICE,
        "unsupported-token-kind",
        "fixture-redacted-token-value",
    );

    let service_error = bridge.create_morrow_token(non_morrow_service);
    let kind_error = bridge.create_morrow_token(unsupported_kind);

    assert_eq!(
        service_error.map_err(|error| error.code()),
        Err(KeychainErrorCode::UnsupportedService)
    );
    assert_eq!(
        kind_error.map_err(|error| error.code()),
        Err(KeychainErrorCode::UnsupportedTokenKind)
    );
}

#[test]
fn keychain_bridge_sources_do_not_contain_file_or_browser_storage_fallbacks() {
    let sources = [
        include_str!("../src/native_bridge/keychain.rs"),
        include_str!("../src/native_bridge/keychain/macos_keychain.rs"),
        include_str!("../src/native_bridge/fake.rs"),
    ];
    let forbidden_fallbacks = [
        "std::fs",
        "fs::",
        "File::",
        "OpenOptions",
        "localStorage",
        "sessionStorage",
        "IndexedDB",
    ];

    for source in sources {
        for forbidden in forbidden_fallbacks {
            assert!(
                !source.contains(forbidden),
                "native token bridge source contains forbidden fallback marker {forbidden}"
            );
        }
    }
}

#[test]
fn native_permissions_denied_command_layer_smoke() {
    let state = NativeBridgeState::with_bridge(FakeNativeBridge::with_all_permissions(
        PermissionState::Denied,
    ));

    let statuses = state.query_permission_statuses();

    assert_eq!(statuses.len(), PermissionKind::ALL.len());
    for status in statuses {
        assert_eq!(status.state, PermissionState::Denied);
        assert_eq!(status.outcome, PermissionOutcome::Warning);
        assert!(status.warning.is_some());
    }
}

#[test]
fn keychain_delete_command_layer_smoke() -> Result<(), String> {
    let state = NativeBridgeState::with_bridge(FakeNativeBridge::default());
    let lookup = TokenLookupRequest::new(MORROW_KEYCHAIN_SERVICE, MORROW_TOKEN_KIND);

    state
        .create_morrow_token(TokenWriteRequest::new(
            MORROW_KEYCHAIN_SERVICE,
            MORROW_TOKEN_KIND,
            "fixture-redacted-token-value",
        ))
        .map_err(|error| error.to_string())?;
    let before_delete = state
        .read_morrow_token(lookup.clone())
        .map_err(|error| error.to_string())?;
    let deleted = state
        .delete_morrow_token(lookup.clone())
        .map_err(|error| error.to_string())?;
    let after_delete = state
        .read_morrow_token(lookup)
        .map_err(|error| error.to_string())?;

    assert_eq!(
        before_delete.storage_surface,
        TokenStorageSurface::KeychainBridge
    );
    assert!(before_delete.present);
    assert!(deleted.deleted);
    assert_eq!(
        after_delete.storage_surface,
        TokenStorageSurface::KeychainBridge
    );
    assert!(!after_delete.present);
    Ok(())
}

#[test]
fn production_keychain_rejects_malformed_lookup_before_storage() {
    let state = NativeBridgeState::default();
    let non_morrow_service = TokenWriteRequest::new(
        "com.example.not-morrow",
        MORROW_TOKEN_KIND,
        "fixture-redacted-token-value",
    );
    let unsupported_kind = TokenWriteRequest::new(
        MORROW_KEYCHAIN_SERVICE,
        "unsupported-token-kind",
        "fixture-redacted-token-value",
    );

    let service_error = state.create_morrow_token(non_morrow_service);
    let kind_error = state.create_morrow_token(unsupported_kind);

    assert_eq!(
        service_error.map_err(|error| error.code()),
        Err(KeychainErrorCode::UnsupportedService)
    );
    assert_eq!(
        kind_error.map_err(|error| error.code()),
        Err(KeychainErrorCode::UnsupportedTokenKind)
    );
}

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
    fn new(lookup: TokenLookupRequest) -> Self {
        Self { lookup }
    }
}

impl Drop for KeychainSmokeCleanup {
    fn drop(&mut self) {
        let _ = NativeBridgeState::default().delete_morrow_token(self.lookup.clone());
    }
}
