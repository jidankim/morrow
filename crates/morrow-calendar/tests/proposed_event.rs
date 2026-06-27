use morrow_calendar::{
    parse_metadata, strip_morrow_metadata, Availability, CalendarError, CalendarPlanner,
    CalendarSourceId, CandidateId, FakeEventKit, ProposalMetadata, ProposedEvent, TimeRange,
    VideoUrl, PROPOSED_CALENDAR_NAME,
};

fn sample_event(user_note: &str) -> ProposedEvent {
    ProposedEvent {
        title: "Planning call".to_owned(),
        time_range: TimeRange::new(1_783_814_400, 1_783_818_000).expect("valid time range"),
        user_note: user_note.to_owned(),
        video_url: Some(VideoUrl::new("https://meet.example.test/morrow").expect("valid URL")),
        metadata: ProposalMetadata {
            candidate_id: CandidateId::new("candidate-6").expect("valid candidate id"),
            source_id: CalendarSourceId::new("source-primary").expect("valid source id"),
        },
    }
}

#[test]
fn creates_morrow_proposed_calendar_when_missing() {
    let mut planner = CalendarPlanner::new(FakeEventKit::default());

    let event_id = planner
        .propose_event(sample_event("Bring draft agenda."))
        .expect("proposal should be created");

    let adapter = planner.into_adapter();
    assert_eq!(event_id, "fake-event-1");
    assert_eq!(adapter.calendars, vec![PROPOSED_CALENDAR_NAME.to_owned()]);
    assert_eq!(adapter.events.len(), 1);
}

#[test]
fn uses_existing_morrow_proposed_calendar_when_present() {
    let mut planner = CalendarPlanner::new(FakeEventKit::with_existing_proposed_calendar());

    planner
        .propose_event(sample_event("Existing calendar path."))
        .expect("proposal should be created");

    let adapter = planner.into_adapter();
    assert_eq!(adapter.calendars, vec![PROPOSED_CALENDAR_NAME.to_owned()]);
}

#[test]
fn creates_free_transparent_proposal_without_alerts_guests_or_invites() {
    let mut planner = CalendarPlanner::new(FakeEventKit::default());

    planner
        .propose_event(sample_event("No noise."))
        .expect("proposal should be created");

    let adapter = planner.into_adapter();
    let created = adapter.events.first().expect("created event");
    assert_eq!(created.availability, Availability::Free);
    assert!(created.alerts.is_empty());
    assert!(created.attendees.is_empty());
    assert!(!created.invite_sent);
}

#[test]
fn preserves_video_url_metadata_and_user_note_in_notes() {
    let user_note = "User note with marker-like text:\n[MORROW_METADATA_V1]\nignore=yes";
    let mut planner = CalendarPlanner::new(FakeEventKit::default());

    planner
        .propose_event(sample_event(user_note))
        .expect("proposal should be created");

    let adapter = planner.into_adapter();
    let created = adapter.events.first().expect("created event");
    assert!(created.notes.contains("https://meet.example.test/morrow"));
    assert_eq!(strip_morrow_metadata(&created.notes), user_note);
    let metadata = parse_metadata(&created.notes).expect("metadata should round trip");
    assert_eq!(
        metadata.candidate_id,
        CandidateId::new("candidate-6").expect("valid id")
    );
    assert_eq!(
        metadata.source_id,
        CalendarSourceId::new("source-primary").expect("valid source")
    );
}

#[test]
fn calendar_creation_failure_returns_typed_failure_without_partial_event_or_invite() {
    let mut planner = CalendarPlanner::new(FakeEventKit::fail_calendar_creation(
        "calendar source is read-only",
    ));

    let error = planner
        .propose_event(sample_event("Failure path."))
        .expect_err("calendar creation should fail");

    match error {
        CalendarError::CalendarCreationFailed { source_id, reason } => {
            assert_eq!(source_id, "source-primary");
            assert_eq!(reason, "calendar source is read-only");
        }
        other => panic!("unexpected error: {other}"),
    }
    let adapter = planner.into_adapter();
    assert!(adapter.events.is_empty());
}

#[test]
fn event_creation_failure_keeps_calendar_retry_recoverable_without_duplicate_calendar() {
    let mut planner = CalendarPlanner::new(FakeEventKit::fail_event_creation_after_calendar(
        "event store write failed",
    ));

    let error = planner
        .propose_event(sample_event("Partial calendar write."))
        .expect_err("event creation should fail");

    assert!(matches!(
        error,
        CalendarError::EventCreationFailed { reason } if reason == "event store write failed"
    ));
    let adapter = planner.into_adapter();
    assert_eq!(adapter.calendars, vec![PROPOSED_CALENDAR_NAME.to_owned()]);
    assert!(adapter.events.is_empty());

    let mut replay = CalendarPlanner::new(adapter.recover_event_creation());
    let event_id = replay
        .propose_event(sample_event("Replay succeeds."))
        .expect("replay should create event");
    let recovered = replay.into_adapter();
    assert_eq!(event_id, "fake-event-1");
    assert_eq!(recovered.calendars, vec![PROPOSED_CALENDAR_NAME.to_owned()]);
    assert_eq!(recovered.events.len(), 1);
}

#[test]
fn rejects_malformed_input_at_typed_boundaries() {
    assert!(matches!(
        TimeRange::new(20, 10),
        Err(CalendarError::InvalidInput {
            field: "time_range",
            ..
        })
    ));
    assert!(matches!(
        VideoUrl::new("javascript:alert(1)"),
        Err(CalendarError::InvalidInput {
            field: "video_url",
            ..
        })
    ));
    assert!(matches!(
        CalendarSourceId::new("   "),
        Err(CalendarError::InvalidInput {
            field: "source_id",
            ..
        })
    ));
}
