use morrow_lib::native_bridge::{
    scan_selected_chats_with_dependencies, ScanSelectedChatsDependencies, ScanSelectedChatsRequest,
};
use serde_json::json;

use super::dependencies::{CandidateProvider, CountingProvider, RecordingProposalAdapter};
use super::message_sqlite::{
    corrupt_provider_candidate_schema_version, corrupt_provider_route_contract_version,
    create_messages_fixture, provider_route_fingerprint_count, provider_route_outcome_count,
    provider_route_outcome_dump, set_provider_route_prompt_version,
    update_provider_route_message_text,
};
use super::support::{chat, scan_request};

#[test]
fn provider_route_ledger_invalidation() -> Result<(), String> {
    // Given
    let fixture = ProviderRouteFixture::new()?;
    let request = provider_route_request()?;
    let provider = CountingProvider::new(CandidateProvider);
    let adapter = RecordingProposalAdapter::default();
    let recorder = morrow_diagnostics::NoopTraceRecorder;

    // When
    scan_provider_route(&fixture, request.clone(), &provider, &adapter, &recorder)?;
    scan_provider_route(&fixture, request.clone(), &provider, &adapter, &recorder)?;

    // Then
    assert_eq!(provider.calls(), 1);
    assert_eq!(provider_route_outcome_count(&fixture.store_path)?, 1);

    // When
    update_provider_route_message_text(
        &fixture.messages_db_path,
        "Maybe meet the project team tomorrow?",
    )?;
    scan_provider_route(&fixture, request, &provider, &adapter, &recorder)?;

    // Then
    assert_eq!(provider.calls(), 2);
    assert_eq!(provider_route_fingerprint_count(&fixture.store_path)?, 2);

    // When
    let hidden_excerpt_request =
        provider_route_request_with_options(false, "Asia/Seoul", 1_782_352_400)?;
    scan_provider_route(
        &fixture,
        hidden_excerpt_request.clone(),
        &provider,
        &adapter,
        &recorder,
    )?;
    scan_provider_route(
        &fixture,
        hidden_excerpt_request,
        &provider,
        &adapter,
        &recorder,
    )?;

    // Then
    assert_eq!(provider.calls(), 3);
    assert_eq!(provider_route_fingerprint_count(&fixture.store_path)?, 3);

    // When
    let utc_reference_request = provider_route_request_with_options(false, "UTC", 1_782_352_400)?;
    scan_provider_route(
        &fixture,
        utc_reference_request.clone(),
        &provider,
        &adapter,
        &recorder,
    )?;
    scan_provider_route(
        &fixture,
        utc_reference_request,
        &provider,
        &adapter,
        &recorder,
    )?;

    // Then
    assert_eq!(provider.calls(), 4);
    assert_eq!(provider_route_fingerprint_count(&fixture.store_path)?, 4);
    println!(
        "invalidation_provider_calls={} fingerprint_rows={}",
        provider.calls(),
        provider_route_fingerprint_count(&fixture.store_path)?
    );
    Ok(())
}

#[test]
fn provider_route_ledger_records_los_angeles_timezone_reference() -> Result<(), String> {
    // Given
    let fixture = ProviderRouteFixture::new()?;
    let mut request = provider_route_request()?;
    request.reference_timezone = "America/Los_Angeles".to_owned();
    let provider = CountingProvider::new(CandidateProvider);
    let adapter = RecordingProposalAdapter::default();
    let recorder = morrow_diagnostics::NoopTraceRecorder;

    // When
    scan_provider_route(&fixture, request, &provider, &adapter, &recorder)?;

    // Then
    let dump = provider_route_outcome_dump(&fixture.store_path)?;
    assert!(
        dump.contains("2026-06-24T18:53:00[America/Los_Angeles]|America/Los_Angeles"),
        "{dump}"
    );
    Ok(())
}

#[test]
fn scheduling_intent_prompt_version_scan_v1_row_invalidates_under_scan_v2() -> Result<(), String> {
    // Given
    let fixture = ProviderRouteFixture::new()?;
    let request = provider_route_request()?;
    let provider = CountingProvider::new(CandidateProvider);
    let adapter = RecordingProposalAdapter::default();
    let recorder = morrow_diagnostics::NoopTraceRecorder;
    scan_provider_route(&fixture, request.clone(), &provider, &adapter, &recorder)?;
    set_provider_route_prompt_version(&fixture.store_path, "scan-v1")?;

    // When
    scan_provider_route(&fixture, request, &provider, &adapter, &recorder)?;

    // Then
    assert_eq!(provider.calls(), 2);
    assert_eq!(provider_route_outcome_count(&fixture.store_path)?, 1);
    let dump = provider_route_outcome_dump(&fixture.store_path)?;
    assert!(dump.contains("|scan-v2|"), "{dump}");
    assert!(!dump.contains("|scan-v1|"), "{dump}");
    println!(
        "prompt_version_invalidation provider_calls={} rows={} dump={}",
        provider.calls(),
        provider_route_outcome_count(&fixture.store_path)?,
        dump
    );
    Ok(())
}

