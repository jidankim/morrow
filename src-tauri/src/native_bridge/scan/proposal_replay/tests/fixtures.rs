use morrow_storage::{
    CalendarProposalPayload, CandidateDraft, CandidateId as StorageCandidateId, CandidateKind,
    ReminderProposalPayload,
};

pub(super) const CANDIDATE_ID: &str = "morrow_0000000000000001";

pub(super) fn reminder_payload(normalized_time: &str) -> ReminderProposalPayload {
    ReminderProposalPayload {
        candidate_id: StorageCandidateId::from_storage(CANDIDATE_ID).expect("candidate id"),
        kind: CandidateKind::TaskReminder,
        normalized_time: normalized_time.to_owned(),
        title: "Messages reminder candidate".to_owned(),
        source_id: "morrow-selected-reminders".to_owned(),
    }
}

pub(super) fn calendar_payload(normalized_time: &str) -> CalendarProposalPayload {
    CalendarProposalPayload {
        candidate_id: StorageCandidateId::from_storage(CANDIDATE_ID).expect("candidate id"),
        kind: CandidateKind::CalendarEvent,
        normalized_time: normalized_time.to_owned(),
        title: "Messages event candidate".to_owned(),
        source_id: "morrow-selected-messages".to_owned(),
    }
}

pub(super) fn candidate_draft(anchor_message_guid: &str) -> CandidateDraft {
    CandidateDraft {
        kind: CandidateKind::CalendarEvent,
        chat_guid: "public-chat".to_owned(),
        anchor_message_guid: anchor_message_guid.to_owned(),
        title: "Morrow QA".to_owned(),
        confidence_millis: 900,
        normalized_time: "2026-07-15T14:00:00Z".to_owned(),
        evidence_excerpt: "source hidden".to_owned(),
        observed_at: 1_782_352_398,
    }
}
