use crate::request::{IngestionRequest, NativeReadRequest, WhitelistedChat};
use crate::types::{
    ChatGuid, IngestionReport, MessageEvidence, MessageTimestamp, MessagesDataSource, PausedChat,
    RawChat,
};
use crate::validation::{short_excerpt, validate_text};
use crate::MessagesError;

const SECONDS_PER_DAY: i64 = 24 * 60 * 60;

pub fn ingest_selected_threads(
    source: &impl MessagesDataSource,
    request: &IngestionRequest,
) -> Result<IngestionReport, MessagesError> {
    if request.whitelist.is_empty() {
        return Ok(IngestionReport::available());
    }

    let native_request = native_read_request(request)?;
    let batch = match source.read_recent(&native_request) {
        Ok(batch) => batch,
        Err(MessagesError::PermissionDenied) => {
            return Ok(IngestionReport::unavailable("messages_permission_denied"));
        }
        Err(err) => return Err(err),
    };

    let mut report = IngestionReport::available();
    let mut seen_whitelisted = Vec::new();
    for chat in batch.chats {
        let Some(whitelisted) = whitelist_entry(&request.whitelist, &chat.guid) else {
            continue;
        };
        seen_whitelisted.push(chat.guid.clone());
        if chat.participant_count != whitelisted.participant_count {
            report.paused_chats.push(PausedChat {
                chat_guid: chat.guid,
                reason: "participant_count_changed".to_owned(),
            });
            continue;
        }
        if participant_ids_changed(whitelisted, &chat) {
            report.paused_chats.push(PausedChat {
                chat_guid: chat.guid,
                reason: "participant_identity_changed".to_owned(),
            });
            continue;
        }
        append_recent_messages(&mut report, &native_request, &chat)?;
    }
    append_unavailable_chat_warnings(&mut report, &request.whitelist, &seen_whitelisted);
    report.messages.sort_by_key(|message| message.timestamp);
    Ok(report)
}

fn native_read_request(request: &IngestionRequest) -> Result<NativeReadRequest, MessagesError> {
    let days = i64::from(request.backfill_days.as_u8());
    let seconds = days
        .checked_mul(SECONDS_PER_DAY)
        .ok_or_else(|| MessagesError::InvalidInput {
            field: "backfill_days",
            reason: "window is too large".to_owned(),
        })?;
    let since =
        request
            .now
            .as_i64()
            .checked_sub(seconds)
            .ok_or_else(|| MessagesError::InvalidInput {
                field: "backfill_days",
                reason: "window starts before timestamp zero".to_owned(),
            })?;
    Ok(NativeReadRequest {
        chat_guids: request
            .whitelist
            .iter()
            .map(|chat| chat.chat_guid.clone())
            .collect(),
        since: MessageTimestamp::new(since)?,
        until: request.now,
    })
}

fn whitelist_entry<'a>(
    whitelist: &'a [WhitelistedChat],
    chat_guid: &ChatGuid,
) -> Option<&'a WhitelistedChat> {
    whitelist
        .iter()
        .find(|entry| entry.chat_guid.as_str() == chat_guid.as_str())
}

fn participant_ids_changed(whitelisted: &WhitelistedChat, chat: &RawChat) -> bool {
    if whitelisted.participant_ids.is_empty() || chat.participant_ids.is_empty() {
        return false;
    }
    if whitelisted.participant_ids.len() != chat.participant_ids.len() {
        return true;
    }
    let mut expected = whitelisted
        .participant_ids
        .iter()
        .map(|id| id.as_str())
        .collect::<Vec<_>>();
    let mut actual = chat
        .participant_ids
        .iter()
        .map(|id| id.as_str())
        .collect::<Vec<_>>();
    expected.sort_unstable();
    actual.sort_unstable();
    expected != actual
}

fn append_unavailable_chat_warnings(
    report: &mut IngestionReport,
    whitelist: &[WhitelistedChat],
    seen_chats: &[ChatGuid],
) {
    for whitelisted in whitelist {
        if seen_chats
            .iter()
            .any(|seen| seen.as_str() == whitelisted.chat_guid.as_str())
        {
            continue;
        }
        report.warnings.push(format!(
            "chat_unavailable:{}",
            whitelisted.chat_guid.as_str()
        ));
    }
}

fn append_recent_messages(
    report: &mut IngestionReport,
    window: &NativeReadRequest,
    chat: &RawChat,
) -> Result<(), MessagesError> {
    for message in &chat.messages {
        if message.chat_guid.as_str() != chat.guid.as_str() {
            continue;
        }
        validate_text("message_text", &message.text, 4_000)?;
        let timestamp = message.timestamp.as_i64();
        if timestamp < window.since.as_i64() || timestamp > window.until.as_i64() {
            continue;
        }
        report.messages.push(MessageEvidence {
            chat_guid: chat.guid.clone(),
            message_guid: message.message_guid.clone(),
            timestamp: message.timestamp,
            participant_count: chat.participant_count,
            tapback_signal: message.tapback.is_some(),
            excerpt: short_excerpt(&message.text),
            evidence_pointer: format!(
                "messages://{}/{}",
                chat.guid.as_str(),
                message.message_guid.as_str()
            ),
        });
    }
    Ok(())
}
