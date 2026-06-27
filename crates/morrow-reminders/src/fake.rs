use std::collections::HashMap;

use crate::{
    ids::{generated_list_id, generated_reminder_id},
    ListId, Operation, ReminderDate, ReminderId, ReminderTime, RemindersError, SourceId,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PermissionMode {
    Allowed,
    Denied,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReminderList {
    pub id: ListId,
    pub source_id: SourceId,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredReminder {
    pub id: ReminderId,
    pub source_id: SourceId,
    pub list_id: ListId,
    pub title: String,
    pub due_date: ReminderDate,
    pub due_time: Option<ReminderTime>,
    pub completed: bool,
}

#[derive(Debug)]
pub struct FakeReminders {
    permission: PermissionMode,
    lists: HashMap<ListId, ReminderList>,
    reminders: HashMap<ReminderId, StoredReminder>,
    next_list: u64,
    next_reminder: u64,
}

impl FakeReminders {
    pub fn allowed() -> Self {
        Self {
            permission: PermissionMode::Allowed,
            lists: HashMap::new(),
            reminders: HashMap::new(),
            next_list: 1,
            next_reminder: 1,
        }
    }

    pub fn permission_denied() -> Self {
        Self {
            permission: PermissionMode::Denied,
            lists: HashMap::new(),
            reminders: HashMap::new(),
            next_list: 1,
            next_reminder: 1,
        }
    }

    pub fn check_permission(&self, operation: Operation) -> Result<(), RemindersError> {
        match self.permission {
            PermissionMode::Allowed => Ok(()),
            PermissionMode::Denied => Err(RemindersError::PermissionDenied { operation }),
        }
    }

    pub fn create_list(
        &mut self,
        source_id: SourceId,
        name: &str,
    ) -> Result<ReminderList, RemindersError> {
        self.check_permission(Operation::CreateList)?;
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(RemindersError::InvalidInput {
                field: "list_name",
                reason: "must not be empty".to_owned(),
            });
        }
        let id = generated_list_id(format!("list-{}", self.next_list));
        self.next_list += 1;
        let list = ReminderList {
            id: id.clone(),
            source_id,
            name: trimmed.to_owned(),
        };
        self.lists.insert(id, list.clone());
        Ok(list)
    }

    pub fn create_reminder(
        &mut self,
        source_id: SourceId,
        list_id: ListId,
        title: String,
        due: ReminderDue,
    ) -> Result<StoredReminder, RemindersError> {
        self.check_permission(Operation::CreateReminder)?;
        if !self.lists.contains_key(&list_id) {
            return Err(RemindersError::ListMissing {
                id: list_id.to_string(),
            });
        }
        let id = generated_reminder_id(format!("reminder-{}", self.next_reminder));
        self.next_reminder += 1;
        let reminder = StoredReminder {
            id: id.clone(),
            source_id,
            list_id,
            title,
            due_date: due.date,
            due_time: due.time,
            completed: false,
        };
        self.reminders.insert(id, reminder.clone());
        Ok(reminder)
    }

    pub fn list_named(&self, source_id: &SourceId, name: &str) -> Option<ReminderList> {
        self.lists
            .values()
            .find(|list| list.source_id == *source_id && list.name == name)
            .cloned()
    }

    pub fn list_count_named(&self, source_id: &SourceId, name: &str) -> usize {
        self.lists
            .values()
            .filter(|list| list.source_id == *source_id && list.name == name)
            .count()
    }

    pub fn delete_list(&mut self, list_id: &ListId) -> Result<(), RemindersError> {
        if self.lists.remove(list_id).is_some() {
            Ok(())
        } else {
            Err(RemindersError::ListMissing {
                id: list_id.to_string(),
            })
        }
    }

    pub fn reminder(&self, reminder_id: &ReminderId) -> Option<StoredReminder> {
        self.reminders.get(reminder_id).cloned()
    }

    pub fn complete_reminder(&mut self, reminder_id: &ReminderId) -> Result<(), RemindersError> {
        let reminder =
            self.reminders
                .get_mut(reminder_id)
                .ok_or_else(|| RemindersError::ReminderMissing {
                    id: reminder_id.to_string(),
                })?;
        reminder.completed = true;
        Ok(())
    }

    pub fn move_reminder(
        &mut self,
        reminder_id: &ReminderId,
        list_id: &ListId,
    ) -> Result<(), RemindersError> {
        if !self.lists.contains_key(list_id) {
            return Err(RemindersError::ListMissing {
                id: list_id.to_string(),
            });
        }
        let reminder =
            self.reminders
                .get_mut(reminder_id)
                .ok_or_else(|| RemindersError::ReminderMissing {
                    id: reminder_id.to_string(),
                })?;
        reminder.list_id = list_id.clone();
        Ok(())
    }

    pub fn update_title(
        &mut self,
        reminder_id: &ReminderId,
        title: String,
    ) -> Result<(), RemindersError> {
        let reminder =
            self.reminders
                .get_mut(reminder_id)
                .ok_or_else(|| RemindersError::ReminderMissing {
                    id: reminder_id.to_string(),
                })?;
        reminder.title = title;
        Ok(())
    }

    pub fn clear(&mut self) {
        self.reminders.clear();
        self.lists.clear();
    }

    pub fn reminder_count(&self) -> usize {
        self.reminders.len()
    }

    pub fn list_count(&self) -> usize {
        self.lists.len()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReminderDue {
    pub date: ReminderDate,
    pub time: Option<ReminderTime>,
}
