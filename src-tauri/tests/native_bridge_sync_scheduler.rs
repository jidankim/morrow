use morrow_lib::native_bridge::{
    NativeBridgeState, SyncSchedulerLastResultCommand, SyncSchedulerStateCommand,
    SyncSchedulerStatusCommand,
};

#[test]
fn sync_scheduler_state_round_trips_through_temp_store_path() -> Result<(), String> {
    // Given
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let store_path = dir.path().join("morrow.sqlite");
    let state = NativeBridgeState::default();
    let scheduled = SyncSchedulerStateCommand {
        enabled: true,
        interval_seconds: 1800,
        status: SyncSchedulerStatusCommand::Scheduled,
        last_started_at: Some(1_783_000_000),
        last_finished_at: Some(1_783_000_030),
        next_run_at: Some(1_783_001_800),
        next_eligible_at: None,
        last_result: Some(SyncSchedulerLastResultCommand::Success),
        retry_attempt: 0,
        last_reason: Some("Automatic sync scheduled.".to_owned()),
        updated_at: 1_783_000_030,
    };

    // When
    let saved = state.set_sync_scheduler_state_at(&store_path, scheduled.clone())?;
    let loaded = state.get_sync_scheduler_state_at(&store_path)?;

    // Then
    assert_eq!(saved, scheduled);
    assert_eq!(loaded, scheduled);
    Ok(())
}

#[test]
fn sync_scheduler_state_rejects_negative_retry_attempt() -> Result<(), String> {
    // Given
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let store_path = dir.path().join("morrow.sqlite");
    let state = NativeBridgeState::default();
    let invalid = SyncSchedulerStateCommand {
        enabled: true,
        interval_seconds: 1800,
        status: SyncSchedulerStatusCommand::Scheduled,
        last_started_at: None,
        last_finished_at: None,
        next_run_at: Some(1_783_001_800),
        next_eligible_at: None,
        last_result: None,
        retry_attempt: -1,
        last_reason: Some("Invalid retry attempt.".to_owned()),
        updated_at: 1_783_000_030,
    };

    // When
    let error = state.set_sync_scheduler_state_at(&store_path, invalid);

    // Then
    match error {
        Ok(_) => Err("negative retry_attempt should fail".to_owned()),
        Err(message) => {
            assert!(message.contains("retry_attempt"));
            Ok(())
        }
    }
}
