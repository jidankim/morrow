use morrow_lib::native_bridge::{
    CodexAuthStatus, CodexProviderAuthReadiness, MORROW_PROVIDER_TOKEN_KIND,
};

#[path = "native_scan_codex/support.rs"]
mod support;

use support::{
    assert_counts, auth_readiness, candidate_json, candidate_json_at, scan_request_at,
    FakeCodexOutcome, RecordingCodexRunner, RejectingProposalAdapter, ScanFixture,
};

#[test]
fn production_scan_uses_codex_provider_when_auth_ready() -> Result<(), String> {
    // Given
    let fixture = ScanFixture::new("codex-ready")?;
    let runner = RecordingCodexRunner::new(vec![FakeCodexOutcome::WriteOutput(
        candidate_json().to_owned(),
    )]);
    let adapter = RejectingProposalAdapter;

    // When
    let result = fixture.scan_with(
        auth_readiness(CodexAuthStatus::LoggedInUsingChatGpt, true),
        &runner,
        &adapter,
    )?;

    // Then
    assert_counts(&result, (1, 1, 0, 0, 1));
    assert_eq!(runner.run_count(), 1);
    assert_eq!(result.created_external_proposal_count, 0);
    Ok(())
}

#[test]
fn production_scan_reports_provider_missing_when_codex_auth_missing() -> Result<(), String> {
    for status in [CodexAuthStatus::MissingCli, CodexAuthStatus::NotLoggedIn] {
        // Given
        let fixture = ScanFixture::new("codex-missing")?;
        let runner = RecordingCodexRunner::new(vec![FakeCodexOutcome::WriteOutput(
            candidate_json().to_owned(),
        )]);
        let adapter = RejectingProposalAdapter;

        // When
        let result = fixture.scan_with(auth_readiness(status, false), &runner, &adapter)?;

        // Then
        assert_counts(&result, (0, 0, 1, 0, 0));
        assert_eq!(runner.run_count(), 0);
    }
    Ok(())
}

#[test]
fn production_scan_does_not_reinterpret_legacy_api_key_as_codex() -> Result<(), String> {
    // Given
    let fixture = ScanFixture::new("legacy-token")?;
    let runner = RecordingCodexRunner::new(vec![FakeCodexOutcome::WriteOutput(
        candidate_json().to_owned(),
    )]);
    let adapter = RejectingProposalAdapter;
    let legacy_key_diagnostic = format!("{MORROW_PROVIDER_TOKEN_KIND} fixture is redacted");

    // When
    let result = fixture.scan_with(
        CodexProviderAuthReadiness {
            status: CodexAuthStatus::UnknownFailure,
            ready: false,
            command_surface: "codex login status".to_owned(),
            command_output_redacted: true,
            diagnostic: legacy_key_diagnostic,
        },
        &runner,
        &adapter,
    )?;

    // Then
    assert_counts(&result, (0, 0, 1, 0, 0));
    assert_eq!(runner.run_count(), 0);
    Ok(())
}

#[test]
fn production_scan_uses_runtime_reference_for_recent_messages() -> Result<(), String> {
    // Given
    let fixture = ScanFixture::new_at("runtime-reference", 1_783_000_190)?;
    let runner = RecordingCodexRunner::new(vec![FakeCodexOutcome::WriteOutput(
        candidate_json_at("2026-07-03T15:00:00[Asia/Seoul]").to_owned(),
    )]);
    let adapter = RejectingProposalAdapter;

    // When
    let result = fixture.scan_with_request(
        scan_request_at(1_783_000_200)?,
        auth_readiness(CodexAuthStatus::LoggedInUsingChatGpt, true),
        &runner,
        &adapter,
    )?;

    // Then
    assert_counts(&result, (1, 1, 0, 0, 1));
    assert_eq!(runner.run_count(), 1);
    Ok(())
}
