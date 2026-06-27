use morrow_reminders::{
    FakeReminders, ListId, Operation, ReminderAdapter, ReminderDate, ReminderDraft,
    ReminderObservation, ReminderTime, RemindersError, SourceId, MORROW_PROPOSED_LIST_NAME,
};

fn draft(title: &str, due_time: Option<ReminderTime>) -> ReminderDraft {
    ReminderDraft::new(
        title,
        ReminderDate::parse("2026-07-17").expect("valid date"),
        due_time,
    )
    .expect("valid draft")
}

#[test]
fn creates_morrow_proposed_list_when_missing() {
    // Given: a selected Reminders source with no Morrow-owned list.
    let source = SourceId::parse("source-primary").expect("valid source");
    let mut fake = FakeReminders::allowed();
    let adapter = ReminderAdapter::new(source.clone());

    // When: a date-bearing task proposal is created.
    let created = adapter
        .create_proposal(&mut fake, draft("Send the deck", None))
        .expect("proposal created");

    // Then: the adapter creates and uses the dedicated proposal list in that source.
    let proposed = fake
        .list_named(&source, MORROW_PROPOSED_LIST_NAME)
        .expect("proposed list");
    assert_eq!(created.list_id, proposed.id);
    assert_eq!(created.list_name, MORROW_PROPOSED_LIST_NAME);
}

#[test]
fn reuses_existing_morrow_proposed_list_when_present() {
    // Given: the selected source already has a Morrow Proposed list.
    let source = SourceId::parse("source-primary").expect("valid source");
    let mut fake = FakeReminders::allowed();
    let existing = fake
        .create_list(source.clone(), MORROW_PROPOSED_LIST_NAME)
        .expect("existing list");
    let adapter = ReminderAdapter::new(source.clone());

    // When: a proposal is created.
    let created = adapter
        .create_proposal(&mut fake, draft("Send the deck", None))
        .expect("proposal created");

    // Then: stale state is reused instead of creating a duplicate list.
    assert_eq!(created.list_id, existing.id);
    assert_eq!(fake.list_count_named(&source, MORROW_PROPOSED_LIST_NAME), 1);
}

#[test]
fn date_only_reminders_preserve_nullable_due_time() {
    // Given: a date-bearing task with no explicit time.
    let source = SourceId::parse("source-primary").expect("valid source");
    let mut fake = FakeReminders::allowed();
    let adapter = ReminderAdapter::new(source);

    // When: the proposal is written.
    let created = adapter
        .create_proposal(&mut fake, draft("Send the deck", None))
        .expect("proposal created");

    // Then: the stored reminder has no invented due time.
    let stored = fake
        .reminder(&created.reminder_id)
        .expect("stored reminder");
    assert_eq!(stored.due_date.as_str(), "2026-07-17");
    assert_eq!(stored.due_time, None);
}

#[test]
fn explicit_due_times_are_retained() {
    // Given: a task candidate with an explicit due time.
    let source = SourceId::parse("source-primary").expect("valid source");
    let mut fake = FakeReminders::allowed();
    let adapter = ReminderAdapter::new(source);
    let due_time = ReminderTime::parse("09:30").expect("valid time");

    // When: the proposal is written.
    let created = adapter
        .create_proposal(&mut fake, draft("Send the deck", Some(due_time)))
        .expect("proposal created");

    // Then: the exact due time is retained.
    let stored = fake
        .reminder(&created.reminder_id)
        .expect("stored reminder");
    assert_eq!(stored.due_time, Some(due_time));
}

#[test]
fn completion_inside_proposed_list_is_rejection_resolved_not_approval() {
    // Given: a visible proposed reminder.
    let source = SourceId::parse("source-primary").expect("valid source");
    let mut fake = FakeReminders::allowed();
    let adapter = ReminderAdapter::new(source);
    let created = adapter
        .create_proposal(&mut fake, draft("Send the deck", None))
        .expect("proposal created");

    // When: the user completes it while it remains in Morrow Proposed.
    fake.complete_reminder(&created.reminder_id)
        .expect("complete proposed");
    let observed = adapter
        .observe(&fake, &created.reminder_id)
        .expect("observe completion");

    // Then: completion is treated as rejection/resolution, never approval.
    assert_eq!(observed, ReminderObservation::RejectedResolved);
}

#[test]
fn moving_proposed_reminder_to_real_list_is_approval() {
    // Given: a visible proposed reminder and a user-owned real list.
    let source = SourceId::parse("source-primary").expect("valid source");
    let mut fake = FakeReminders::allowed();
    let real_list = fake
        .create_list(source.clone(), "Personal")
        .expect("real list");
    let adapter = ReminderAdapter::new(source);
    let created = adapter
        .create_proposal(&mut fake, draft("Send the deck", None))
        .expect("proposal created");

    // When: the user moves the reminder out of Morrow Proposed.
    fake.move_reminder(&created.reminder_id, &real_list.id)
        .expect("move to real list");
    let observed = adapter
        .observe(&fake, &created.reminder_id)
        .expect("observe move");

    // Then: the move is approval with the destination list recorded.
    assert_eq!(
        observed,
        ReminderObservation::Approved {
            list_id: real_list.id
        }
    );
}

