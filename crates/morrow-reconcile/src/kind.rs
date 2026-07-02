use morrow_storage::{CandidateKind, ExternalSource};

use crate::LifecycleReason;

pub(super) const fn is_manual_change(kind: CandidateKind) -> bool {
    match kind {
        CandidateKind::CalendarEvent | CandidateKind::TaskReminder => false,
        CandidateKind::EventUpdate
        | CandidateKind::EventReschedule
        | CandidateKind::EventCancellation
        | CandidateKind::ReminderUpdate
        | CandidateKind::ReminderReschedule
        | CandidateKind::ReminderCancellation => true,
    }
}

pub(super) const fn manual_change_reason(kind: CandidateKind) -> LifecycleReason {
    match kind {
        CandidateKind::CalendarEvent | CandidateKind::TaskReminder => {
            LifecycleReason::ManualChangeProposalOnly
        }
        CandidateKind::EventUpdate | CandidateKind::ReminderUpdate => {
            LifecycleReason::ManualChangeProposalOnly
        }
        CandidateKind::EventReschedule | CandidateKind::ReminderReschedule => {
            LifecycleReason::CandidateRescheduled
        }
        CandidateKind::EventCancellation | CandidateKind::ReminderCancellation => {
            LifecycleReason::CandidateCancelled
        }
    }
}

pub(super) const fn source_for_kind(kind: CandidateKind) -> ExternalSource {
    match kind {
        CandidateKind::CalendarEvent
        | CandidateKind::EventUpdate
        | CandidateKind::EventReschedule
        | CandidateKind::EventCancellation => ExternalSource::Calendar,
        CandidateKind::TaskReminder
        | CandidateKind::ReminderUpdate
        | CandidateKind::ReminderReschedule
        | CandidateKind::ReminderCancellation => ExternalSource::Reminders,
    }
}
