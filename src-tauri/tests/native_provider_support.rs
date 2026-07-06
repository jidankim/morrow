use morrow_detection::{
    ConfidenceThreshold, ListReminderProfile, ProviderIdentity, ReferenceTime, SourceExcerptPolicy,
};
use morrow_lib::native_bridge::OPENAI_MODEL;
use morrow_messages::{ChatGuid, MessageEvidence, MessageGuid, MessageTimestamp};

pub fn candidate_json() -> &'static str {
    "{\"kind\":\"calendar_event\",\"title\":\"Provider meeting\",\"confidence_millis\":800,\
     \"normalized_time\":\"2026-06-26T15:00:00[Asia/Seoul]\",\
     \"anchor_message_guid\":\"msg-ambiguous-1\",\
     \"evidence_message_guids\":[\"msg-ambiguous-1\"]}"
}

pub fn message(
    chat_guid: &str,
    message_guid: &str,
    excerpt: &str,
) -> Result<MessageEvidence, String> {
    Ok(MessageEvidence {
        chat_guid: ChatGuid::parse(chat_guid).map_err(|error| error.to_string())?,
        message_guid: MessageGuid::parse(message_guid).map_err(|error| error.to_string())?,
        timestamp: MessageTimestamp::new(1_782_352_400).map_err(|error| error.to_string())?,
        participant_count: 2,
        tapback_signal: true,
        excerpt: excerpt.to_owned(),
        evidence_pointer: format!("messages://{chat_guid}/{message_guid}"),
    })
}

pub fn config() -> Result<morrow_detection::DetectionConfig, String> {
    Ok(morrow_detection::DetectionConfig {
        reference: ReferenceTime::parse("2026-06-25T09:00:00", "Asia/Seoul")
            .map_err(|error| error.to_string())?,
        threshold: ConfidenceThreshold::new(550).map_err(|error| error.to_string())?,
        provider: ProviderIdentity::new("openai", OPENAI_MODEL, "native-provider-v1")
            .map_err(|error| error.to_string())?,
        profile: ListReminderProfile::disabled(),
        source_excerpts: SourceExcerptPolicy::Include,
    })
}
