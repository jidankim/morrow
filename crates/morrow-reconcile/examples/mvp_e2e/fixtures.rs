use morrow_detection::{
    AiProvider, ConfidenceThreshold, DetectionConfig, DetectionOutcome, DetectionPipeline,
    ProviderError, ProviderIdentity, ProviderRequest, ProviderResponse, ReferenceTime,
    SourceExcerptPolicy,
};
use morrow_messages::{
    ingest_selected_threads, BackfillDays, ChatGuid, IngestionRequest, MessageGuid,
    MessageTimestamp, MessagesDataSource, MessagesError, NativeBatch, NativeReadRequest,
    ParticipantId, RawChat, RawMessage, TapbackKind, WhitelistedChat,
};
use morrow_storage::{CandidateDraft, Store};

use crate::feedback::{record_candidate_feedback, record_quiet_feedback};

const NON_WHITELISTED_SECRET: &str = "NON_WHITELISTED_NEVER_STORE_TASK12";

/// Candidate set detected from whitelisted message fixtures.
#[derive(Debug)]
pub struct WorkflowCandidates {
    /// Calendar candidate detected from the fixture provider.
    pub calendar: CandidateDraft,
    /// Reminder candidate detected from the fixture provider.
    pub reminder: CandidateDraft,
    /// Number of quiet-log records persisted.
    pub quiet_logs: usize,
    /// Whether the non-whitelisted chat was ignored.
    pub ignored_non_whitelisted: bool,
}

/// Detects workflow candidates from whitelisted synthetic Messages data.
pub fn detect_from_whitelisted_messages(
    store: &Store,
) -> Result<WorkflowCandidates, Box<dyn std::error::Error>> {
    let source = FixtureMessages;
    let request = ingestion_request()?;
    let report = ingest_selected_threads(&source, &request)?;
    if report.messages.iter().any(|message| {
        message.chat_guid.as_str() == "chat-non-whitelisted"
            || message.excerpt.contains(NON_WHITELISTED_SECRET)
    }) {
        return Err("non-whitelisted message reached ingestion report".into());
    }

    let pipeline = DetectionPipeline::new(&FixtureProvider);
    let detection = pipeline.detect(&report.messages, &detection_config()?);
    let mut calendar = None;
    let mut reminder = None;
    let mut quiet_logs = 0usize;

    for outcome in detection.outcomes {
        match outcome {
            DetectionOutcome::Candidate(candidate) => {
                store.create_candidate(candidate.clone())?;
                record_candidate_feedback(store, &candidate)?;
                match candidate.kind {
                    morrow_storage::CandidateKind::CalendarEvent => calendar = Some(candidate),
                    morrow_storage::CandidateKind::TaskReminder => reminder = Some(candidate),
                    morrow_storage::CandidateKind::EventUpdate
                    | morrow_storage::CandidateKind::EventReschedule
                    | morrow_storage::CandidateKind::EventCancellation
                    | morrow_storage::CandidateKind::ReminderUpdate
                    | morrow_storage::CandidateKind::ReminderReschedule
                    | morrow_storage::CandidateKind::ReminderCancellation => {}
                }
            }
            DetectionOutcome::QuietLog(quiet) => {
                store.record_quiet_log(quiet.clone())?;
                record_quiet_feedback(store, &quiet)?;
                quiet_logs += 1;
            }
        }
    }

    Ok(WorkflowCandidates {
        calendar: calendar.ok_or("missing calendar candidate")?,
        reminder: reminder.ok_or("missing reminder candidate")?,
        quiet_logs,
        ignored_non_whitelisted: true,
    })
}

struct FixtureMessages;

impl MessagesDataSource for FixtureMessages {
    fn read_recent(&self, request: &NativeReadRequest) -> Result<NativeBatch, MessagesError> {
        if !request
            .chat_guids
            .iter()
            .any(|chat| chat.as_str() == "chat-alpha")
        {
            return Ok(NativeBatch::default());
        }
        Ok(NativeBatch {
            chats: vec![whitelisted_chat()?, non_whitelisted_chat()?],
        })
    }
}

struct FixtureProvider;

impl AiProvider for FixtureProvider {
    fn extract(&self, request: ProviderRequest<'_>) -> Result<ProviderResponse, ProviderError> {
        let message = request
            .evidence()
            .first()
            .ok_or_else(|| ProviderError::Unavailable {
                reason: "missing evidence".to_owned(),
            })?;
        let raw = match message.message_guid.as_str() {
            "msg-reminder-date-only" => REMINDER_PROVIDER_RESPONSE,
            _ => CALENDAR_PROVIDER_RESPONSE,
        };
        Ok(ProviderResponse::new(raw))
    }
}

