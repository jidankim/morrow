use morrow_storage::ExternalSource;

use crate::native_bridge::eventkit_proposal::{ReminderDueComponents, ReminderDueTimeZone};

use super::super::fixtures::{reminder_payload, CANDIDATE_ID};
use super::super::reminder::{proposed_reminder_from_payload, reminder_mapping_from_payload};
use super::*;

#[test]
fn reminder_payload_converts_to_proposal_record_with_due_components() {
    // Given
    let payload = reminder_payload("2026-07-25T09:00:30[Asia/Seoul]");

    // When
    let reminder = proposed_reminder_from_payload(&payload).expect("reminder payload converts");

    // Then
    assert_eq!(reminder.title, "Messages reminder candidate");
    assert_eq!(reminder.candidate_id.as_str(), CANDIDATE_ID);
    assert_eq!(
        reminder.selected_source_id.as_str(),
        "morrow-selected-reminders"
    );
    assert_eq!(
        reminder.due,
        ReminderDueComponents {
            year: 2026,
            month: 7,
            day: 25,
            hour: 9,
            minute: 0,
            second: 30,
            time_zone: ReminderDueTimeZone::Named("Asia/Seoul".to_owned()),
        }
    );
}

#[test]
fn reminder_payload_mapping_uses_adapter_receipt() {
    // Given
    let payload = reminder_payload("2026-07-25T00:00:00Z");
    let adapter = FakeProposalAdapter::default();

    // When
    let mapping =
        reminder_mapping_from_payload(&payload, &adapter).expect("reminder mapping created");

    // Then
    assert_eq!(mapping.candidate_id.as_str(), CANDIDATE_ID);
    assert_eq!(mapping.source, ExternalSource::Reminders);
    assert_eq!(
        mapping.external_object_id,
        "fake-reminder-morrow_0000000000000001"
    );
    assert_eq!(mapping.external_source_id, "fake-list-1");
    assert_eq!(mapping.mapped_at, 1_782_352_400);
}
