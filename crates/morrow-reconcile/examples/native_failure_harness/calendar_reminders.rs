use std::error::Error;

use morrow_calendar::{
    CalendarError, CalendarPlanner, CalendarSourceId, CandidateId, FakeEventKit, ProposalMetadata,
    ProposedEvent, TimeRange,
};
use morrow_reminders::{
    FakeReminders, Operation, ReminderAdapter, ReminderDate, ReminderDraft, RemindersError,
    SourceId, MORROW_PROPOSED_LIST_NAME,
};

use crate::harness_error::HarnessError;

#[derive(Debug, Clone)]
pub(crate) struct NativeSummary {
    pub(crate) permission_failure: &'static str,
    pub(crate) calendar_failure: &'static str,
    pub(crate) calendar_failure_events: usize,
    pub(crate) deleted_list_failure: &'static str,
    pub(crate) recovered_calendar_count: usize,
    pub(crate) recovered_event_count: usize,
    pub(crate) proposed_list_count: usize,
}

pub(crate) fn run() -> Result<NativeSummary, Box<dyn Error>> {
    let permission_failure = reminder_permission_failure()?;
    let (calendar_failure, calendar_failure_events) = calendar_creation_failure()?;
    let deleted_list_failure = deleted_list_failure()?;
    let (recovered_calendar_count, recovered_event_count) = recovered_calendar_write()?;
    let proposed_list_count = reminder_no_duplicate_list()?;

    Ok(NativeSummary {
        permission_failure,
        calendar_failure,
        calendar_failure_events,
        deleted_list_failure,
        recovered_calendar_count,
        recovered_event_count,
        proposed_list_count,
    })
}

fn reminder_permission_failure() -> Result<&'static str, Box<dyn Error>> {
    let source = SourceId::parse("source-primary")?;
    let mut reminders = FakeReminders::permission_denied();
    let adapter = ReminderAdapter::new(source);
    let result = adapter.create_proposal(&mut reminders, draft("Denied task")?);
    match result {
        Err(RemindersError::PermissionDenied {
            operation: Operation::EnsureProposedList,
        }) => Ok("permission_denied"),
        Err(error) => Err(HarnessError::UnexpectedOutcome {
            scenario: "reminder_permission_failure",
            expected: "permission_denied",
            actual: format!("{error:?}"),
        }
        .into()),
        Ok(_) => Err(HarnessError::UnexpectedOutcome {
            scenario: "reminder_permission_failure",
            expected: "permission_denied",
            actual: "success".to_owned(),
        }
        .into()),
    }
}

fn calendar_creation_failure() -> Result<(&'static str, usize), Box<dyn Error>> {
    let mut planner = CalendarPlanner::new(FakeEventKit::fail_calendar_creation(
        "calendar source missing",
    ));
    let result = planner.propose_event(proposal("calendar-failure")?);
    let failure = match result {
        Err(CalendarError::CalendarCreationFailed { .. }) => Ok("calendar_creation_failed"),
        Err(error) => Err(HarnessError::UnexpectedOutcome {
            scenario: "calendar_creation_failure",
            expected: "calendar_creation_failed",
            actual: format!("{error:?}"),
        }),
        Ok(_) => Err(HarnessError::UnexpectedOutcome {
            scenario: "calendar_creation_failure",
            expected: "calendar_creation_failed",
            actual: "success".to_owned(),
        }),
    };
    Ok((failure?, planner.into_adapter().events.len()))
}

fn deleted_list_failure() -> Result<&'static str, Box<dyn Error>> {
    let source = SourceId::parse("source-primary")?;
    let mut reminders = FakeReminders::allowed();
    let adapter = ReminderAdapter::new(source);
    let created = adapter.create_proposal(&mut reminders, draft("List disappears")?)?;
    reminders.delete_list(&created.list_id)?;
    let observed = adapter.observe(&reminders, &created.reminder_id);
    match observed {
        Err(RemindersError::ProposedListMissing { .. }) => Ok("proposed_list_missing"),
        Err(error) => Err(HarnessError::UnexpectedOutcome {
            scenario: "deleted_list_failure",
            expected: "proposed_list_missing",
            actual: format!("{error:?}"),
        }
        .into()),
        Ok(_) => Err(HarnessError::UnexpectedOutcome {
            scenario: "deleted_list_failure",
            expected: "proposed_list_missing",
            actual: "observed reminder".to_owned(),
        }
        .into()),
    }
}

fn recovered_calendar_write() -> Result<(usize, usize), CalendarError> {
    let mut planner = CalendarPlanner::new(FakeEventKit::fail_event_creation_after_calendar(
        "event write failed",
    ));
    let result = planner.propose_event(proposal("partial-write")?);
    if !matches!(result, Err(CalendarError::EventCreationFailed { .. })) {
        return Err(CalendarError::EventCreationFailed {
            reason: "partial write did not fail as expected".to_owned(),
        });
    }
    let mut replay = CalendarPlanner::new(planner.into_adapter().recover_event_creation());
    replay.propose_event(proposal("partial-replay")?)?;
    let adapter = replay.into_adapter();
    Ok((adapter.calendars.len(), adapter.events.len()))
}

fn reminder_no_duplicate_list() -> Result<usize, RemindersError> {
    let source = SourceId::parse("source-primary")?;
    let mut reminders = FakeReminders::allowed();
    let adapter = ReminderAdapter::new(source.clone());
    adapter.create_proposal(&mut reminders, draft("First task")?)?;
    adapter.create_proposal(&mut reminders, draft("Second task")?)?;
    Ok(reminders.list_count_named(&source, MORROW_PROPOSED_LIST_NAME))
}

fn proposal(candidate_suffix: &str) -> Result<ProposedEvent, CalendarError> {
    Ok(ProposedEvent {
        title: "Native failure proposal".to_owned(),
        time_range: TimeRange::new(1_783_814_400, 1_783_818_000)?,
        user_note: "Harness note".to_owned(),
        video_url: None,
        metadata: ProposalMetadata {
            candidate_id: CandidateId::new(&format!("candidate-{candidate_suffix}"))?,
            source_id: CalendarSourceId::new("source-primary")?,
        },
    })
}

fn draft(title: &str) -> Result<ReminderDraft, RemindersError> {
    ReminderDraft::new(title, ReminderDate::parse("2026-07-17")?, None)
}
