use std::collections::BTreeSet;

use crate::types::{ChatGuid, MessageTimestamp, ParticipantId};
use crate::MessagesError;

const DEFAULT_DISCOVERY_LABEL: &str = "Messages chat";

pub trait MessagesDiscoveryDataSource {
    fn discover_chats(&self) -> Result<MessagesDiscoveryReport, MessagesError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredChat {
    chat_guid: ChatGuid,
    display_label: String,
    participant_count: u16,
    participant_ids: Vec<ParticipantId>,
    latest_activity_timestamp: MessageTimestamp,
}

pub struct DiscoveredChatParts<'a> {
    pub chat_guid: &'a str,
    pub display_label: &'a str,
    pub participant_count: u16,
    pub participant_ids: Vec<&'a str>,
    pub latest_activity_timestamp: i64,
}

impl DiscoveredChat {
    pub fn parse(parts: DiscoveredChatParts<'_>) -> Result<Self, MessagesError> {
        Ok(Self {
            chat_guid: ChatGuid::parse(parts.chat_guid)?,
            display_label: discovery_label(parts.display_label),
            participant_count: parts.participant_count,
            participant_ids: discovery_participant_ids(
                parts.participant_count,
                &parts.participant_ids,
            )?,
            latest_activity_timestamp: MessageTimestamp::new(parts.latest_activity_timestamp)?,
        })
    }

    pub fn chat_guid(&self) -> &ChatGuid {
        &self.chat_guid
    }

    pub fn display_label(&self) -> &str {
        &self.display_label
    }

    pub const fn participant_count(&self) -> u16 {
        self.participant_count
    }

    pub fn participant_ids(&self) -> &[ParticipantId] {
        &self.participant_ids
    }

    pub const fn latest_activity_timestamp(&self) -> MessageTimestamp {
        self.latest_activity_timestamp
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessagesDiscoveryStatus {
    Ready,
    Empty,
    PermissionDenied,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessagesDiscoveryReport {
    status: MessagesDiscoveryStatus,
    chats: Vec<DiscoveredChat>,
}

impl MessagesDiscoveryReport {
    pub fn new(
        status: MessagesDiscoveryStatus,
        chats: Vec<DiscoveredChat>,
    ) -> Result<Self, MessagesError> {
        let valid = match status {
            MessagesDiscoveryStatus::Ready => !chats.is_empty(),
            MessagesDiscoveryStatus::Empty
            | MessagesDiscoveryStatus::PermissionDenied
            | MessagesDiscoveryStatus::Unavailable => chats.is_empty(),
        };
        if valid {
            Ok(Self { status, chats })
        } else {
            Err(invalid(
                "discovery_report",
                "status and chat list are inconsistent",
            ))
        }
    }

    pub fn ready(chats: Vec<DiscoveredChat>) -> Result<Self, MessagesError> {
        Self::new(MessagesDiscoveryStatus::Ready, chats)
    }

    pub fn empty() -> Self {
        Self::degraded(MessagesDiscoveryStatus::Empty)
    }

    pub fn permission_denied() -> Self {
        Self::degraded(MessagesDiscoveryStatus::PermissionDenied)
    }

    pub fn unavailable() -> Self {
        Self::degraded(MessagesDiscoveryStatus::Unavailable)
    }

    pub const fn status(&self) -> MessagesDiscoveryStatus {
        self.status
    }

    pub fn chats(&self) -> &[DiscoveredChat] {
        &self.chats
    }

    fn degraded(status: MessagesDiscoveryStatus) -> Self {
        Self {
            status,
            chats: Vec::new(),
        }
    }
}

fn discovery_label(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() || is_raw_handle_like(trimmed) {
        DEFAULT_DISCOVERY_LABEL.to_owned()
    } else {
        trimmed.to_owned()
    }
}

fn discovery_participant_ids(
    participant_count: u16,
    values: &[&str],
) -> Result<Vec<ParticipantId>, MessagesError> {
    if participant_count == 0 {
        return Err(invalid("participant_count", "must be greater than zero"));
    }
    if values.len() != usize::from(participant_count) {
        return Err(invalid("participant_ids", "must match participant_count"));
    }
    let mut seen = BTreeSet::new();
    let mut participant_ids = Vec::with_capacity(values.len());
    for value in values {
        if is_raw_handle_like(value) {
            return Err(invalid(
                "participant_ids",
                "must be opaque local identifiers",
            ));
        }
        let participant_id = ParticipantId::parse(value)?;
        if !seen.insert(participant_id.as_str().to_owned()) {
            return Err(invalid(
                "participant_ids",
                "must be unique stable identifiers",
            ));
        }
        participant_ids.push(participant_id);
    }
    Ok(participant_ids)
}

fn invalid(field: &'static str, reason: &'static str) -> MessagesError {
    MessagesError::InvalidInput {
        field,
        reason: reason.to_owned(),
    }
}

fn is_raw_handle_like(value: &str) -> bool {
    let normalized = value.trim().to_ascii_lowercase();
    normalized.contains('@')
        || normalized.starts_with("tel:")
        || normalized.starts_with("sms:")
        || normalized.starts_with("sms;")
        || normalized.starts_with("imessage:")
        || normalized.starts_with("imessage;")
        || is_phone_like(&normalized)
}

fn is_phone_like(value: &str) -> bool {
    let digit_count = value.chars().filter(|ch| ch.is_ascii_digit()).count();
    digit_count >= 7
        && value
            .chars()
            .all(|ch| ch.is_ascii_digit() || matches!(ch, '+' | '-' | '(' | ')' | '.' | ' ' | '\t'))
}
