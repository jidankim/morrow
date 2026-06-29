use crate::calendar_reminders::NativeSummary;
use crate::harness_error::HarnessError;
use crate::messages::MessagesSummary;
use crate::partial_replay::ReplaySummary;
use crate::provider::ProviderSummary;

#[derive(Clone, Copy)]
struct CountExpectation {
    scenario: &'static str,
    field: &'static str,
    expected: usize,
}

/// Summaries validated by the native failure harness.
#[derive(Clone, Copy, Debug)]
pub struct HarnessSummaries<'a> {
    /// Messages ingestion summary.
    pub messages: &'a MessagesSummary,
    /// Native adapter summary.
    pub native: &'a NativeSummary,
    /// Replay recovery summary.
    pub replay: &'a ReplaySummary,
    /// Provider summary.
    pub provider: &'a ProviderSummary,
}

/// Validates native failure harness summaries.
pub fn validate_harness(summaries: HarnessSummaries<'_>) -> Result<(), HarnessError> {
    validate_messages(summaries.messages, summaries.native)?;
    validate_native(summaries.native)?;
    validate_partial_write(summaries.native, summaries.replay)?;
    validate_provider_and_duplicates(summaries.native, summaries.replay, summaries.provider)?;
    Ok(())
}

fn validate_messages(
    messages: &MessagesSummary,
    native: &NativeSummary,
) -> Result<(), HarnessError> {
    require_text(
        "permission_denial",
        "unavailable",
        messages.permission_status,
    )?;
    require_text(
        "permission_denial",
        "messages_permission_denied",
        &messages.permission_warning,
    )?;
    require_text(
        "permission_denial",
        "permission_denied",
        native.permission_failure,
    )?;
    require_text(
        "unavailable_chats",
        "chat_unavailable:chat-alpha",
        &messages.unavailable_warning,
    )?;
    require_count(
        CountExpectation {
            scenario: "unavailable_chats",
            field: "emitted_messages",
            expected: 0,
        },
        messages.unavailable_messages,
    )?;
    require_text(
        "guid_change",
        "chat_unavailable:chat-alpha",
        &messages.guid_change_warning,
    )?;
    require_count(
        CountExpectation {
            scenario: "guid_change",
            field: "emitted_messages",
            expected: 0,
        },
        messages.guid_change_messages,
    )?;
    require_text(
        "group_membership",
        "participant_identity_changed",
        &messages.group_pause_reason,
    )?;
    require_count(
        CountExpectation {
            scenario: "group_membership",
            field: "emitted_messages",
            expected: 0,
        },
        messages.group_messages,
    )?;
    Ok(())
}

fn validate_native(native: &NativeSummary) -> Result<(), HarnessError> {
    require_text(
        "missing_calendar_list",
        "calendar_creation_failed",
        native.calendar_failure,
    )?;
    require_count(
        CountExpectation {
            scenario: "missing_calendar_list",
            field: "calendar_events",
            expected: 0,
        },
        native.calendar_failure_events,
    )?;
    require_text(
        "missing_calendar_list",
        "proposed_list_missing",
        native.deleted_list_failure,
    )?;
    Ok(())
}

fn validate_partial_write(
    native: &NativeSummary,
    replay: &ReplaySummary,
) -> Result<(), HarnessError> {
    require_count(
        CountExpectation {
            scenario: "partial_writes",
            field: "calendar_count",
            expected: 1,
        },
        native.recovered_calendar_count,
    )?;
    require_count(
        CountExpectation {
            scenario: "partial_writes",
            field: "event_count",
            expected: 1,
        },
        native.recovered_event_count,
    )?;
    require_count(
        CountExpectation {
            scenario: "partial_writes",
            field: "recovered_reason",
            expected: 1,
        },
        replay.recovered_reason_count,
    )?;
    require_count(
        CountExpectation {
            scenario: "partial_writes",
            field: "replay_create_actions",
            expected: 0,
        },
        replay.replay_create_actions,
    )?;
    Ok(())
}

fn validate_provider_and_duplicates(
    native: &NativeSummary,
    replay: &ReplaySummary,
    provider: &ProviderSummary,
) -> Result<(), HarnessError> {
    require_text(
        "provider_failures",
        "provider_unavailable",
        &provider.quiet_reason,
    )?;
    require_count(
        CountExpectation {
            scenario: "provider_failures",
            field: "candidates",
            expected: 0,
        },
        provider.candidates,
    )?;
    require_count(
        CountExpectation {
            scenario: "no_duplicates",
            field: "proposed_calendars",
            expected: 1,
        },
        native.recovered_calendar_count,
    )?;
    require_count(
        CountExpectation {
            scenario: "no_duplicates",
            field: "proposed_lists",
            expected: 1,
        },
        native.proposed_list_count,
    )?;
    require_count(
        CountExpectation {
            scenario: "no_duplicates",
            field: "replay_candidates",
            expected: 0,
        },
        replay.replay_candidates,
    )?;
    Ok(())
}

fn require_text(
    scenario: &'static str,
    expected: &'static str,
    actual: &str,
) -> Result<(), HarnessError> {
    if actual == expected {
        Ok(())
    } else {
        Err(HarnessError::UnexpectedOutcome {
            scenario,
            expected,
            actual: actual.to_owned(),
        })
    }
}

const fn require_count(expectation: CountExpectation, actual: usize) -> Result<(), HarnessError> {
    if actual == expectation.expected {
        Ok(())
    } else {
        Err(HarnessError::CountMismatch {
            scenario: expectation.scenario,
            field: expectation.field,
            expected: expectation.expected,
            actual,
        })
    }
}
