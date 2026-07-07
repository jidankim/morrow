use morrow_lib::native_bridge::{
    decide_list_intake_proposal_at, load_list_intake_review_at, ListIntakeDecisionRequest,
    ListIntakeReviewItem,
};
use morrow_storage::{
    ListIntakeConfidenceTier, ListIntakeExtractionDraft, ListIntakeItemDraft,
    ListIntakeLocalDayWindow, Store,
};

#[test]
fn list_intake_review_load_maps_store_rows_without_private_identifiers() -> Result<(), String> {
    // Given
    let (_dir, store_path, store) = fresh_store("native-list-intake-review-load.sqlite")?;
    store
        .record_list_intake_extraction(extraction(
            "raw-message-guid-load",
            ListIntakeConfidenceTier::AutoAggregate,
            vec![item("anchovies", 2), item("salmon", 3)],
        ))
        .map_err(|error| error.to_string())?;
    store
        .record_list_intake_extraction(extraction(
            "raw-message-guid-review",
            ListIntakeConfidenceTier::Review,
            vec![item("tuna", 4)],
        ))
        .map_err(|error| error.to_string())?;

    // When
    let report = load_list_intake_review_at(&store_path, 1_783_450_200)?;
    let serialized = serde_json::to_string(&report).map_err(|error| error.to_string())?;

    // Then
    assert_eq!(report.generated_at_unix_seconds, 1_783_450_200);
    assert_eq!(report.aggregates.len(), 1);
    assert_eq!(report.aggregates[0].local_date, "2026-07-07");
    assert_eq!(report.aggregates[0].sender_label, "Sender 1");
    assert_eq!(report.aggregates[0].category_label, "Seafood");
    assert_eq!(report.aggregates[0].items.len(), 2);
    assert_eq!(report.proposals.len(), 1);
    assert_eq!(report.proposals[0].sender_label, "Sender 1");
    assert_eq!(report.proposals[0].items[0].item_name, "tuna");
    assert!(!serialized.contains("raw-message-guid"));
    assert!(!serialized.contains("senderKey-raw-alice"));
    assert!(!serialized.contains("chat-key-private"));
    println!(
        "native_list_intake_review_load aggregates={} proposals={} leaked_private_tokens=false",
        report.aggregates.len(),
        report.proposals.len()
    );
    Ok(())
}

#[test]
fn list_intake_review_decisions_refresh_report_state() -> Result<(), String> {
    // Given
    let (_dir, store_path, store) = fresh_store("native-list-intake-review-decisions.sqlite")?;
    store
        .record_list_intake_extraction(extraction(
            "raw-message-guid-approve",
            ListIntakeConfidenceTier::Review,
            vec![item("anchovies", 2)],
        ))
        .map_err(|error| error.to_string())?;
    let pending = load_list_intake_review_at(&store_path, 1_783_450_200)?;
    let proposal_id = pending.proposals[0].proposal_id.clone();

    // When
    let approved = decide_list_intake_proposal_at(
        &store_path,
        ListIntakeDecisionRequest::ApproveEdited {
            proposal_id,
            items: vec![ListIntakeReviewItem {
                item_name: "anchovies".to_owned(),
                quantity: 5,
                unit: None,
                category_id: "seafood".to_owned(),
                category_label: "Seafood".to_owned(),
            }],
        },
        1_783_450_201,
        1_783_450_202,
    )?;

    // Then
    assert!(approved.proposals.is_empty());
    assert_eq!(approved.aggregates.len(), 1);
    assert_eq!(approved.aggregates[0].items[0].quantity, 5);

    // Given
    store
        .record_list_intake_extraction(extraction(
            "raw-message-guid-reject",
            ListIntakeConfidenceTier::Review,
            vec![item("salmon", 3)],
        ))
        .map_err(|error| error.to_string())?;
    let pending_reject = load_list_intake_review_at(&store_path, 1_783_450_203)?;
    let reject_id = pending_reject.proposals[0].proposal_id.clone();

    // When
    let rejected = decide_list_intake_proposal_at(
        &store_path,
        ListIntakeDecisionRequest::Reject {
            proposal_id: reject_id,
        },
        1_783_450_204,
        1_783_450_205,
    )?;

    // Then
    assert!(rejected.proposals.is_empty());
    assert_eq!(rejected.aggregates.len(), 1);
    assert_eq!(rejected.aggregates[0].items[0].quantity, 5);
    println!(
        "native_list_intake_review_decisions approve_refreshed=true reject_refreshed=true aggregates={}",
        rejected.aggregates.len()
    );
    Ok(())
}

#[test]
fn list_intake_review_approval_rejects_category_not_stored_on_proposal() -> Result<(), String> {
    // Given
    let (_dir, store_path, store) =
        fresh_store("native-list-intake-review-category-forgery.sqlite")?;
    store
        .record_list_intake_extraction(extraction(
            "raw-message-guid-forged-category",
            ListIntakeConfidenceTier::Review,
            vec![item("anchovies", 2)],
        ))
        .map_err(|error| error.to_string())?;
    let pending = load_list_intake_review_at(&store_path, 1_783_450_200)?;
    let proposal_id = pending.proposals[0].proposal_id.clone();

    // When
    let result = decide_list_intake_proposal_at(
        &store_path,
        ListIntakeDecisionRequest::ApproveEdited {
            proposal_id: proposal_id.clone(),
            items: vec![ListIntakeReviewItem {
                item_name: "anchovies".to_owned(),
                quantity: 2,
                unit: None,
                category_id: "forged-category".to_owned(),
                category_label: "Forged category".to_owned(),
            }],
        },
        1_783_450_201,
        1_783_450_202,
    );
    let still_pending = load_list_intake_review_at(&store_path, 1_783_450_203)?;

    // Then
    assert!(result.is_err());
    assert_eq!(still_pending.proposals.len(), 1);
    assert_eq!(still_pending.proposals[0].proposal_id, proposal_id);
    assert!(still_pending.aggregates.is_empty());
    println!("native_list_intake_review_category_forgery rejected=true proposal_pending=true");
    Ok(())
}

fn fresh_store(name: &str) -> Result<(tempfile::TempDir, std::path::PathBuf, Store), String> {
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let db_path = dir.path().join(name);
    let store = Store::open(&db_path).map_err(|error| error.to_string())?;
    Ok((dir, db_path, store))
}

fn extraction(
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
        chat_key: "chat-key-private".to_owned(),
        sender_key: Some("senderKey-raw-alice".to_owned()),
        window: ListIntakeLocalDayWindow {
            window_local_date: "2026-07-07".to_owned(),
            window_timezone: "Asia/Seoul".to_owned(),
            window_start_unix_seconds: 1_783_440_000,
        },
        confidence_tier,
        confidence_millis: 920,
        items,
        observed_at: 1_783_450_000,
    }
}

fn item(name: &str, quantity: i64) -> ListIntakeItemDraft {
    ListIntakeItemDraft {
        item_name: name.to_owned(),
        quantity,
        unit: None,
        category_id: "seafood".to_owned(),
    }
}
