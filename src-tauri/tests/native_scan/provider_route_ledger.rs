use morrow_lib::native_bridge::{
    scan_selected_chats_with_dependencies, ScanSelectedChatsDependencies,
};
use morrow_storage::Store;

use super::dependencies::{
    CandidateProvider, CountingProvider, InvalidJsonProvider, RecordingProposalAdapter,
};
use super::message_sqlite::{
    create_messages_fixture, drop_candidate_failure_trigger, drop_external_mapping_failure_trigger,
    drop_quiet_log_failure_trigger, install_candidate_failure_trigger,
    install_external_mapping_failure_trigger, install_quiet_log_failure_trigger,
    provider_route_outcome_count,
};
use super::support::{assert_counts, chat, scan_request};

#[test]
fn provider_route_ledger_skips_provider_on_second_scan() -> Result<(), String> {
    // Given
    let fixture = ProviderRouteFixture::new()?;
    let request = provider_route_request()?;
    let provider = CountingProvider::new(CandidateProvider);
    let adapter = RecordingProposalAdapter::default();
    let recorder = morrow_diagnostics::NoopTraceRecorder;

    // When
    let first = scan_selected_chats_with_dependencies(
        request.clone(),
        &fixture.store_path,
        ScanSelectedChatsDependencies {
            source: &fixture.source,
            provider: &provider,
            proposal_adapter: &adapter,
            trace_recorder: &recorder,
        },
    )
    .map_err(|error| error.to_string())?;
    let first_provider_calls = provider.calls();
    let second = scan_selected_chats_with_dependencies(
        request,
        &fixture.store_path,
        ScanSelectedChatsDependencies {
            source: &fixture.source,
            provider: &provider,
            proposal_adapter: &adapter,
            trace_recorder: &recorder,
        },
    )
    .map_err(|error| error.to_string())?;
    let second_provider_calls = provider.calls();
    println!(
        "first_provider_calls={first_provider_calls} second_provider_calls={second_provider_calls}"
    );

    // Then
    assert_eq!(first_provider_calls, 1);
    assert_eq!(second_provider_calls, 1);
    assert_counts(&first, (1, 1, 0, 1, 0));
    assert_counts(&second, (0, 0, 0, 0, 0));
    assert_eq!(second.created_external_proposal_count, 0);
    assert_eq!(second.failed_external_proposal_count, 0);
    assert_eq!(provider_route_outcome_count(&fixture.store_path)?, 1);
    Ok(())
}

