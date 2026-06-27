use morrow_messages::{
    ChatGuid, DiscoveredChat, DiscoveredChatParts, MessageTimestamp, MessagesDiscoveryDataSource,
    MessagesDiscoveryReport, MessagesDiscoveryStatus, MessagesError, ParticipantId,
    WhitelistedChat,
};

#[test]
fn discovery_baseline_pins_existing_boundary_validation() {
    // Given: existing message-domain boundary values used by ingestion.
    let chat_guid = ChatGuid::parse("chat-valid").expect("chat guid");
    let first_participant = ParticipantId::parse("participant-local-a").expect("participant id");
    let second_participant = ParticipantId::parse("participant-local-b").expect("participant id");

    // When: existing constructors parse valid and invalid boundary input.
    let timestamp = MessageTimestamp::new(1_783_000_000).expect("timestamp");
    let whitelisted = WhitelistedChat::with_participants(
        chat_guid,
        2,
        vec![first_participant.clone(), second_participant],
    )
    .expect("whitelisted chat");

    // Then: valid values are retained and current invalid cases stay rejected.
    assert_eq!(whitelisted.participant_count, 2);
    assert_eq!(
        whitelisted.participant_ids[0].as_str(),
        "participant-local-a"
    );
    assert_eq!(timestamp.as_i64(), 1_783_000_000);
    assert!(ChatGuid::parse("").is_err());
    assert!(ParticipantId::parse("").is_err());
    assert!(MessageTimestamp::new(-1).is_err());
    assert!(WhitelistedChat::with_participants(
        ChatGuid::parse("chat-duplicate").expect("chat guid"),
        2,
        vec![first_participant.clone(), first_participant],
    )
    .is_err());
}

#[test]
fn discovery_valid_discovered_chats_parse_when_inputs_are_non_private() {
    // Given: a native discovery row with a non-private display label.
    let chat = valid_discovered_chat("chat-valid", "Team planning", 1_783_000_000)
        .expect("discovered chat");

    // When: the chat is placed in a ready discovery report.
    let report = MessagesDiscoveryReport::ready(vec![chat.clone()]).expect("report");

    // Then: the typed chat exposes only discovery metadata.
    assert_eq!(chat.chat_guid().as_str(), "chat-valid");
    assert_eq!(chat.display_label(), "Team planning");
    assert_eq!(chat.participant_count(), 2);
    assert_eq!(chat.participant_ids()[0].as_str(), "local-participant-a");
    assert_eq!(chat.latest_activity_timestamp().as_i64(), 1_783_000_000);
    assert_eq!(report.status(), MessagesDiscoveryStatus::Ready);
    assert_eq!(report.chats().len(), 1);
}

#[test]
fn discovery_label_falls_back_when_empty_or_raw_handle_shaped() {
    // Given: native labels that are blank or expose a raw handle.
    let empty = discovered_chat("chat-empty-label", "", 1_783_000_000).expect("empty label");
    let email =
        discovered_chat("chat-email-label", "person@example.com", 1_783_000_001).expect("email");
    let phone =
        discovered_chat("chat-phone-label", "+1 (555) 123-4567", 1_783_000_002).expect("phone");

    // When: labels are read from the typed discovery values.
    let labels = [
        empty.display_label(),
        email.display_label(),
        phone.display_label(),
    ];

    // Then: private-looking labels use the same generic non-private fallback.
    assert_eq!(labels, ["Messages chat", "Messages chat", "Messages chat"]);
}

#[test]
fn discovery_rejects_duplicate_participant_ids() {
    // Given: a discovery row with duplicated opaque participant IDs.
    let duplicate = DiscoveredChat::parse(DiscoveredChatParts {
        chat_guid: "chat-duplicate-participants",
        display_label: "Planning",
        participant_count: 2,
        participant_ids: vec!["local-participant-a", "local-participant-a"],
        latest_activity_timestamp: 1_783_000_000,
    });

    // When / Then: discovery rejects the row at the boundary.
    assert!(duplicate.is_err());
}

