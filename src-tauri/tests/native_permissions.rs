use morrow_lib::native_bridge::{
    FakeNativeBridge, NativeBridgeState, PermissionKind, PermissionOutcome, PermissionState,
};

#[test]
fn permission_status_mapper_covers_supported_states() {
    for kind in PermissionKind::ALL {
        for state in PermissionState::ALL {
            let bridge = FakeNativeBridge::with_permission(kind, state);

            let statuses = bridge.query_permission_statuses();
            let status = statuses
                .iter()
                .find(|status| status.kind == kind)
                .expect("permission status exists");

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
