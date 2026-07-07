use morrow_storage::{CandidateDraft, CandidateKind};

pub fn candidate_draft() -> CandidateDraft {
    CandidateDraft {
        kind: CandidateKind::CalendarEvent,
        chat_guid: "chat-guid-private".to_owned(),
        anchor_message_guid: "message-guid-private".to_owned(),
        title: "private title".to_owned(),
        confidence_millis: 860,
        normalized_time: "2026-07-15T00:00:00Z".to_owned(),
        evidence_excerpt: "private message excerpt".to_owned(),
        observed_at: 1_783_000_000,
    }
}
