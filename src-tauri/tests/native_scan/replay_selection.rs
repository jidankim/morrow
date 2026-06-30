use morrow_lib::native_bridge::{
    scan_selected_chats_with_dependencies, ScanSelectedChatsDependencies,
};
use morrow_messages::TapbackKind;

use super::dependencies::{RecordingProposalAdapter, UnavailableTestProvider};
use super::support::{assert_counts, batch, chat, raw_chat, scan_request, temp_db};

#[test]
fn replay_selection_omits_recoverable_candidates_already_visible() -> Result<(), String> {
    // Given
    let (_dir, db_path) = temp_db("native-scan-replay-selection.sqlite")?;
    let source =
        morrow_lib::native_bridge::FakeNativeBridge::with_morrow_store_path(db_path.clone())
            .with_messages(batch(vec![raw_chat(
                "design-partners",
                "msg-replay-selection",
                "Let's meet 2026-07-15 14:00 at the private clinic.",
                Some(TapbackKind::Like),
            )?]));
    let request = scan_request(
        &[chat("design-partners", 3, &["p1", "p2", "p3"])],
        &[],
        true,
        1,
        0,
    )?;
    let provider = UnavailableTestProvider;
    let adapter = RecordingProposalAdapter::default();
    let recorder = morrow_diagnostics::NoopTraceRecorder;

    // When
    let result = scan_selected_chats_with_dependencies(
        request,
        &db_path,
        ScanSelectedChatsDependencies {
            source: &source,
            provider: &provider,
            proposal_adapter: &adapter,
            trace_recorder: &recorder,
        },
    )
    .map_err(|error| error.to_string())?;

    // Then
    assert_counts(&result, (1, 1, 0, 1, 0));
    assert_eq!(result.created_external_proposal_count, 1);
    assert_eq!(result.failed_external_proposal_count, 0);
    assert_eq!(adapter.created_count(), 1);
    Ok(())
}