#[test]
fn provider_route_ledger_records_after_local_side_effects_only() -> Result<(), String> {
    // Given / When / Then: candidate persistence failure leaves no cache row.
    let candidate_failure = ProviderRouteFixture::new()?;
    Store::open(&candidate_failure.store_path).map_err(|error| error.to_string())?;
    install_candidate_failure_trigger(&candidate_failure.store_path)?;
    let candidate_provider = CountingProvider::new(CandidateProvider);
    let adapter = RecordingProposalAdapter::default();
    let recorder = morrow_diagnostics::NoopTraceRecorder;
    let request = provider_route_request()?;
    let failed_candidate = scan_selected_chats_with_dependencies(
        request.clone(),
        &candidate_failure.store_path,
        ScanSelectedChatsDependencies {
            source: &candidate_failure.source,
            provider: &candidate_provider,
            proposal_adapter: &adapter,
            trace_recorder: &recorder,
        },
    );
    assert!(matches!(
        failed_candidate,
        Err(morrow_lib::native_bridge::ScanSelectedChatsError::Storage(
            _
        ))
    ));
    assert_eq!(candidate_provider.calls(), 1);
    assert_eq!(
        provider_route_outcome_count(&candidate_failure.store_path)?,
        0
    );
    drop_candidate_failure_trigger(&candidate_failure.store_path)?;
    scan_selected_chats_with_dependencies(
        request,
        &candidate_failure.store_path,
        ScanSelectedChatsDependencies {
            source: &candidate_failure.source,
            provider: &candidate_provider,
            proposal_adapter: &adapter,
            trace_recorder: &recorder,
        },
    )
    .map_err(|error| error.to_string())?;
    assert_eq!(candidate_provider.calls(), 2);

    // Given / When / Then: quiet-log persistence failure leaves no cache row.
    let quiet_failure = ProviderRouteFixture::new()?;
    Store::open(&quiet_failure.store_path).map_err(|error| error.to_string())?;
    install_quiet_log_failure_trigger(&quiet_failure.store_path)?;
    let quiet_provider = CountingProvider::new(InvalidJsonProvider);
    let quiet_adapter = RecordingProposalAdapter::default();
    let quiet_request = provider_route_request()?;
    let failed_quiet = scan_selected_chats_with_dependencies(
        quiet_request.clone(),
        &quiet_failure.store_path,
        ScanSelectedChatsDependencies {
            source: &quiet_failure.source,
            provider: &quiet_provider,
            proposal_adapter: &quiet_adapter,
            trace_recorder: &recorder,
        },
    );
    assert!(matches!(
        failed_quiet,
        Err(morrow_lib::native_bridge::ScanSelectedChatsError::Storage(
            _
        ))
    ));
    assert_eq!(quiet_provider.calls(), 1);
    assert_eq!(provider_route_outcome_count(&quiet_failure.store_path)?, 0);
    drop_quiet_log_failure_trigger(&quiet_failure.store_path)?;
    scan_selected_chats_with_dependencies(
        quiet_request,
        &quiet_failure.store_path,
        ScanSelectedChatsDependencies {
            source: &quiet_failure.source,
            provider: &quiet_provider,
            proposal_adapter: &quiet_adapter,
            trace_recorder: &recorder,
        },
    )
    .map_err(|error| error.to_string())?;
    assert_eq!(quiet_provider.calls(), 2);

    // Given / When / Then: proposal-adapter failure leaves no cache row.
    let proposal_failure = ProviderRouteFixture::new()?;
    let proposal_provider = CountingProvider::new(CandidateProvider);
    let failing_adapter = RecordingProposalAdapter::failing_calendar();
    let proposal_request = provider_route_request()?;
    let failed_proposal = scan_selected_chats_with_dependencies(
        proposal_request.clone(),
        &proposal_failure.store_path,
        ScanSelectedChatsDependencies {
            source: &proposal_failure.source,
            provider: &proposal_provider,
            proposal_adapter: &failing_adapter,
            trace_recorder: &recorder,
        },
    )
    .map_err(|error| error.to_string())?;
    assert_eq!(failed_proposal.failed_external_proposal_count, 1);
    assert_eq!(proposal_provider.calls(), 1);
    assert_eq!(
        provider_route_outcome_count(&proposal_failure.store_path)?,
        0
    );
    let succeeding_adapter = RecordingProposalAdapter::default();
    scan_selected_chats_with_dependencies(
        proposal_request,
        &proposal_failure.store_path,
        ScanSelectedChatsDependencies {
            source: &proposal_failure.source,
            provider: &proposal_provider,
            proposal_adapter: &succeeding_adapter,
            trace_recorder: &recorder,
        },
    )
    .map_err(|error| error.to_string())?;
    assert_eq!(proposal_provider.calls(), 2);

    // Given / When / Then: external mapping storage failure leaves no cache row.
    let mapping_failure = ProviderRouteFixture::new()?;
    Store::open(&mapping_failure.store_path).map_err(|error| error.to_string())?;
    install_external_mapping_failure_trigger(&mapping_failure.store_path)?;
    let mapping_provider = CountingProvider::new(CandidateProvider);
    let mapping_adapter = RecordingProposalAdapter::default();
    let mapping_request = provider_route_request()?;
    let failed_mapping = scan_selected_chats_with_dependencies(
        mapping_request.clone(),
        &mapping_failure.store_path,
        ScanSelectedChatsDependencies {
            source: &mapping_failure.source,
            provider: &mapping_provider,
            proposal_adapter: &mapping_adapter,
            trace_recorder: &recorder,
        },
    )
    .map_err(|error| error.to_string())?;
    assert_eq!(failed_mapping.failed_external_proposal_count, 1);
    assert_eq!(mapping_provider.calls(), 1);
    assert_eq!(
        provider_route_outcome_count(&mapping_failure.store_path)?,
        0
    );
    drop_external_mapping_failure_trigger(&mapping_failure.store_path)?;
    scan_selected_chats_with_dependencies(
        mapping_request,
        &mapping_failure.store_path,
        ScanSelectedChatsDependencies {
            source: &mapping_failure.source,
            provider: &mapping_provider,
            proposal_adapter: &mapping_adapter,
            trace_recorder: &recorder,
        },
    )
    .map_err(|error| error.to_string())?;
    assert_eq!(mapping_provider.calls(), 2);
    Ok(())
}

struct ProviderRouteFixture {
    _dir: tempfile::TempDir,
    store_path: std::path::PathBuf,
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
                messages_db_path,
            ),
            _dir: dir,
            store_path,
        })
    }
}

fn provider_route_request() -> Result<morrow_lib::native_bridge::ScanSelectedChatsRequest, String> {
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