#[test]
fn provider_route_ledger_contract_and_schema_mismatch_invalidates() -> Result<(), String> {
    // Given
    let contract_fixture = ProviderRouteFixture::new()?;
    let request = provider_route_request()?;
    let contract_provider = CountingProvider::new(CandidateProvider);
    let adapter = RecordingProposalAdapter::default();
    let recorder = morrow_diagnostics::NoopTraceRecorder;
    scan_provider_route(
        &contract_fixture,
        request.clone(),
        &contract_provider,
        &adapter,
        &recorder,
    )?;
    corrupt_provider_route_contract_version(&contract_fixture.store_path)?;

    // When
    scan_provider_route(
        &contract_fixture,
        request.clone(),
        &contract_provider,
        &adapter,
        &recorder,
    )?;

    // Then
    assert_eq!(contract_provider.calls(), 2);
    assert!(
        provider_route_outcome_dump(&contract_fixture.store_path)?
            .contains("provider-route-ledger-v1"),
        "contract version was not refreshed"
    );

    // Given
    let schema_fixture = ProviderRouteFixture::new()?;
    let schema_provider = CountingProvider::new(CandidateProvider);
    scan_provider_route(
        &schema_fixture,
        request.clone(),
        &schema_provider,
        &adapter,
        &recorder,
    )?;
    corrupt_provider_candidate_schema_version(&schema_fixture.store_path)?;

    // When
    scan_provider_route(
        &schema_fixture,
        request,
        &schema_provider,
        &adapter,
        &recorder,
    )?;

    // Then
    assert_eq!(schema_provider.calls(), 2);
    assert!(
        provider_route_outcome_dump(&schema_fixture.store_path)?
            .contains("provider-candidate-schema-v2"),
        "candidate schema version was not refreshed"
    );
    Ok(())
}

struct ProviderRouteFixture {
    _dir: tempfile::TempDir,
    store_path: std::path::PathBuf,
    messages_db_path: std::path::PathBuf,
    source: morrow_lib::native_bridge::messages_sqlite::MessagesSqliteAdapter,
}

impl ProviderRouteFixture {
    fn new() -> Result<Self, String> {
        let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
        let store_path = dir.path().join("morrow.sqlite");
        let messages_db_path = dir.path().join("chat.db");
        create_messages_fixture(&messages_db_path)?;
        Ok(Self {
            source: morrow_lib::native_bridge::messages_sqlite::MessagesSqliteAdapter::new(
                messages_db_path.clone(),
            ),
            _dir: dir,
            store_path,
            messages_db_path,
        })
    }
}

fn provider_route_request() -> Result<ScanSelectedChatsRequest, String> {
    scan_request(
        &[chat(
            "iMessage;-;+15555550103",
            1,
            &["messages-participant-6044b729eea9fa126e78d421e4a41ac8"],
        )],
        &[],
        true,
        1,
        0,
    )
}

fn provider_route_request_with_options(
    source_excerpts_enabled: bool,
    reference_timezone: &str,
    reference_unix_seconds: i64,
) -> Result<ScanSelectedChatsRequest, String> {
    serde_json::from_value(json!({
        "selectedChatIds": ["iMessage;-;+15555550103"],
        "selectedChats": [{
            "id": "iMessage;-;+15555550103",
            "participantCount": 1,
            "participantIds": ["messages-participant-6044b729eea9fa126e78d421e4a41ac8"],
        }],
        "referenceTimezone": reference_timezone,
        "referenceUnixSeconds": reference_unix_seconds,
        "backfillPromptChatIds": [],
        "sourceExcerptsEnabled": source_excerpts_enabled,
        "feedbackTextSnapshotsEnabled": false,
        "localDiagnosticsEnabled": false,
        "localDiagnosticsRetentionDays": 30,
        "capPolicy": {
            "mode": "refillForPending",
            "maxVisible": 1,
            "pendingCount": 0,
        },
    }))
    .map_err(|error| error.to_string())
}

fn scan_provider_route<P, R>(
    fixture: &ProviderRouteFixture,
    request: ScanSelectedChatsRequest,
    provider: &P,
    adapter: &RecordingProposalAdapter,
    recorder: &R,
) -> Result<morrow_lib::native_bridge::ScanSelectedChatsResult, String>
where
    P: morrow_detection::AiProvider,
    R: morrow_diagnostics::TraceRecorder + ?Sized,
{
    scan_selected_chats_with_dependencies(
        request,
        &fixture.store_path,
        ScanSelectedChatsDependencies {
            source: &fixture.source,
            provider,
            proposal_adapter: adapter,
            trace_recorder: recorder,
        },
    )
    .map_err(|error| error.to_string())
}
