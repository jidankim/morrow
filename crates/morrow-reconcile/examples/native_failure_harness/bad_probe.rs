use std::error::Error;

use crate::calendar_reminders::NativeSummary;
use crate::harness_error::HarnessError;
use crate::messages::MessagesSummary;
use crate::partial_replay::ReplaySummary;
use crate::provider::ProviderSummary;
use crate::validation::{validate_harness, HarnessSummaries};

pub(crate) fn run() -> Result<(), Box<dyn Error>> {
    let messages = MessagesSummary {
        permission_status: "available",
        permission_warning: "wrong_warning".to_owned(),
        unavailable_warning: "chat_unavailable:chat-alpha".to_owned(),
        unavailable_messages: 1,
        guid_change_warning: "chat_unavailable:chat-alpha".to_owned(),
        guid_change_messages: 0,
        group_pause_reason: "participant_identity_changed".to_owned(),
        group_messages: 0,
    };
    let native = NativeSummary {
        permission_failure: "permission_denied",
        calendar_failure: "calendar_creation_failed",
        calendar_failure_events: 0,
        deleted_list_failure: "proposed_list_missing",
        recovered_calendar_count: 1,
        recovered_event_count: 1,
        proposed_list_count: 1,
    };
    let replay = ReplaySummary {
        recovered_reason_count: 1,
        replay_create_actions: 0,
        replay_candidates: 0,
    };
    let provider = ProviderSummary {
        quiet_reason: "provider_unavailable".to_owned(),
        candidates: 0,
    };

    if validate_harness(HarnessSummaries {
        messages: &messages,
        native: &native,
        replay: &replay,
        provider: &provider,
    })
    .is_ok()
    {
        return Err(HarnessError::UnexpectedOutcome {
            scenario: "adversarial_bad_probe",
            expected: "validator rejection",
            actual: "accepted bad summary".to_owned(),
        }
        .into());
    }

    Err(HarnessError::UnexpectedOutcome {
        scenario: "adversarial_bad_probe",
        expected: "valid native failure summary",
        actual: "deliberately bad summary rejected".to_owned(),
    }
    .into())
}
