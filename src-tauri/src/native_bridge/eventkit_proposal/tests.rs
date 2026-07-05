use morrow_calendar::{
    CalendarSourceId, CandidateId, EventRecord, ProposalMetadata, ProposedEvent, TimeRange,
};

use super::{
    EventKitProposalAdapter, EventKitProposalClient, EventKitProposalError,
    EventKitProposalReceipt, EventKitReminderReceipt, ProposedReminder, ReminderDate, ReminderDue,
    ReminderProposalMetadata, ReminderTime,
};

#[test]
fn creates_free_proposal_event_when_eventkit_save_succeeds() -> Result<(), String> {
    // Given
    let client = FakeEventKitClient::succeeding();
    let mut adapter = EventKitProposalAdapter::new(client);

    // When
    let receipt = adapter
        .propose_event(proposed_event()?)
        .map_err(|error| error.to_string())?;

    // Then
    assert_eq!(receipt.event_id, "event-1");
    assert_eq!(receipt.calendar_id, "calendar-morrow-proposed");
    assert_eq!(receipt.source_id, "source-local");
    let record = adapter
        .client
        .created
        .as_ref()
        .ok_or_else(|| "missing created EventKit record".to_owned())?;
    assert_eq!(record.title, "Messages event candidate");
    assert_eq!(record.calendar_name, "Morrow Proposed");
    assert_eq!(record.availability, morrow_calendar::Availability::Free);
    assert!(record.alerts.is_empty());
    assert!(record.attendees.is_empty());
    assert!(!record.invite_sent);
    assert!(record.notes.contains("[MORROW_METADATA_V1]"));
    assert!(record
        .notes
        .contains("candidate_id=63616e6469646174652d76697369626c652d31"));
    assert!(record
        .notes
        .contains("source_id=6d6f72726f772d73656c65637465642d6d65737361676573"));
    Ok(())
}

#[test]
fn reports_permission_denied_when_eventkit_access_is_denied() -> Result<(), String> {
    // Given
    let mut adapter = EventKitProposalAdapter::new(FakeEventKitClient::failing(
        EventKitProposalError::PermissionDenied {
            reason: "Calendar access was denied".to_owned(),
        },
    ));

    // When
    let error = adapter
        .propose_event(proposed_event()?)
        .err()
        .ok_or_else(|| "proposal unexpectedly succeeded".to_owned())?;

    // Then
    assert_eq!(
        error,
        EventKitProposalError::PermissionDenied {
            reason: "Calendar access was denied".to_owned(),
        }
    );
    Ok(())
}

#[test]
fn reports_source_unavailable_when_calendar_cannot_be_written() -> Result<(), String> {
    // Given
    let mut adapter = EventKitProposalAdapter::new(FakeEventKitClient::failing(
        EventKitProposalError::SourceUnavailable {
            reason: "Morrow Proposed exists but is read-only".to_owned(),
        },
    ));

    // When
    let error = adapter
        .propose_event(proposed_event()?)
        .err()
        .ok_or_else(|| "proposal unexpectedly succeeded".to_owned())?;

    // Then
    assert_eq!(
        error,
        EventKitProposalError::SourceUnavailable {
            reason: "Morrow Proposed exists but is read-only".to_owned(),
        }
    );
    Ok(())
}

#[test]
fn reports_save_failure_when_eventkit_save_fails() -> Result<(), String> {
    // Given
    let mut adapter = EventKitProposalAdapter::new(FakeEventKitClient::failing(
        EventKitProposalError::SaveFailed {
            reason: "database is locked".to_owned(),
        },
    ));

    // When
    let error = adapter
        .propose_event(proposed_event()?)
        .err()
        .ok_or_else(|| "proposal unexpectedly succeeded".to_owned())?;

    // Then
    assert_eq!(
        error,
        EventKitProposalError::SaveFailed {
            reason: "database is locked".to_owned(),
        }
    );
    Ok(())
}

#[test]
fn rejects_empty_event_identifier_when_eventkit_reports_success() -> Result<(), String> {
    // Given
    let mut adapter = EventKitProposalAdapter::new(FakeEventKitClient {
        receipt: Ok(EventKitProposalReceipt {
            event_id: String::new(),
            calendar_id: "calendar-morrow-proposed".to_owned(),
            source_id: "source-local".to_owned(),
        }),
        reminder_receipt: Ok(EventKitReminderReceipt {
            reminder_id: "reminder-1".to_owned(),
            list_id: "list-morrow-proposed".to_owned(),
            source_id: "source-local".to_owned(),
        }),
        created: None,
        created_reminder: None,
    });

    // When
    let error = adapter
        .propose_event(proposed_event()?)
        .err()
        .ok_or_else(|| "proposal unexpectedly succeeded".to_owned())?;

    // Then
    assert_eq!(error, EventKitProposalError::EmptyEventIdentifier);
    Ok(())
}

