use std::env;

use morrow_reminders::{
    FakeReminders, Operation, ReminderAdapter, ReminderDate, ReminderDraft, ReminderObservation,
    ReminderTime, RemindersError, SourceId, MORROW_PROPOSED_LIST_NAME,
};

fn main() -> Result<(), RemindersError> {
    let scenario = parse_scenario()?;
    match scenario.as_str() {
        "proposed-reminder" => run_proposed_reminder(),
        _ => Err(RemindersError::InvalidInput {
            field: "scenario",
            reason: format!("unknown scenario {scenario}"),
        }),
    }
}

fn run_proposed_reminder() -> Result<(), RemindersError> {
    let source = SourceId::parse("smoke-source")?;
    let mut fake = FakeReminders::allowed();
    let adapter = ReminderAdapter::new(source.clone());
    let date_only = adapter.create_proposal(
        &mut fake,
        ReminderDraft::new(
            "Smoke date-only task",
            ReminderDate::parse("2026-07-17")?,
            None,
        )?,
    )?;
    let explicit_time = ReminderTime::parse("09:30")?;
    let timed = adapter.create_proposal(
        &mut fake,
        ReminderDraft::new(
            "Smoke timed task",
            ReminderDate::parse("2026-07-18")?,
            Some(explicit_time),
        )?,
    )?;
    fake.complete_reminder(&date_only.reminder_id)?;
    let completion = adapter.observe(&fake, &date_only.reminder_id)?;
    let personal = fake.create_list(source, "Personal")?;
    fake.move_reminder(&timed.reminder_id, &personal.id)?;
    let moved = adapter.observe(&fake, &timed.reminder_id)?;
    let mut denied = FakeReminders::permission_denied();
    let denied_err = adapter
        .create_proposal(
            &mut denied,
            ReminderDraft::new("Denied task", ReminderDate::parse("2026-07-19")?, None)?,
        )
        .expect_err("permission denied");
    println!("scenario=proposed-reminder");
    println!("proposed_list={MORROW_PROPOSED_LIST_NAME}");
    println!("date_only_due_time=null");
    println!("explicit_due_time={}", explicit_time.as_str());
    println!("completion_observation={}", observation_name(&completion));
    println!("move_observation={}", observation_name(&moved));
    println!(
        "permission_denied={}",
        permission_operation_name(&denied_err).unwrap_or("unexpected")
    );
    fake.clear();
    println!("cleanup_fake_reminders={}", fake.reminder_count());
    println!("cleanup_fake_lists={}", fake.list_count());
    println!("PASS reminders_smoke");
    Ok(())
}

fn parse_scenario() -> Result<String, RemindersError> {
    let mut args = env::args().skip(1);
    match (args.next().as_deref(), args.next(), args.next()) {
        (Some("--scenario"), Some(scenario), None) => Ok(scenario),
        _ => Err(RemindersError::InvalidInput {
            field: "args",
            reason: "usage: reminders_smoke --scenario proposed-reminder".to_owned(),
        }),
    }
}

fn observation_name(observation: &ReminderObservation) -> &'static str {
    match observation {
        ReminderObservation::Pending => "pending",
        ReminderObservation::Approved { list_id: _ } => "approved",
        ReminderObservation::RejectedResolved => "rejected_resolved",
        ReminderObservation::RejectedDeleted => "rejected_deleted",
    }
}

fn permission_operation_name(err: &RemindersError) -> Option<&'static str> {
    match err {
        RemindersError::PermissionDenied {
            operation: Operation::EnsureProposedList,
        } => Some("EnsureProposedList"),
        RemindersError::PermissionDenied {
            operation: Operation::CreateList,
        } => Some("CreateList"),
        RemindersError::PermissionDenied {
            operation: Operation::CreateReminder,
        } => Some("CreateReminder"),
        RemindersError::PermissionDenied {
            operation: Operation::ObserveReminder,
        } => Some("ObserveReminder"),
        RemindersError::PermissionDenied {
            operation: Operation::MutatePending,
        } => Some("MutatePending"),
        RemindersError::InvalidInput { .. }
        | RemindersError::ReminderMissing { .. }
        | RemindersError::ListMissing { .. }
        | RemindersError::ProposedListMissing { .. }
        | RemindersError::ApprovedMutationRejected { .. } => None,
    }
}
