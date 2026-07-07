use std::path::PathBuf;

use morrow_storage::{
    ListIntakeConfidenceTier, ListIntakeExtractionDraft, ListIntakeItemDraft,
    ListIntakeLocalDayWindow, Store,
};

pub fn fresh_store(name: &str) -> (tempfile::TempDir, PathBuf, Store) {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join(name);
    let store = Store::open(&db_path).expect("open store");
    (dir, db_path, store)
}

pub fn window() -> ListIntakeLocalDayWindow {
    ListIntakeLocalDayWindow {
        window_local_date: "2026-07-07".to_owned(),
        window_timezone: "Asia/Seoul".to_owned(),
        window_start_unix_seconds: 1_783_440_000,
    }
}

pub fn item(name: &str, quantity: i64) -> ListIntakeItemDraft {
    item_in_category(name, quantity, "seafood")
}

pub fn item_in_category(name: &str, quantity: i64, category_id: &str) -> ListIntakeItemDraft {
    ListIntakeItemDraft {
        item_name: name.to_owned(),
        quantity,
        unit: None,
        category_id: category_id.to_owned(),
    }
}

pub fn extraction(
    message_guid: &str,
    confidence_tier: ListIntakeConfidenceTier,
    items: Vec<ListIntakeItemDraft>,
) -> ListIntakeExtractionDraft {
    ListIntakeExtractionDraft {
        profile_id: "list-intake-fishcount".to_owned(),
        profile_version: "list-intake-v2".to_owned(),
        examples_hash: "sha256:examples".to_owned(),
        message_guid: message_guid.to_owned(),
        evidence_pointer: format!("evidence:{message_guid}"),
        chat_key: "chat-hash-1".to_owned(),
        sender_key: Some("sender-hash-alice".to_owned()),
        window: window(),
        confidence_tier,
        confidence_millis: 920,
        items,
        observed_at: 1_783_450_000,
    }
}
