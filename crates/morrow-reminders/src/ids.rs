use std::fmt::{Display, Formatter};

use crate::RemindersError;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceId(String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ListId(String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ReminderId(String);

impl SourceId {
    pub fn parse(raw: &str) -> Result<Self, RemindersError> {
        parse_id(raw, "source_id").map(Self)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl ListId {
    pub fn parse(raw: &str) -> Result<Self, RemindersError> {
        parse_id(raw, "list_id").map(Self)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl ReminderId {
    pub fn parse(raw: &str) -> Result<Self, RemindersError> {
        parse_id(raw, "reminder_id").map(Self)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl Display for SourceId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Display for ListId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Display for ReminderId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

pub(crate) fn generated_list_id(raw: String) -> ListId {
    ListId(raw)
}

pub(crate) fn generated_reminder_id(raw: String) -> ReminderId {
    ReminderId(raw)
}

fn parse_id(raw: &str, field: &'static str) -> Result<String, RemindersError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(RemindersError::InvalidInput {
            field,
            reason: "must not be empty".to_owned(),
        });
    }
    Ok(trimmed.to_owned())
}