#[test]
fn discovery_rejects_raw_phone_or_email_participant_ids() {
    // Given: discovery rows whose participant identifiers expose raw handles.
    let email = DiscoveredChat::parse(DiscoveredChatParts {
        chat_guid: "chat-email-participant",
        display_label: "Planning",
        participant_count: 1,
        participant_ids: vec!["person@example.com"],
        latest_activity_timestamp: 1_783_000_000,
    });
    let phone = DiscoveredChat::parse(DiscoveredChatParts {
        chat_guid: "chat-phone-participant",
        display_label: "Planning",
        participant_count: 1,
        participant_ids: vec!["+15551234567"],
        latest_activity_timestamp: 1_783_000_000,
    });

    // When / Then: raw handles cannot cross into discovery metadata.
    assert!(email.is_err());
    assert!(phone.is_err());
}

#[test]
fn discovery_rejects_invalid_latest_activity_timestamps() {
    // Given: a discovery row with a negative latest activity timestamp.
    let invalid = discovered_chat("chat-invalid-time", "Planning", -1);

    // When / Then: timestamp validation is shared with message ingestion.
    assert!(invalid.is_err());
}

#[test]
fn discovery_reports_reject_invalid_status_chat_combinations() {
    // Given: ready and degraded report states.
    let chat =
        valid_discovered_chat("chat-status", "Planning", 1_783_000_000).expect("discovered chat");

    // When / Then: ready reports require chats, while degraded reports do not carry chats.
    assert!(MessagesDiscoveryReport::ready(Vec::new()).is_err());
    assert!(MessagesDiscoveryReport::new(MessagesDiscoveryStatus::Empty, vec![chat]).is_err());
    assert_eq!(
        MessagesDiscoveryReport::permission_denied().status(),
        MessagesDiscoveryStatus::PermissionDenied
    );
    assert_eq!(
        MessagesDiscoveryReport::unavailable().status(),
        MessagesDiscoveryStatus::Unavailable
    );
}

#[test]
fn discovery_report_contract_is_separate_from_recent_message_reads() {
    // Given: a source implementing only the discovery data-source trait.
    let report = MessagesDiscoveryReport::empty();
    let source = FakeDiscoverySource { report };

    // When: discovery is requested through the dedicated discovery method.
    let discovered = source.discover_chats().expect("discovery report");

    // Then: callers receive discovery metadata without invoking read_recent.
    assert_eq!(discovered.status(), MessagesDiscoveryStatus::Empty);
    assert!(discovered.chats().is_empty());
}

#[test]
fn discovery_reports_cannot_carry_message_text() {
    // Given: a valid discovery report built from the public contract.
    let report = MessagesDiscoveryReport::ready(vec![valid_discovered_chat(
        "chat-no-message-text",
        "Team planning",
        1_783_000_000,
    )
    .expect("discovered chat")])
    .expect("report");

    // When: the report is inspected through public accessors.
    let debug = format!("{report:#?}");

    // Then: there are no message-body, preview, prompt, or response fields to expose.
    assert!(!debug.contains("text:"));
    assert!(!debug.contains("body:"));
    assert!(!debug.contains("preview:"));
    assert!(!debug.contains("prompt:"));
    assert!(!debug.contains("response:"));
}

#[derive(Debug, Clone)]
struct FakeDiscoverySource {
    report: MessagesDiscoveryReport,
}

impl MessagesDiscoveryDataSource for FakeDiscoverySource {
    fn discover_chats(&self) -> Result<MessagesDiscoveryReport, MessagesError> {
        Ok(self.report.clone())
    }
}

fn valid_discovered_chat(
    chat_guid: &'static str,
    display_label: &'static str,
    latest_activity_timestamp: i64,
) -> Result<DiscoveredChat, MessagesError> {
    DiscoveredChat::parse(DiscoveredChatParts {
        chat_guid,
        display_label,
        participant_count: 2,
        participant_ids: vec!["local-participant-a", "local-participant-b"],
        latest_activity_timestamp,
    })
}

fn discovered_chat(
    chat_guid: &'static str,
    display_label: &'static str,
    latest_activity_timestamp: i64,
) -> Result<DiscoveredChat, MessagesError> {
    DiscoveredChat::parse(DiscoveredChatParts {
        chat_guid,
        display_label,
        participant_count: 1,
        participant_ids: vec!["local-participant-a"],
        latest_activity_timestamp,
    })
}