#[test]
fn deleted_proposed_list_is_typed_failure_not_approval() {
    // Given: a proposed reminder remains, but the Morrow Proposed list was deleted.
    let source = SourceId::parse("source-primary").expect("valid source");
    let mut fake = FakeReminders::allowed();
    let adapter = ReminderAdapter::new(source.clone());
    let created = adapter
        .create_proposal(&mut fake, draft("Send the deck", None))
        .expect("proposal created");
    fake.delete_list(&created.list_id)
        .expect("delete proposed list");

    // When: reconciliation observes the reminder.
    let err = adapter
        .observe(&fake, &created.reminder_id)
        .expect_err("proposed list missing");

    // Then: the adapter reports a typed missing-list failure instead of approval.
    assert!(matches!(
        err,
        RemindersError::ProposedListMissing { source_id } if source_id == source.to_string()
    ));
    assert_eq!(fake.list_count_named(&source, MORROW_PROPOSED_LIST_NAME), 0);
}

#[test]
fn selected_source_is_used_even_when_other_sources_have_proposed_lists() {
    // Given: a different source already has a Morrow Proposed list.
    let other = SourceId::parse("source-other").expect("valid source");
    let selected = SourceId::parse("source-selected").expect("valid source");
    let mut fake = FakeReminders::allowed();
    fake.create_list(other, MORROW_PROPOSED_LIST_NAME)
        .expect("other proposed list");
    let adapter = ReminderAdapter::new(selected.clone());

    // When: a proposal is created for the selected source.
    let created = adapter
        .create_proposal(&mut fake, draft("Send the deck", None))
        .expect("proposal created");

    // Then: the adapter creates and uses Morrow Proposed in the selected source.
    let selected_list = fake
        .list_named(&selected, MORROW_PROPOSED_LIST_NAME)
        .expect("selected proposed list");
    assert_eq!(created.list_id, selected_list.id);
}

#[test]
fn permission_denied_returns_typed_error() {
    // Given: the Reminders surface denies access.
    let source = SourceId::parse("source-primary").expect("valid source");
    let mut fake = FakeReminders::permission_denied();
    let adapter = ReminderAdapter::new(source);

    // When: the adapter tries to create a proposal.
    let err = adapter
        .create_proposal(&mut fake, draft("Send the deck", None))
        .expect_err("permission denied");

    // Then: callers receive a typed permission failure with the attempted operation.
    assert!(matches!(
        err,
        RemindersError::PermissionDenied {
            operation: Operation::EnsureProposedList
        }
    ));
}

#[test]
fn malformed_inputs_are_rejected_at_boundary() {
    // Given: invalid boundary strings.
    let bad_date = ReminderDate::parse("2026-02-30").expect_err("bad date");
    let bad_time = ReminderTime::parse("24:00").expect_err("bad time");
    let bad_source = SourceId::parse("   ").expect_err("bad source");
    let bad_list = ListId::parse("").expect_err("bad list");

    // When/Then: each parse failure is typed as invalid input.
    assert!(matches!(bad_date, RemindersError::InvalidInput { .. }));
    assert!(matches!(bad_time, RemindersError::InvalidInput { .. }));
    assert!(matches!(bad_source, RemindersError::InvalidInput { .. }));
    assert!(matches!(bad_list, RemindersError::InvalidInput { .. }));
}

#[test]
fn approved_reminders_are_not_mutated_by_pending_edits() {
    // Given: a proposed reminder that the user approved by moving to a real list.
    let source = SourceId::parse("source-primary").expect("valid source");
    let mut fake = FakeReminders::allowed();
    let real_list = fake
        .create_list(source.clone(), "Personal")
        .expect("real list");
    let adapter = ReminderAdapter::new(source);
    let created = adapter
        .create_proposal(&mut fake, draft("Send the deck", None))
        .expect("proposal created");
    fake.move_reminder(&created.reminder_id, &real_list.id)
        .expect("move to real list");

    // When: a later model output tries to edit the approved reminder automatically.
    let err = adapter
        .apply_pending_title(&mut fake, &created.reminder_id, "Send the updated deck")
        .expect_err("approved mutation rejected");

    // Then: the mutation is refused and the user-owned reminder is unchanged.
    let stored = fake
        .reminder(&created.reminder_id)
        .expect("stored reminder");
    assert!(matches!(
        err,
        RemindersError::ApprovedMutationRejected { .. }
    ));
    assert_eq!(stored.title, "Send the deck");
}
