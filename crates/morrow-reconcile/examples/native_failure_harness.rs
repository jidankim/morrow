//! Fake native failure harness for Morrow proposal boundary failures.

#![allow(clippy::redundant_pub_crate)]

use std::error::Error;

#[path = "native_failure_harness/bad_probe.rs"]
mod bad_probe;
#[path = "native_failure_harness/calendar_reminders.rs"]
mod calendar_reminders;
#[path = "native_failure_harness/harness_error.rs"]
mod harness_error;
#[path = "native_failure_harness/messages.rs"]
mod messages;
#[path = "native_failure_harness/partial_replay.rs"]
mod partial_replay;
#[path = "native_failure_harness/provider.rs"]
mod provider;
#[path = "native_failure_harness/validation.rs"]
mod validation;

fn main() -> Result<(), Box<dyn Error>> {
    if std::env::args().any(|arg| arg == "--adversarial-bad-probe") {
        return bad_probe::run();
    }

    let messages = messages::run()?;
    let native = calendar_reminders::run()?;
    let replay = partial_replay::run()?;
    let provider = provider::run()?;
    validation::validate_harness(validation::HarnessSummaries {
        messages: &messages,
        native: &native,
        replay: &replay,
        provider: &provider,
    })?;

    println!(
        "permission_denial messages_status={} messages_warning={} reminders_failure={}",
        messages.permission_status, messages.permission_warning, native.permission_failure
    );
    println!(
        "unavailable_chats warning={} emitted_messages={}",
        messages.unavailable_warning, messages.unavailable_messages
    );
    println!(
        "guid_change warning={} emitted_messages={}",
        messages.guid_change_warning, messages.guid_change_messages
    );
    println!(
        "group_membership paused_reason={} emitted_messages={}",
        messages.group_pause_reason, messages.group_messages
    );
    println!(
        "missing_calendar_list calendar_failure={} calendar_events={} list_failure={}",
        native.calendar_failure, native.calendar_failure_events, native.deleted_list_failure
    );
    println!(
        "partial_writes calendar_count={} event_count={} recovered_reason={} replay_create_actions={}",
        native.recovered_calendar_count,
        native.recovered_event_count,
        replay.recovered_reason_count,
        replay.replay_create_actions
    );
    println!(
        "provider_failures quiet_reason={} candidates={}",
        provider.quiet_reason, provider.candidates
    );
    println!(
        "no_duplicates proposed_calendars={} proposed_lists={} replay_candidates={}",
        native.recovered_calendar_count, native.proposed_list_count, replay.replay_candidates
    );
    println!("PASS native_failure_harness");
    Ok(())
}
