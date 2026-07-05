use super::{
    reminder_test_support::{reminder_record, FakeEventKitClient},
    EventKitProposalAdapter, EventKitReminderProposalError, EventKitReminderProposalReceipt,
    ReminderDueComponents, ReminderDueTimeZone,
};

#[test]
fn creates_reminder_proposal_when_eventkit_save_succeeds() {
    // Given
    let client = FakeEventKitClient::succeeding();
    let mut adapter = EventKitProposalAdapter::new(client);

    // When
    let receipt = adapter
        .propose_reminder(reminder_record())
        .expect("reminder proposal succeeds");

    // Then
    assert_eq!(receipt.reminder_id, "reminder-1");
    assert_eq!(receipt.list_id, "list-morrow-proposed");
    assert_eq!(receipt.source_id, "source-reminders-local");
    let record = adapter
        .client
        .created_reminder
        .as_ref()
        .expect("created EventKit reminder record");
    assert_eq!(record.title, "Messages reminder candidate");
    assert_eq!(
        record.selected_source_id.as_str(),
        "morrow-selected-reminders"
    );
    assert_eq!(
        record.due,
        ReminderDueComponents {
            year: 2026,
            month: 7,
            day: 15,
            hour: 14,
            minute: 30,
            second: 0,
            time_zone: ReminderDueTimeZone::Named("Asia/Seoul".to_owned()),
        }
    );
    assert!(record.notes.contains("[MORROW_METADATA_V1]"));
    assert!(record
        .notes
        .contains("candidate_id=63616e6469646174652d72656d696e6465722d31"));
    assert!(record
        .notes
        .contains("source_id=6d6f72726f772d73656c65637465642d72656d696e64657273"));
}

#[test]
fn reports_permission_denied_when_reminders_access_is_denied() {
    // Given
    let mut adapter = EventKitProposalAdapter::new(FakeEventKitClient::failing_reminder(
        EventKitReminderProposalError::PermissionDenied {
            reason: "Reminders access was denied".to_owned(),
        },
    ));

    // When
    let error = adapter
        .propose_reminder(reminder_record())
        .expect_err("proposal rejects");

    // Then
    assert_eq!(
        error,
        EventKitReminderProposalError::PermissionDenied {
            reason: "Reminders access was denied".to_owned(),
        }
    );
}

#[test]
fn reports_source_unavailable_when_reminders_source_cannot_be_written() {
    // Given
    let mut adapter = EventKitProposalAdapter::new(FakeEventKitClient::failing_reminder(
        EventKitReminderProposalError::SourceUnavailable {
            reason: "Morrow Proposed list is read-only".to_owned(),
        },
    ));

    // When
    let error = adapter
        .propose_reminder(reminder_record())
        .expect_err("proposal rejects");

    // Then
    assert_eq!(
        error,
        EventKitReminderProposalError::SourceUnavailable {
            reason: "Morrow Proposed list is read-only".to_owned(),
        }
    );
}

#[test]
fn reports_save_failure_when_reminder_save_fails() -> Result<(), String> {
    // Given
    let mut adapter = EventKitProposalAdapter::new(FakeEventKitClient::failing_reminder(
        EventKitReminderProposalError::SaveFailed {
            reason: "database is locked".to_owned(),
        },
    ));

    // When
    let error = adapter
        .propose_reminder(reminder_record())
        .err()
        .ok_or_else(|| "proposal unexpectedly succeeded".to_owned())?;

    // Then
    assert_eq!(
        error,
        EventKitReminderProposalError::SaveFailed {
            reason: "database is locked".to_owned(),
        }
    );
    assert_eq!(
        error.to_string(),
        "Reminders reminder save failed: database is locked"
    );
    Ok(())
}

#[test]
fn rejects_empty_reminder_identifier_when_eventkit_reports_success() -> Result<(), String> {
    // Given
    let mut adapter = EventKitProposalAdapter::new(FakeEventKitClient {
        reminder_receipt: Ok(EventKitReminderProposalReceipt {
            reminder_id: String::new(),
            list_id: "list-morrow-proposed".to_owned(),
            source_id: "source-reminders-local".to_owned(),
        }),
        created_reminder: None,
    });

    // When
    let error = adapter
        .propose_reminder(reminder_record())
        .err()
        .ok_or_else(|| "proposal unexpectedly succeeded".to_owned())?;

    // Then
    assert_eq!(
        error,
        EventKitReminderProposalError::EmptyIdentifier {
            field: "reminder_id"
        }
    );
    Ok(())
}

#[test]
fn rejects_truncated_reminder_identifier() -> Result<(), String> {
    // Given
    let mut adapter = EventKitProposalAdapter::new(FakeEventKitClient {
        reminder_receipt: Ok(EventKitReminderProposalReceipt {
            reminder_id: "r".repeat(256),
            list_id: "list-morrow-proposed".to_owned(),
            source_id: "source-reminders-local".to_owned(),
        }),
        created_reminder: None,
    });

    // When
    let error = adapter
        .propose_reminder(reminder_record())
        .err()
        .ok_or_else(|| "proposal unexpectedly succeeded".to_owned())?;

    // Then
    assert_eq!(
        error,
        EventKitReminderProposalError::TruncatedIdentifier {
            field: "reminder_id"
        }
    );
    Ok(())
}

#[test]
fn rejects_nul_title_before_eventkit_call() -> Result<(), String> {
    // Given
    let mut adapter = EventKitProposalAdapter::new(FakeEventKitClient::succeeding());
    let mut record = reminder_record();
    record.title = "bad\0title".to_owned();

    // When
    let error = adapter
        .propose_reminder(record)
        .err()
        .ok_or_else(|| "proposal unexpectedly succeeded".to_owned())?;

    // Then
    assert_eq!(
        error,
        EventKitReminderProposalError::InvalidInput {
            field: "title",
            reason: "must not contain NUL bytes".to_owned(),
        }
    );
    assert!(adapter.client.created_reminder.is_none());
    Ok(())
}

#[test]
fn rejects_nul_notes_before_eventkit_call() -> Result<(), String> {
    // Given
    let mut adapter = EventKitProposalAdapter::new(FakeEventKitClient::succeeding());
    let mut record = reminder_record();
    record.notes = "bad\0notes".to_owned();

    // When
    let error = adapter
        .propose_reminder(record)
        .err()
        .ok_or_else(|| "proposal unexpectedly succeeded".to_owned())?;

    // Then
    assert_eq!(
        error,
        EventKitReminderProposalError::InvalidInput {
            field: "notes",
            reason: "must not contain NUL bytes".to_owned(),
        }
    );
    assert!(adapter.client.created_reminder.is_none());
    Ok(())
}

#[cfg(not(target_os = "macos"))]
#[test]
fn bridge_is_typed_reminders_unavailable_on_non_macos() -> Result<(), String> {
    // Given
    let bridge = super::EventKitProposalBridge;

    // When
    let error = bridge
        .propose_reminder(reminder_record())
        .err()
        .ok_or_else(|| "proposal unexpectedly succeeded".to_owned())?;

    // Then
    assert_eq!(
        error,
        EventKitReminderProposalError::Unavailable {
            reason: "Reminders proposal creation requires macOS EventKit".to_owned(),
        }
    );
    Ok(())
}
