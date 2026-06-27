use std::path::Path;

use morrow_lib::native_bridge::{FakeNativeBridge, NativeBridgeState};
use morrow_messages::{
    DiscoveredChat, DiscoveredChatParts, MessagesDiscoveryReport, MessagesDiscoveryStatus,
};

#[test]
fn discover_messages_chats_returns_fake_discovered_chats() -> Result<(), String> {
    // Given
    let state = NativeBridgeState::with_bridge(
        FakeNativeBridge::default().with_discovery_report(
            MessagesDiscoveryReport::ready(vec![
                synthetic_chat("fake-chat-a", "Design Partners", 1_783_000_100)?,
                synthetic_chat("fake-chat-b", "Ops Triage", 1_783_000_200)?,
            ])
            .map_err(|error| error.to_string())?,
        ),
    );

    // When
    let report = state
        .discover_messages_chats_at(Path::new("/private/var/should-not-be-used/chat.db"))
        .map_err(|error| error.to_string())?;

    // Then
    assert_eq!(report.status(), MessagesDiscoveryStatus::Ready);
    assert_eq!(report.chats().len(), 2);
    assert_eq!(report.chats()[0].chat_guid().as_str(), "fake-chat-a");
    assert_eq!(report.chats()[1].display_label(), "Ops Triage");
    Ok(())
}

fn synthetic_chat(
    chat_guid: &str,
    display_label: &str,
    latest_activity_timestamp: i64,
) -> Result<DiscoveredChat, String> {
    DiscoveredChat::parse(DiscoveredChatParts {
        chat_guid,
        display_label,
        participant_count: 1,
        participant_ids: vec!["messages-participant-11111111111111111111111111111111"],
        latest_activity_timestamp,
    })
    .map_err(|error| error.to_string())
}
