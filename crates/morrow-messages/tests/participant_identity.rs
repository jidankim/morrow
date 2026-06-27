use morrow_messages::{
    ingest_selected_threads, BackfillDays, ChatGuid, IngestionRequest, MessageGuid,
    MessageTimestamp, MessagesDataSource, MessagesError, NativeBatch, NativeReadRequest,
    ParticipantId, RawChat, RawMessage, WhitelistedChat,
};

#[derive(Debug)]
struct FakeMessages {
    batch: NativeBatch,
}

impl MessagesDataSource for FakeMessages {
    fn read_recent(&self, _request: &NativeReadRequest) -> Result<NativeBatch, MessagesError> {
        Ok(self.batch.clone())
    }
}

#[test]
fn pauses_group_chat_when_participant_identity_changes_with_same_count() {
    // Given: a whitelisted group chat confirmed with two synthetic participant IDs.
    let source = FakeMessages {
        batch: NativeBatch {
            chats: vec![RawChat {
                guid: ChatGuid::parse("chat-group").expect("chat guid"),
                participant_count: 2,
                participant_ids: vec![
                    participant("participant-hash-a"),
                    participant("participant-hash-c"),
                ],
                messages: vec![message("chat-group", "msg-group", 1_783_000_180)],
            }],
        },
    };
    let request = IngestionRequest::new(
        vec![WhitelistedChat::with_participants(
            ChatGuid::parse("chat-group").expect("chat guid"),
            2,
            vec![
                participant("participant-hash-a"),
                participant("participant-hash-b"),
            ],
        )
        .expect("whitelisted chat")],
        BackfillDays::new(7).expect("backfill"),
        MessageTimestamp::new(1_783_000_200).expect("now"),
    )
    .expect("request");

    // When: ingestion sees the same count with a different participant identity set.
    let report = ingest_selected_threads(&source, &request).expect("ingest");

    // Then: monitoring pauses until reconfirmed and no group messages are emitted.
    assert!(report.messages.is_empty());
    assert_eq!(report.paused_chats.len(), 1);
    assert_eq!(report.paused_chats[0].chat_guid.as_str(), "chat-group");
    assert_eq!(
        report.paused_chats[0].reason,
        "participant_identity_changed"
    );
}

fn message(chat_guid: &str, message_guid: &str, timestamp: i64) -> RawMessage {
    RawMessage {
        chat_guid: ChatGuid::parse(chat_guid).expect("chat guid"),
        message_guid: MessageGuid::parse(message_guid).expect("message guid"),
        timestamp: MessageTimestamp::new(timestamp).expect("timestamp"),
        text: "Planning Friday at 3".to_owned(),
        tapback: None,
    }
}

fn participant(value: &str) -> ParticipantId {
    ParticipantId::parse(value).expect("participant id")
}