#[test]
fn creates_proposal_reminder_when_eventkit_save_succeeds() -> Result<(), String> {
    // Given
    let client = FakeEventKitClient::succeeding();
    let mut adapter = EventKitProposalAdapter::new(client);

    // When
    let receipt = adapter
        .propose_reminder(proposed_reminder())
        .map_err(|error| error.to_string())?;

    // Then
    assert_eq!(receipt.reminder_id, "reminder-1");
    assert_eq!(receipt.list_id, "list-morrow-proposed");
    assert_eq!(receipt.source_id, "source-local");
    let record = adapter
        .client
        .created_reminder
        .as_ref()
        .ok_or_else(|| "missing created EventKit reminder".to_owned())?;
    assert_eq!(record.title, "Finish review of the essay");
    assert_eq!(record.due.date.year, 2026);
    assert_eq!(record.due.date.month, 7);
    assert_eq!(record.due.date.day, 25);
    assert_eq!(
        record.due.time,
        Some(ReminderTime {
            hour: 9,
            minute: 30,
        })
    );
    assert!(record.notes.contains("[MORROW_METADATA_V1]"));
    assert!(record
        .notes
        .contains("candidate_id=morrow_0000000000000001"));
    assert!(record.notes.contains("source_id=morrow-selected-messages"));
    Ok(())
}

#[test]
#[cfg(target_os = "macos")]
fn rejects_truncated_eventkit_identifiers() {
    // Given
    let raw = super::macos::RawEventKitProposalResult::success_for_test(
        "event-1",
        "calendar-morrow-proposed",
        "source-local",
        true,
    );

    // When
    let error = raw.into_result().expect_err("truncated identifier rejects");

    // Then
    assert_eq!(
        error,
        EventKitProposalError::SaveFailed {
            reason: "EventKit returned a truncated event identifier".to_owned(),
        }
    );
}

#[cfg(not(target_os = "macos"))]
#[test]
fn bridge_is_typed_unavailable_on_non_macos() -> Result<(), String> {
    // Given
    let bridge = super::EventKitProposalBridge;

    // When
    let error = bridge
        .propose_event(proposed_event()?)
        .err()
        .ok_or_else(|| "proposal unexpectedly succeeded".to_owned())?;

    // Then
    assert_eq!(
        error,
        EventKitProposalError::Unavailable {
            reason: "Calendar proposal creation requires macOS EventKit".to_owned(),
        }
    );
    Ok(())
}

#[derive(Debug)]
struct FakeEventKitClient {
    receipt: Result<EventKitProposalReceipt, EventKitProposalError>,
    reminder_receipt: Result<EventKitReminderReceipt, EventKitProposalError>,
    created: Option<EventRecord>,
    created_reminder: Option<super::ReminderRecord>,
}

impl FakeEventKitClient {
    fn succeeding() -> Self {
        Self {
            receipt: Ok(EventKitProposalReceipt {
                event_id: "event-1".to_owned(),
                calendar_id: "calendar-morrow-proposed".to_owned(),
                source_id: "source-local".to_owned(),
            }),
            reminder_receipt: Ok(EventKitReminderReceipt {
                reminder_id: "reminder-1".to_owned(),
                list_id: "list-morrow-proposed".to_owned(),
                source_id: "source-local".to_owned(),
            }),
            created: None,
            created_reminder: None,
        }
    }

    fn failing(error: EventKitProposalError) -> Self {
        Self {
            receipt: Err(error),
            reminder_receipt: Err(EventKitProposalError::Unavailable {
                reason: "unused reminder fake".to_owned(),
            }),
            created: None,
            created_reminder: None,
        }
    }
}

impl EventKitProposalClient for FakeEventKitClient {
    fn create_proposal_event(
        &mut self,
        record: &EventRecord,
    ) -> Result<EventKitProposalReceipt, EventKitProposalError> {
        self.created = Some(record.clone());
        self.receipt.clone()
    }

    fn create_proposal_reminder(
        &mut self,
        record: &super::ReminderRecord,
    ) -> Result<EventKitReminderReceipt, EventKitProposalError> {
        self.created_reminder = Some(record.clone());
        self.reminder_receipt.clone()
    }
}

fn proposed_event() -> Result<ProposedEvent, String> {
    Ok(ProposedEvent {
        title: "Messages event candidate".to_owned(),
        time_range: TimeRange::new(1_782_352_400, 1_782_354_200)
            .map_err(|error| error.to_string())?,
        user_note: String::new(),
        video_url: None,
        metadata: ProposalMetadata {
            candidate_id: CandidateId::new("candidate-visible-1")
                .map_err(|error| error.to_string())?,
            source_id: CalendarSourceId::new("morrow-selected-messages")
                .map_err(|error| error.to_string())?,
        },
    })
}

fn proposed_reminder() -> ProposedReminder {
    ProposedReminder {
        title: "Finish review of the essay".to_owned(),
        due: ReminderDue {
            date: ReminderDate {
                year: 2026,
                month: 7,
                day: 25,
            },
            time: Some(ReminderTime {
                hour: 9,
                minute: 30,
            }),
            timezone_name: Some("Asia/Seoul".to_owned()),
        },
        metadata: ReminderProposalMetadata {
            candidate_id: "morrow_0000000000000001".to_owned(),
            source_id: "morrow-selected-messages".to_owned(),
        },
    }
}