const REMINDER_PROVIDER_RESPONSE: &str = concat!(
    r#"{"kind":"task_reminder","title":"Send launch notes","confidence_millis":810,"#,
    r#""normalized_time":"2026-07-02T00:00:00[Asia/Seoul]","#,
    r#""anchor_message_guid":"msg-reminder-date-only","#,
    r#""evidence_message_guids":["msg-reminder-date-only"]}"#,
);

const CALENDAR_PROVIDER_RESPONSE: &str = concat!(
    r#"{"kind":"calendar_event","title":"Unexpected","confidence_millis":100,"#,
    r#""normalized_time":"2026-07-03T10:00:00[Asia/Seoul]","#,
    r#""anchor_message_guid":"msg-unknown","evidence_message_guids":["msg-unknown"]}"#,
);

fn ingestion_request() -> Result<IngestionRequest, Box<dyn std::error::Error>> {
    let chat = WhitelistedChat::with_participants(
        ChatGuid::parse("chat-alpha")?,
        2,
        vec![
            ParticipantId::parse("user-main")?,
            ParticipantId::parse("contact-a")?,
        ],
    )?;
    Ok(IngestionRequest::new(
        vec![chat],
        BackfillDays::new(7)?,
        MessageTimestamp::new(1_782_390_000)?,
    )?)
}

fn detection_config() -> Result<DetectionConfig, Box<dyn std::error::Error>> {
    Ok(DetectionConfig {
        reference: ReferenceTime::parse("2026-06-25T09:00:00", "Asia/Seoul")?,
        threshold: ConfidenceThreshold::new(550)?,
        provider: ProviderIdentity::new("fake-provider", "offline-contract", "prompt-v1")?,
        source_excerpts: SourceExcerptPolicy::Include,
    })
}

fn whitelisted_chat() -> Result<RawChat, MessagesError> {
    let chat_guid = ChatGuid::parse("chat-alpha")?;
    Ok(RawChat {
        guid: chat_guid.clone(),
        participant_count: 2,
        participant_ids: vec![
            ParticipantId::parse("contact-a")?,
            ParticipantId::parse("user-main")?,
        ],
        messages: vec![
            RawMessage {
                chat_guid: chat_guid.clone(),
                message_guid: MessageGuid::parse("msg-calendar-meeting")?,
                timestamp: MessageTimestamp::new(1_782_351_000)?,
                text: "Confirmed project review meeting 2026-07-01 15:30. Agenda has the launch checklist and partner notes. DO_NOT_STORE_FULL_WHITELISTED_MESSAGE_TASK12"
                    .to_owned(),
                tapback: Some(TapbackKind::Like),
            },
            RawMessage {
                chat_guid: chat_guid.clone(),
                message_guid: MessageGuid::parse("msg-reminder-date-only")?,
                timestamp: MessageTimestamp::new(1_782_352_000)?,
                text: [
                    "I will send launch notes by 2026-07-02 after the morning review and ",
                    "include the partner checklist, rollout notes, and ownership table. ",
                    "DO_NOT_STORE_FULL_REMINDER_MESSAGE_TASK12",
                ]
                .concat(),
                tapback: None,
            },
            RawMessage {
                chat_guid,
                message_guid: MessageGuid::parse("msg-quiet-negative")?,
                timestamp: MessageTimestamp::new(1_782_353_000)?,
                text: "Random coffee beans are good today".to_owned(),
                tapback: None,
            },
        ],
    })
}

fn non_whitelisted_chat() -> Result<RawChat, MessagesError> {
    let chat_guid = ChatGuid::parse("chat-non-whitelisted")?;
    Ok(RawChat {
        guid: chat_guid.clone(),
        participant_count: 2,
        participant_ids: vec![
            ParticipantId::parse("contact-x")?,
            ParticipantId::parse("user-main")?,
        ],
        messages: vec![RawMessage {
            chat_guid,
            message_guid: MessageGuid::parse("msg-non-whitelisted")?,
            timestamp: MessageTimestamp::new(1_782_354_000)?,
            text: format!("Private ignored message {NON_WHITELISTED_SECRET}"),
            tapback: Some(TapbackKind::Like),
        }],
    })
}
