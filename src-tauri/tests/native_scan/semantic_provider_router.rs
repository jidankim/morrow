use morrow_lib::native_bridge::{
    scan_selected_chats_with_dependencies, ScanSelectedChatsDependencies,
};

use super::dependencies::{CountingProvider, RecordingProposalAdapter, UnavailableTestProvider};
use super::message_sqlite::{
    external_mapping_count, external_mapping_count_for_source, provider_route_outcome_count,
    provider_route_outcome_dump,
};
use super::provider_route_support::assert_provider_route_dump_hides;
use super::scheduling_intent_support::{messages_request, CalendarTitleProvider, MessagesFixture};
use super::support::assert_counts;

const CAL_PROVIDER_015: &str = "Let's do the contract readout at 10 on July 16.";
const CONTRACT_READOUT_TITLE: &str = "Contract readout";
const CONTRACT_READOUT_TIME: &str = "2026-07-16T10:00:00[Asia/Seoul]";
const REM_PROVIDER_004: &str = "I owe Sam the mockups by next Wednesday.";
const REMINDER_TITLE: &str = "Send Sam the mockups";
const REMINDER_TIME: &str = "2026-07-15T09:00:00[Asia/Seoul]";
const EXPLICIT_FALLBACK: &str = "Send the renewal packet by July 25, 2026";
const VAGUE_FALLBACK: &str = "Follow up with Dana by Friday";
const CONTEXT_ONLY_FALLBACK: &str = "A: Friday morning still good?\nB: For the budget sync, yes.";

#[test]
fn semantic_provider_router_cal_provider_015_keeps_safe_title_and_private_ledger(
) -> Result<(), String> {
    // Given
    let fixture = MessagesFixture::with_text(CAL_PROVIDER_015)?;
    let provider = CountingProvider::new(CalendarTitleProvider::new(
        CONTRACT_READOUT_TITLE,
        CONTRACT_READOUT_TIME,
    ));
    let adapter = RecordingProposalAdapter::default();

    // When
    let result = scan_semantic_fixture(&fixture, &provider, &adapter)?;

    // Then
    assert_counts(&result, (1, 1, 0, 1, 0));
    assert_eq!(provider.calls(), 1);
    assert_eq!(adapter.created_titles(), [CONTRACT_READOUT_TITLE]);
    assert_eq!(external_mapping_count(&fixture.store_path)?, 1);
    assert_eq!(provider_route_outcome_count(&fixture.store_path)?, 1);
    let dump = provider_route_outcome_dump(&fixture.store_path)?;
    assert!(dump.contains("|Messages event candidate|"), "{dump}");
    assert_provider_route_dump_hides(
        &dump,
        &[
            CAL_PROVIDER_015,
            CONTRACT_READOUT_TITLE,
            "{\"kind\"",
            "\"title\"",
            "\"calendar_event\"",
        ],
    );
    println!(
        "cal_provider_015_native provider_calls={} visible_title=\"{}\" external_mappings={} route_rows={} ledger={}",
        provider.calls(),
        adapter.created_titles().join(","),
        external_mapping_count(&fixture.store_path)?,
        provider_route_outcome_count(&fixture.store_path)?,
        dump.trim()
    );
    Ok(())
}

#[test]
fn semantic_provider_router_reminder_keeps_safe_title_and_private_ledger() -> Result<(), String> {
    // Given
    let fixture = MessagesFixture::with_text(REM_PROVIDER_004)?;
    let provider = CountingProvider::new(TaskReminderTitleProvider);
    let adapter = RecordingProposalAdapter::default();

    // When
    let result = scan_semantic_fixture(&fixture, &provider, &adapter)?;

    // Then
    assert_counts(&result, (1, 1, 0, 1, 0));
    assert_eq!(provider.calls(), 1);
    assert_eq!(adapter.created_titles(), [REMINDER_TITLE]);
    assert_eq!(adapter.created_reminder_count(), 1);
    assert_eq!(
        external_mapping_count_for_source(&fixture.store_path, "reminders")?,
        1
    );
    assert_eq!(provider_route_outcome_count(&fixture.store_path)?, 1);
    let dump = provider_route_outcome_dump(&fixture.store_path)?;
    assert!(dump.contains("|Messages event candidate|"), "{dump}");
    assert_provider_route_dump_hides(
        &dump,
        &[
            REM_PROVIDER_004,
            REMINDER_TITLE,
            "{\"kind\"",
            "\"title\"",
            "\"task_reminder\"",
        ],
    );
    println!(
        "rem_provider_004_native provider_calls={} visible_title=\"{}\" reminder_mappings={} route_rows={} ledger={}",
        provider.calls(),
        adapter.created_titles().join(","),
        external_mapping_count_for_source(&fixture.store_path, "reminders")?,
        provider_route_outcome_count(&fixture.store_path)?,
        dump.trim()
    );
    Ok(())
}

