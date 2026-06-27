use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    EnsureProposedList,
    CreateList,
    CreateReminder,
    ObserveReminder,
    MutatePending,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemindersError {
    InvalidInput { field: &'static str, reason: String },
    PermissionDenied { operation: Operation },
    ReminderMissing { id: String },
    ListMissing { id: String },
    ProposedListMissing { source_id: String },
    ApprovedMutationRejected { reminder_id: String },
}

impl Display for RemindersError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput { field, reason } => write!(f, "invalid {field}: {reason}"),
            Self::PermissionDenied { operation } => {
                write!(f, "reminders permission denied during {operation:?}")
            }
            Self::ReminderMissing { id } => write!(f, "reminder not found: {id}"),
            Self::ListMissing { id } => write!(f, "reminders list not found: {id}"),
            Self::ProposedListMissing { source_id } => {
                write!(f, "Morrow Proposed list missing in source {source_id}")
            }
            Self::ApprovedMutationRejected { reminder_id } => {
                write!(
                    f,
                    "refusing automatic mutation of approved reminder {reminder_id}"
                )
            }
        }
    }
}

impl std::error::Error for RemindersError {}
