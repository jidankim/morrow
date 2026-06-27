use crate::{
    fake::ReminderDue, FakeReminders, ListId, Operation, ReminderDate, ReminderId, ReminderTime,
    RemindersError, SourceId, MORROW_PROPOSED_LIST_NAME,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReminderDraft {
    pub title: String,
    pub due_date: ReminderDate,
    pub due_time: Option<ReminderTime>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatedReminder {
    pub reminder_id: ReminderId,
    pub list_id: ListId,
    pub list_name: String,
    pub due_date: ReminderDate,
    pub due_time: Option<ReminderTime>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReminderObservation {
    Pending,
    Approved { list_id: ListId },
    RejectedResolved,
    RejectedDeleted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReminderAdapter {
    selected_source_id: SourceId,
}

impl ReminderDraft {
    pub fn new(
        title: &str,
        due_date: ReminderDate,
        due_time: Option<ReminderTime>,
    ) -> Result<Self, RemindersError> {
        let trimmed = title.trim();
        if trimmed.is_empty() {
            return Err(RemindersError::InvalidInput {
                field: "title",
                reason: "must not be empty".to_owned(),
            });
        }
        Ok(Self {
            title: trimmed.to_owned(),
            due_date,
            due_time,
        })
    }
}

impl ReminderAdapter {
    pub const fn new(selected_source_id: SourceId) -> Self {
        Self { selected_source_id }
    }

    pub fn create_proposal(
        &self,
        reminders: &mut FakeReminders,
        draft: ReminderDraft,
    ) -> Result<CreatedReminder, RemindersError> {
        let proposed_list = self.ensure_proposed_list(reminders)?;
        let due = ReminderDue {
            date: draft.due_date,
            time: draft.due_time,
        };
        let stored = reminders.create_reminder(
            self.selected_source_id.clone(),
            proposed_list.id.clone(),
            draft.title,
            due,
        )?;
        Ok(CreatedReminder {
            reminder_id: stored.id,
            list_id: proposed_list.id,
            list_name: proposed_list.name,
            due_date: stored.due_date,
            due_time: stored.due_time,
        })
    }

    pub fn observe(
        &self,
        reminders: &FakeReminders,
        reminder_id: &ReminderId,
    ) -> Result<ReminderObservation, RemindersError> {
        reminders.check_permission(Operation::ObserveReminder)?;
        let Some(reminder) = reminders.reminder(reminder_id) else {
            return Ok(ReminderObservation::RejectedDeleted);
        };
        let proposed = reminders
            .list_named(&self.selected_source_id, MORROW_PROPOSED_LIST_NAME)
            .ok_or_else(|| RemindersError::ProposedListMissing {
                source_id: self.selected_source_id.to_string(),
            })?;
        if reminder.list_id != proposed.id {
            return Ok(ReminderObservation::Approved {
                list_id: reminder.list_id,
            });
        }
        match reminder.completed {
            true => Ok(ReminderObservation::RejectedResolved),
            false => Ok(ReminderObservation::Pending),
        }
    }

    pub fn apply_pending_title(
        &self,
        reminders: &mut FakeReminders,
        reminder_id: &ReminderId,
        title: &str,
    ) -> Result<(), RemindersError> {
        reminders.check_permission(Operation::MutatePending)?;
        let reminder =
            reminders
                .reminder(reminder_id)
                .ok_or_else(|| RemindersError::ReminderMissing {
                    id: reminder_id.to_string(),
                })?;
        let proposed = reminders
            .list_named(&self.selected_source_id, MORROW_PROPOSED_LIST_NAME)
            .ok_or_else(|| RemindersError::ProposedListMissing {
                source_id: self.selected_source_id.to_string(),
            })?;
        if reminder.list_id != proposed.id {
            return Err(RemindersError::ApprovedMutationRejected {
                reminder_id: reminder_id.to_string(),
            });
        }
        let trimmed = title.trim();
        if trimmed.is_empty() {
            return Err(RemindersError::InvalidInput {
                field: "title",
                reason: "must not be empty".to_owned(),
            });
        }
        reminders.update_title(reminder_id, trimmed.to_owned())
    }

    fn ensure_proposed_list(
        &self,
        reminders: &mut FakeReminders,
    ) -> Result<crate::ReminderList, RemindersError> {
        reminders.check_permission(Operation::EnsureProposedList)?;
        if let Some(existing) =
            reminders.list_named(&self.selected_source_id, MORROW_PROPOSED_LIST_NAME)
        {
            return Ok(existing);
        }
        reminders.create_list(self.selected_source_id.clone(), MORROW_PROPOSED_LIST_NAME)
    }
}
