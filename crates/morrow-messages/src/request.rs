use std::collections::BTreeSet;

use crate::types::{ChatGuid, MessageTimestamp, ParticipantId};
use crate::MessagesError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackfillDays(u8);

impl BackfillDays {
    pub fn new(value: u8) -> Result<Self, MessagesError> {
        if (1..=7).contains(&value) {
            Ok(Self(value))
        } else {
            Err(MessagesError::InvalidInput {
                field: "backfill_days",
                reason: "must be between 1 and 7".to_owned(),
            })
        }
    }

    pub const fn as_u8(self) -> u8 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WhitelistedChat {
    pub chat_guid: ChatGuid,
    pub participant_count: u16,
    pub participant_ids: Vec<ParticipantId>,
}

impl WhitelistedChat {
    pub fn new(chat_guid: ChatGuid, participant_count: u16) -> Result<Self, MessagesError> {
        Self::with_participants(chat_guid, participant_count, Vec::new())
    }

    pub fn with_participants(
        chat_guid: ChatGuid,
        participant_count: u16,
        participant_ids: Vec<ParticipantId>,
    ) -> Result<Self, MessagesError> {
        validate_participants(participant_count, &participant_ids)?;
        Ok(Self {
            chat_guid,
            participant_count,
            participant_ids,
        })
    }
}

fn validate_participants(
    participant_count: u16,
    participant_ids: &[ParticipantId],
) -> Result<(), MessagesError> {
    if participant_count == 0 {
        return Err(MessagesError::InvalidInput {
            field: "participant_count",
            reason: "must be greater than zero".to_owned(),
        });
    }
    if !participant_ids.is_empty() && participant_ids.len() != usize::from(participant_count) {
        return Err(MessagesError::InvalidInput {
            field: "participant_ids",
            reason: "must match participant_count".to_owned(),
        });
    }
    let mut seen = BTreeSet::new();
    for id in participant_ids {
        if !seen.insert(id.as_str()) {
            return Err(MessagesError::InvalidInput {
                field: "participant_ids",
                reason: "must be unique stable identifiers".to_owned(),
            });
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngestionRequest {
    pub whitelist: Vec<WhitelistedChat>,
    pub backfill_days: BackfillDays,
    pub now: MessageTimestamp,
}

impl IngestionRequest {
    pub fn new(
        whitelist: Vec<WhitelistedChat>,
        backfill_days: BackfillDays,
        now: MessageTimestamp,
    ) -> Result<Self, MessagesError> {
        Ok(Self {
            whitelist,
            backfill_days,
            now,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeReadRequest {
    pub chat_guids: Vec<ChatGuid>,
    pub since: MessageTimestamp,
    pub until: MessageTimestamp,
}