#[test]
fn semantic_provider_router_provider_unavailable_uses_safe_fallback_or_quiet() -> Result<(), String>
{
    // Given / When
    let explicit = MessagesFixture::with_text(EXPLICIT_FALLBACK)?;
    let explicit_provider = CountingProvider::new(UnavailableTestProvider);
    let explicit_adapter = RecordingProposalAdapter::default();
    let explicit_result = scan_semantic_fixture(&explicit, &explicit_provider, &explicit_adapter)?;

    let vague = MessagesFixture::with_text(VAGUE_FALLBACK)?;
    let vague_provider = CountingProvider::new(UnavailableTestProvider);
    let vague_adapter = RecordingProposalAdapter::default();
    let vague_result = scan_semantic_fixture(&vague, &vague_provider, &vague_adapter)?;

    let context = MessagesFixture::with_text(CONTEXT_ONLY_FALLBACK)?;
    let context_provider = CountingProvider::new(UnavailableTestProvider);
    let context_adapter = RecordingProposalAdapter::default();
    let context_result = scan_semantic_fixture(&context, &context_provider, &context_adapter)?;

    // Then
    assert_counts(&explicit_result, (1, 1, 0, 1, 0));
    assert_eq!(explicit_provider.calls(), 1);
    assert_eq!(explicit_adapter.created_titles(), [EXPLICIT_FALLBACK]);
    assert_eq!(
        external_mapping_count_for_source(&explicit.store_path, "reminders")?,
        1
    );
    assert_eq!(provider_route_outcome_count(&explicit.store_path)?, 0);

    assert_counts(&vague_result, (0, 0, 1, 0, 0));
    assert_eq!(vague_provider.calls(), 1);
    assert!(vague_adapter.created_titles().is_empty());
    assert_eq!(
        external_mapping_count_for_source(&vague.store_path, "reminders")?,
        0
    );
    assert_eq!(provider_route_outcome_count(&vague.store_path)?, 0);
    assert_counts(&context_result, (0, 0, 1, 0, 0));
    assert_eq!(context_provider.calls(), 1);
    assert!(context_adapter.created_titles().is_empty());
    assert_eq!(
        external_mapping_count_for_source(&context.store_path, "reminders")?,
        0
    );
    assert_eq!(provider_route_outcome_count(&context.store_path)?, 0);
    println!(
        "provider_unavailable_native explicit_title=\"{}\" explicit_route_rows={} vague_created={} vague_route_rows={} context_created={} context_route_rows={}",
        explicit_adapter.created_titles().join(","),
        provider_route_outcome_count(&explicit.store_path)?,
        vague_adapter.created_titles().len(),
        provider_route_outcome_count(&vague.store_path)?,
        context_adapter.created_titles().len(),
        provider_route_outcome_count(&context.store_path)?
    );
    Ok(())
}

#[test]
fn semantic_provider_router_reminder_replay_failure_does_not_cache_route() -> Result<(), String> {
    // Given
    let fixture = MessagesFixture::with_text(REM_PROVIDER_004)?;
    let provider = CountingProvider::new(TaskReminderTitleProvider);
    let adapter = RecordingProposalAdapter::failing_reminder_permission_denied();

    // When
    let result = scan_semantic_fixture(&fixture, &provider, &adapter)?;

    // Then
    assert_counts(&result, (1, 1, 0, 1, 0));
    assert_eq!(provider.calls(), 1);
    assert_eq!(result.created_external_proposal_count, 0);
    assert_eq!(result.failed_external_proposal_count, 1);
    assert_eq!(adapter.created_reminder_count(), 0);
    assert_eq!(
        external_mapping_count_for_source(&fixture.store_path, "reminders")?,
        0
    );
    assert_eq!(provider_route_outcome_count(&fixture.store_path)?, 0);
    println!(
        "rem_provider_004_replay_failure provider_calls={} failed_external={} reminder_mappings={} route_rows={}",
        provider.calls(),
        result.failed_external_proposal_count,
        external_mapping_count_for_source(&fixture.store_path, "reminders")?,
        provider_route_outcome_count(&fixture.store_path)?
    );
    Ok(())
}

fn scan_semantic_fixture<P>(
    fixture: &MessagesFixture,
    provider: &P,
    adapter: &RecordingProposalAdapter,
) -> Result<morrow_lib::native_bridge::ScanSelectedChatsResult, String>
where
    P: morrow_detection::AiProvider,
{
    let source = morrow_lib::native_bridge::messages_sqlite::MessagesSqliteAdapter::new(
        fixture.messages_db_path.clone(),
    );
    let recorder = morrow_diagnostics::NoopTraceRecorder;
    scan_selected_chats_with_dependencies(
        messages_request()?,
        &fixture.store_path,
        ScanSelectedChatsDependencies {
            source: &source,
            provider,
            proposal_adapter: adapter,
            trace_recorder: &recorder,
        },
    )
    .map_err(|error| error.to_string())
}

#[derive(Debug, Clone, Copy)]
struct TaskReminderTitleProvider;

impl morrow_detection::AiProvider for TaskReminderTitleProvider {
    fn extract(
        &self,
        _request: morrow_detection::ProviderRequest<'_>,
    ) -> Result<morrow_detection::ProviderResponse, morrow_detection::ProviderError> {
        let response = serde_json::json!({
            "kind": "task_reminder",
            "title": REMINDER_TITLE,
            "confidence_millis": 820,
            "normalized_time": REMINDER_TIME,
            "anchor_message_guid": "beta-provider-route",
            "evidence_message_guids": ["beta-provider-route"],
        });
        Ok(morrow_detection::ProviderResponse::new(
            &response.to_string(),
        ))
    }
}
