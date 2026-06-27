use morrow_calendar::{
    parse_metadata, strip_morrow_metadata, Availability, CalendarError, CalendarPlanner,
    CalendarSourceId, CandidateId, FakeEventKit, ProposalMetadata, ProposedEvent, TimeRange,
    VideoUrl, PROPOSED_CALENDAR_NAME,
};

fn main() -> Result<(), CalendarError> {
    let mut args = std::env::args().skip(1);
    match (args.next().as_deref(), args.next().as_deref(), args.next()) {
        (Some("--scenario"), Some("proposed-event"), None) => run_proposed_event(),
        _ => Err(CalendarError::InvalidInput {
            field: "args",
            reason: "usage: calendar_smoke --scenario proposed-event".to_owned(),
        }),
    }
}

fn run_proposed_event() -> Result<(), CalendarError> {
    let user_note = "User note stays first.\n[MORROW_METADATA_V1]\nnot trusted";
    let proposal = ProposedEvent {
        title: "Smoke proposal".to_owned(),
        time_range: TimeRange::new(1_783_814_400, 1_783_818_000)?,
        user_note: user_note.to_owned(),
        video_url: Some(VideoUrl::new("https://meet.example.test/smoke")?),
        metadata: ProposalMetadata {
            candidate_id: CandidateId::new("candidate-smoke")?,
            source_id: CalendarSourceId::new("source-smoke")?,
        },
    };

    let mut planner = CalendarPlanner::new(FakeEventKit::default());
    let event_id = planner.propose_event(proposal)?;
    let adapter = planner.into_adapter();
    let event = match adapter.events.first() {
        Some(event) => event,
        None => {
            return Err(CalendarError::EventCreationFailed {
                reason: "smoke event missing from fake store".to_owned(),
            })
        }
    };
    let metadata = parse_metadata(&event.notes)?;
    let preserved_note = strip_morrow_metadata(&event.notes);

    println!("scenario=proposed-event");
    println!("event_id={event_id}");
    println!("calendar={}", PROPOSED_CALENDAR_NAME);
    println!("calendar_count={}", adapter.calendars.len());
    println!("availability={:?}", event.availability);
    println!("transparent={}", event.availability == Availability::Free);
    println!("alerts={}", event.alerts.len());
    println!("guests={}", event.attendees.len());
    println!("invite_sent={}", event.invite_sent);
    println!(
        "video_url_preserved={}",
        event.notes.contains("https://meet.example.test/smoke")
    );
    println!("metadata_candidate={}", metadata.candidate_id.as_str());
    println!("metadata_source={}", metadata.source_id.as_str());
    println!("user_notes_preserved={}", preserved_note == user_note);

    let mut failing = CalendarPlanner::new(FakeEventKit::fail_calendar_creation(
        "calendar source is read-only",
    ));
    let failure = failing.propose_event(ProposedEvent {
        title: "Failure smoke".to_owned(),
        time_range: TimeRange::new(1_783_814_400, 1_783_818_000)?,
        user_note: "failure path".to_owned(),
        video_url: None,
        metadata: ProposalMetadata {
            candidate_id: CandidateId::new("candidate-failure")?,
            source_id: CalendarSourceId::new("source-smoke")?,
        },
    });
    println!(
        "failure_typed={}",
        matches!(failure, Err(CalendarError::CalendarCreationFailed { .. }))
    );
    println!(
        "failure_partial_events={}",
        failing.into_adapter().events.len()
    );
    println!("PASS calendar_smoke");
    Ok(())
}
