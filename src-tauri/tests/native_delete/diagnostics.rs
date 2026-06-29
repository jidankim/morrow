use morrow_lib::native_bridge::{DeleteMorrowDataRequest, FakeNativeBridge, NativeBridgeState};
use morrow_storage::Store;

#[test]
fn delete_all_removes_diagnostics_artifacts() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let db_path = dir.path().join("native-delete-diagnostics.sqlite");
    let app_data_dir = db_path
        .parent()
        .ok_or_else(|| "missing app data directory".to_owned())?;
    let diagnostics_dir = app_data_dir.join("diagnostics");
    let traces_dir = diagnostics_dir.join("traces");
    let evals_dir = diagnostics_dir.join("evals");
    let exports_dir = diagnostics_dir.join("exports");
    let sibling_dir = app_data_dir.join("diagnostics-user-export");

    for artifact_dir in [&traces_dir, &evals_dir, &exports_dir, &sibling_dir] {
        std::fs::create_dir_all(artifact_dir).map_err(|error| error.to_string())?;
        std::fs::write(artifact_dir.join("artifact.jsonl"), b"diagnostic artifact")
            .map_err(|error| error.to_string())?;
    }
    Store::open(&db_path).map_err(|error| error.to_string())?;
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

    assert!(receipt.database_deleted);
    assert!(!diagnostics_dir.exists());
    assert!(sibling_dir.exists());
    let serialized_receipt = serde_json::to_value(&receipt).map_err(|error| error.to_string())?;
    assert_eq!(
        serialized_receipt.get("diagnosticsArtifactsDeleted"),
        Some(&serde_json::json!(true))
    );
    Ok(())
}
