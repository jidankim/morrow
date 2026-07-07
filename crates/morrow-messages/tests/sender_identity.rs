use std::{cell::Cell, collections::BTreeMap};

use morrow_messages::{
    ingest_selected_threads, BackfillDays, ChatGuid, IngestionReport, IngestionRequest,
    MessageGuid, MessageSenderIdentity, MessageTimestamp, MessagesDataSource, MessagesError,
    NativeBatch, NativeReadRequest, RawChat, RawMessage, SenderKey, WhitelistedChat,
};

#[derive(Debug)]
struct SenderSource {
    batch: NativeBatch,
    calls: Cell<usize>,
}

impl MessagesDataSource for SenderSource {
    fn read_recent(&self, _request: &NativeReadRequest) -> Result<NativeBatch, MessagesError> {
        self.calls.set(self.calls.get() + 1);
        Ok(self.batch.clone())
    }

    fn sender_identity(
        &self,
        _chat_guid: &ChatGuid,
        message_guid: &MessageGuid,
    ) -> MessageSenderIdentity {
        match message_guid.as_str() {
            "msg-a" | "msg-a-again" => MessageSenderIdentity::known(sender_key("senderKey-a")),
            "msg-b" => MessageSenderIdentity::known(sender_key("senderKey-b")),
            "msg-null" => MessageSenderIdentity::unknown(),
            _ => MessageSenderIdentity::unknown(),
        }
    }
}

#[test]
fn sender_display_aliases_are_stable_and_unknown_is_one_bucket() {
    // Given: one selected chat has two known senders and one message with no sender.
    let chat = ChatGuid::parse("chat-alpha").expect("chat guid");
    let source = SenderSource {
        batch: NativeBatch {
            chats: vec![RawChat {
                guid: chat.clone(),
                participant_count: 2,
                participant_ids: Vec::new(),
                messages: vec![
                    message(&chat, "msg-a", "1 apples"),
                    message(&chat, "msg-b", "2 bananas"),
                    message(&chat, "msg-null", "3 carrots"),
                    message(&chat, "msg-a-again", "4 dates"),
                ],
            }],
        },
        calls: Cell::new(0),
    };
    let request = IngestionRequest::new(
        vec![WhitelistedChat::new(chat, 2).expect("whitelist")],
        BackfillDays::new(7).expect("backfill"),
        MessageTimestamp::new(1_783_000_200).expect("now"),
    )
    .expect("request");

    // When: ingestion runs twice over the same sender identities.
    let first = ingest_selected_threads(&source, &request).expect("first ingest");
    let second = ingest_selected_threads(&source, &request).expect("second ingest");

    // Then: only display labels leave the ingestion report boundary, and the labels are stable.
    let first_labels = labels_by_message(&first);
    let second_labels = labels_by_message(&second);
    assert_eq!(first_labels, second_labels);
    assert_eq!(first_labels.get("msg-a"), Some(&"Sender 1".to_owned()));
    assert_eq!(
        first_labels.get("msg-a-again"),
        Some(&"Sender 1".to_owned())
    );
    assert_eq!(first_labels.get("msg-b"), Some(&"Sender 2".to_owned()));
    assert_eq!(
        first_labels.get("msg-null"),
        Some(&"Unknown sender".to_owned())
    );
    assert!(!format!("{first:?}").contains("senderKey-a"));
}

fn sender_key(value: &str) -> SenderKey {
    SenderKey::from_private_digest(value).expect("sender key")
}

fn message(chat_guid: &ChatGuid, message_guid: &str, text: &str) -> RawMessage {
    RawMessage {
        chat_guid: chat_guid.clone(),
        message_guid: MessageGuid::parse(message_guid).expect("message guid"),
        timestamp: MessageTimestamp::new(1_783_000_190).expect("timestamp"),
        text: text.to_owned(),
        tapback: None,
    }
}

fn labels_by_message(report: &IngestionReport) -> BTreeMap<String, String> {
    report
        .sender_groups
        .iter()
        .map(|group| {
            (
                group.message_guid.as_str().to_owned(),
                group.display_label.as_str().to_owned(),
            )
        })
        .collect()
}
