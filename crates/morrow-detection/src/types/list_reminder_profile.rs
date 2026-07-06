use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListReminderProfile {
    pub enabled: bool,
    pub profile_id: ListReminderProfileId,
    pub profile_version: ListReminderProfileVersion,
    pub routing_mode: ListReminderRoutingMode,
    pub default_due_mode: ListReminderDefaultDueMode,
    pub default_due_time: ListReminderDefaultDueTime,
    pub recurrence_mode: ListReminderRecurrenceMode,
    pub item_output_mode: ListReminderItemOutputMode,
}

impl ListReminderProfile {
    pub const fn disabled() -> Self {
        Self {
            enabled: false,
            profile_id: ListReminderProfileId::ListReminders,
            profile_version: ListReminderProfileVersion::ListRemindersV1,
            routing_mode: ListReminderRoutingMode::ExplicitOnly,
            default_due_mode: ListReminderDefaultDueMode::ExplicitOnly,
            default_due_time: ListReminderDefaultDueTime::TwentyThreeFiftyNine,
            recurrence_mode: ListReminderRecurrenceMode::None,
            item_output_mode: ListReminderItemOutputMode::SingleReminderTitle,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum ListReminderProfileId {
    #[serde(rename = "list-reminders")]
    ListReminders,
}

impl ListReminderProfileId {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ListReminders => "list-reminders",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum ListReminderProfileVersion {
    #[serde(rename = "list-reminders-v1")]
    ListRemindersV1,
}

impl ListReminderProfileVersion {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ListRemindersV1 => "list-reminders-v1",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ListReminderRoutingMode {
    ExplicitOnly,
    ProfileBareQuantityLists,
}

impl ListReminderRoutingMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ExplicitOnly => "explicitOnly",
            Self::ProfileBareQuantityLists => "profileBareQuantityLists",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ListReminderDefaultDueMode {
    ExplicitOnly,
    NextLocalDayAtDefaultTime,
}

impl ListReminderDefaultDueMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ExplicitOnly => "explicitOnly",
            Self::NextLocalDayAtDefaultTime => "nextLocalDayAtDefaultTime",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum ListReminderDefaultDueTime {
    #[serde(rename = "23:59")]
    TwentyThreeFiftyNine,
}

impl ListReminderDefaultDueTime {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TwentyThreeFiftyNine => "23:59",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ListReminderRecurrenceMode {
    None,
}

impl ListReminderRecurrenceMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ListReminderItemOutputMode {
    SingleReminderTitle,
}

impl ListReminderItemOutputMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SingleReminderTitle => "singleReminderTitle",
        }
    }
}
