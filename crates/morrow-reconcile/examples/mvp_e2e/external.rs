#![allow(clippy::redundant_pub_crate)]

use morrow_calendar::{
    parse_metadata, Availability, CalendarPlanner, CalendarSourceId, CandidateId as CalendarId,
    FakeEventKit, ProposalMetadata, ProposedEvent, TimeRange, VideoUrl, PROPOSED_CALENDAR_NAME,
};
use morrow_reminders::{
    FakeReminders, ReminderAdapter, ReminderDate, ReminderDraft, ReminderObservation, ReminderTime,
    SourceId, MORROW_PROPOSED_LIST_NAME,
};
use morrow_storage::{
    CandidateDraft, CandidateId, CandidateKind, CandidateState, ExternalObjectMapping,
    ExternalSource, Store,
};

pub(crate) struct CalendarSummary {
    pub(crate) candidate_id: CandidateId,
    pub(crate) visible_created: u64,
    pub(crate) latency_seconds: u64,
}

pub(crate) struct ReminderSummary {
    pub(crate) candidate_id: CandidateId,
    pub(crate) visible_created: u64,
    pub(crate) latency_seconds: u64,
}

pub(crate) fn create_calendar_proposal(
    store: &Store,
    draft: &CandidateDraft,
) -> Result<CalendarSummary, Box<dyn std::error::Error>> {
    if draft.kind != CandidateKind::CalendarEvent {
        return Err("calendar proposal received non-calendar candidate".into());
    }
    let candidate_id = create_visible_shell(store, draft, ExternalSource::Calendar)?;
    let source_id = CalendarSourceId::new("calendar-source-e2e")?;
    let proposed = ProposedEvent {
        title: draft.title.clone(),
        time_range: TimeRange::new(1_783_000_000, 1_783_003_600)?,
        user_note: format!(
            "Source excerpt: {}\nChat: {}\nWhy: scheduling-shaped fixture",
            draft.evidence_excerpt, draft.chat_guid
        ),
        video_url: Some(VideoUrl::new("https://meet.example/morrow-e2e")?),
        metadata: ProposalMetadata {
            candidate_id: CalendarId::new(candidate_id.as_str())?,
            source_id: source_id.clone(),
        },
    };

    let mut planner = CalendarPlanner::new(FakeEventKit::with_existing_proposed_calendar());
    let external_object_id = planner.propose_event(proposed)?;
    let adapter = planner.into_adapter();
    let event = adapter
        .events
        .first()
        .ok_or("missing fake calendar event")?;
    assert_eq!(event.calendar_name, PROPOSED_CALENDAR_NAME);
    assert_eq!(event.availability, Availability::Free);
    assert!(event.alerts.is_empty());
    assert!(event.attendees.is_empty());
    assert!(!event.invite_sent);
    assert_eq!(
        parse_metadata(&event.notes)?.candidate_id.as_str(),
        candidate_id.as_str()
    );
    assert!(event.notes.contains("Source excerpt:"));

    store.upsert_external_mapping(ExternalObjectMapping {
        candidate_id: candidate_id.clone(),
        source: ExternalSource::Calendar,
        external_object_id,
        external_source_id: source_id.as_str().to_owned(),
        mapped_at: draft.observed_at + 2,
    })?;
    store.transition_candidate(
        &candidate_id,
        CandidateState::Visible,
        "e2e_calendar_visible",
        draft.observed_at + 3,
    )?;

    Ok(CalendarSummary {
        candidate_id,
        visible_created: 1,
        latency_seconds: 3,
    })
}

pub(crate) fn create_reminder_proposals(
    store: &Store,
    draft: &CandidateDraft,
) -> Result<ReminderSummary, Box<dyn std::error::Error>> {
    if draft.kind != CandidateKind::TaskReminder {
        return Err("reminder proposal received non-reminder candidate".into());
    }
    let candidate_id = create_visible_shell(store, draft, ExternalSource::Reminders)?;
    let source_id = SourceId::parse("reminder-source-e2e")?;
    let adapter = ReminderAdapter::new(source_id.clone());
    let mut reminders = FakeReminders::allowed();
    let date_only = adapter.create_proposal(
        &mut reminders,
        ReminderDraft::new(
            &draft.title,
            ReminderDate::parse(date_from_normalized(&draft.normalized_time)?)?,
            None,
        )?,
    )?;
    assert_eq!(date_only.list_name, MORROW_PROPOSED_LIST_NAME);
    assert!(date_only.due_time.is_none());

    let timed = adapter.create_proposal(
        &mut reminders,
        ReminderDraft::new(
            "Timed follow-up",
            ReminderDate::parse("2026-07-03")?,
            Some(ReminderTime::parse("16:30")?),
        )?,
    )?;
    assert_eq!(
        timed
            .due_time
            .ok_or("missing explicit reminder time")?
            .as_str(),
        "16:30"
    );
    reminders.complete_reminder(&date_only.reminder_id)?;
    assert_eq!(
        adapter.observe(&reminders, &date_only.reminder_id)?,
        ReminderObservation::RejectedResolved
    );
    let real_list = reminders.create_list(source_id.clone(), "Real Tasks")?;
    reminders.move_reminder(&timed.reminder_id, &real_list.id)?;
    assert_eq!(
        adapter.observe(&reminders, &timed.reminder_id)?,
        ReminderObservation::Approved {
            list_id: real_list.id
        }
    );

    store.upsert_external_mapping(ExternalObjectMapping {
        candidate_id: candidate_id.clone(),
        source: ExternalSource::Reminders,
        external_object_id: date_only.reminder_id.as_str().to_owned(),
        external_source_id: source_id.as_str().to_owned(),
        mapped_at: draft.observed_at + 2,
    })?;
    store.transition_candidate(
        &candidate_id,
        CandidateState::Visible,
        "e2e_reminder_visible",
        draft.observed_at + 4,
    )?;

    Ok(ReminderSummary {
        candidate_id,
        visible_created: 1,
        latency_seconds: 4,
    })
}

fn create_visible_shell(
    store: &Store,
    draft: &CandidateDraft,
    source: ExternalSource,
) -> Result<CandidateId, Box<dyn std::error::Error>> {
    let candidate_id = CandidateId::derive(
        draft.kind,
        &draft.chat_guid,
        &draft.anchor_message_guid,
        &draft.normalized_time,
    );
    store.transition_candidate(
        &candidate_id,
        CandidateState::CreatingExternal,
        match source {
            ExternalSource::Calendar => "e2e_calendar_creating",
            ExternalSource::Reminders => "e2e_reminder_creating",
        },
        draft.observed_at + 1,
    )?;
    Ok(candidate_id)
}

fn date_from_normalized(value: &str) -> Result<&str, Box<dyn std::error::Error>> {
    let (date, _) = value
        .split_once('T')
        .ok_or("normalized reminder time missing T separator")?;
    Ok(date)
}
