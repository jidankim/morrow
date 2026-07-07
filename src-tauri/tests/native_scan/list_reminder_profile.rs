use morrow_detection::{AiProvider, ProviderError, ProviderRequest, ProviderResponse};
use morrow_lib::native_bridge::{
    scan_selected_chats_with_dependencies, ScanSelectedChatsDependencies, ScanSelectedChatsRequest,
};
use morrow_messages::TapbackKind;

use super::dependencies::{CountingProvider, RecordingProposalAdapter};
use super::message_sqlite::{
    external_mapping_count_for_source, provider_route_outcome_count, provider_route_outcome_dump,
};
use super::support::{batch, chat, query_sqlite, raw_chat, scan_request, temp_db};

#[test]
fn list_reminder_profile_disabled_stops_bare_quantity_list_before_provider() -> Result<(), String> {
    // Given
    let (_dir, store_path) = temp_db("native-scan-list-reminder-disabled.sqlite")?;
    let source =
        morrow_lib::native_bridge::FakeNativeBridge::with_morrow_store_path(store_path.clone())
            .with_messages(list_batch("msg-list-disabled")?);
    let request = list_request()?;
    let provider = CountingProvider::new(StructuredListReminderProvider);
    let adapter = RecordingProposalAdapter::default();
    let recorder = morrow_diagnostics::NoopTraceRecorder;

    // When
    let result = scan_selected_chats_with_dependencies(
        request,
        &store_path,
        ScanSelectedChatsDependencies {
            source: &source,
            provider: &provider,
            proposal_adapter: &adapter,
            trace_recorder: &recorder,
        },
    )
    .map_err(|error| error.to_string())?;

    // Then
    assert_eq!(result.created_candidate_count, 0);
    assert_eq!(result.created_external_proposal_count, 0);
    assert_eq!(provider.calls(), 0);
    assert_eq!(adapter.created_reminder_count(), 0);
    assert_eq!(provider_route_outcome_count(&store_path)?, 0);
    println!(
        "list_reminder_profile_disabled provider_calls={} candidates={} quiet={} reminder_proposals={}",
        provider.calls(),
        result.created_candidate_count,
        result.quiet_log_count,
        adapter.created_reminder_count()
    );
    Ok(())
}

#[test]
fn legacy_list_reminder_profile_cannot_enable_native_list_routing() -> Result<(), String> {
    // Given
    let (_dir, store_path) = temp_db("native-scan-list-reminder-enabled.sqlite")?;
    let source =
        morrow_lib::native_bridge::FakeNativeBridge::with_morrow_store_path(store_path.clone())
            .with_messages(list_batch("msg-list-enabled")?);
    let request = list_request()?;
    let provider = CountingProvider::new(StructuredListReminderProvider);
    let adapter = RecordingProposalAdapter::default();
    let recorder = morrow_diagnostics::NoopTraceRecorder;

    // When
    let result = scan_selected_chats_with_dependencies(
        request,
        &store_path,
        ScanSelectedChatsDependencies {
            source: &source,
            provider: &provider,
            proposal_adapter: &adapter,
            trace_recorder: &recorder,
        },
    )
    .map_err(|error| error.to_string())?;

    // Then
    assert_eq!(result.created_candidate_count, 0);
    assert_eq!(result.created_external_proposal_count, 0);
    assert_eq!(result.failed_external_proposal_count, 0);
    assert_eq!(provider.calls(), 0);
    assert!(adapter.created_titles().is_empty());
    assert_eq!(adapter.created_reminder_count(), 0);
    assert_eq!(
        external_mapping_count_for_source(&store_path, "reminders")?,
        0
    );
    assert_eq!(provider_route_outcome_count(&store_path)?, 0);
    let persisted = persisted_candidate_dump(&store_path)?;
    assert!(persisted.is_empty(), "{persisted}");
    println!(
        "legacy_list_reminder_profile_inactive provider_calls={} rendered_title=\"{}\" reminder_proposals={} persisted={} route_rows={} dump={}",
        provider.calls(),
        adapter
            .created_titles()
            .first()
            .map_or("", String::as_str),
        adapter.created_reminder_count(),
        persisted.trim(),
        provider_route_outcome_count(&store_path)?,
        provider_route_outcome_dump(&store_path)?.trim()
    );
    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct StructuredListReminderProvider;

impl AiProvider for StructuredListReminderProvider {
    fn extract(&self, _request: ProviderRequest<'_>) -> Result<ProviderResponse, ProviderError> {
        Ok(ProviderResponse::new(
            "{\"kind\":\"task_reminder\",\"title\":\"Provider list reminder\",\
             \"confidence_millis\":800,\
             \"normalized_time\":\"2026-08-01T12:34:00[Asia/Seoul]\",\
             \"anchor_evidence_id\":\"evidence://selected/0\",\
             \"evidence_ids\":[\"evidence://selected/0\"],\
             \"items\":[\
               {\"name\":\"anchovies\",\"quantity\":2,\"unit\":null,\"evidence_ids\":[\"evidence://selected/0\"]},\
               {\"name\":\"salmon\",\"quantity\":3,\"unit\":null,\"evidence_ids\":[\"evidence://selected/0\"]}\
             ]}",
        ))
    }
}

fn list_batch(message_guid: &str) -> Result<morrow_messages::NativeBatch, String> {
    Ok(batch(vec![raw_chat(
        "grocery-list-chat",
        message_guid,
        "2 anchovies, 3 salmon",
        Some(TapbackKind::Like),
    )?]))
}

fn list_request() -> Result<ScanSelectedChatsRequest, String> {
    scan_request(
        &[chat("grocery-list-chat", 3, &["p1", "p2", "p3"])],
        &[],
        true,
        1,
        0,
    )
}

fn persisted_candidate_dump(db_path: &std::path::Path) -> Result<String, String> {
    query_sqlite(
        db_path,
        "SELECT kind || '|' || title || '|' || normalized_time FROM candidates ORDER BY title;",
    )
}
