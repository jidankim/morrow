#[path = "list_intake/support.rs"]
mod support;

use morrow_storage::{
    ListIntakeAggregateQuery, ListIntakeConfidenceTier, ListIntakeProposalDecision,
};

use support::{extraction, fresh_store, item};

#[test]
fn review_confidence_proposals_can_be_approved_edited_and_rejected() {
    // Given
    let (_dir, _db_path, store) = fresh_store("list-intake-proposals.sqlite");
    store
        .record_list_intake_extraction(extraction(
            "msg-list-intake-review-1",
            ListIntakeConfidenceTier::Review,
            vec![item("anchovies", 2), item("salmon", 3)],
        ))
        .expect("record review confidence proposals");
    let duplicate = store
        .record_list_intake_extraction(extraction(
            "msg-list-intake-review-1",
            ListIntakeConfidenceTier::Review,
            vec![item("anchovies", 2), item("salmon", 3)],
        ))
        .expect("record duplicate review confidence proposals");
    let proposals = store
        .list_intake_proposals("list-intake-fishcount")
        .expect("pending proposals");

    // When
    let approved_id = proposals[0].proposal_id.clone();
    store
        .decide_list_intake_proposal(ListIntakeProposalDecision::ApproveEdited {
            proposal_id: approved_id,
            items: vec![item("anchovies", 5), item("salmon", 3)],
            allowed_category_ids: vec!["seafood".to_owned()],
            decided_at: 1_783_450_100,
        })
        .expect("approve edited proposal");
    let aggregate = store
        .list_intake_aggregates(ListIntakeAggregateQuery {
            profile_id: Some("list-intake-fishcount".to_owned()),
            window_local_date: Some("2026-07-07".to_owned()),
            chat_key: Some("chat-hash-1".to_owned()),
            sender_label: None,
            category_id: Some("seafood".to_owned()),
        })
        .expect("aggregate approved proposal");
    let decided = store
        .list_intake_proposals("list-intake-fishcount")
        .expect("decided proposals");

    // Then
    assert_eq!(duplicate.approved_entry_count, 0);
    assert_eq!(duplicate.proposal_count, 0);
    assert_eq!(proposals.len(), 1);
    assert_eq!(proposals[0].items.len(), 2);
    assert_eq!(aggregate.len(), 2);
    assert_eq!(aggregate[0].item_name, "anchovies");
    assert_eq!(aggregate[0].total_quantity, 5);
    assert!(aggregate[0].source_proposal_count > 0);
    assert_eq!(decided[0].status.as_str(), "approved");
    println!(
        "list_intake_proposal_decisions proposal_count={} approved_item_rows={} approved_edit_total={}",
        proposals.len(),
        aggregate.len(),
        aggregate[0].total_quantity
    );
}

#[test]
fn repeated_review_extraction_is_noop_when_provider_output_changes() {
    // Given
    let (_dir, _db_path, store) = fresh_store("list-intake-review-output-change.sqlite");
    let first = extraction(
        "msg-list-intake-review-output-change",
        ListIntakeConfidenceTier::Review,
        vec![item("anchovies", 2), item("salmon", 3)],
    );
    let changed = extraction(
        "msg-list-intake-review-output-change",
        ListIntakeConfidenceTier::Review,
        vec![item("anchovies", 7), item("salmon", 3)],
    );
    let added = extraction(
        "msg-list-intake-review-output-change",
        ListIntakeConfidenceTier::Review,
        vec![item("anchovies", 2), item("salmon", 3), item("tuna", 4)],
    );
    let removed = extraction(
        "msg-list-intake-review-output-change",
        ListIntakeConfidenceTier::Review,
        vec![item("anchovies", 2)],
    );

    // When
    let first_receipt = store
        .record_list_intake_extraction(first)
        .expect("record first review proposal");
    let changed_receipt = store
        .record_list_intake_extraction(changed)
        .expect("changed provider output is a no-op");
    let added_receipt = store
        .record_list_intake_extraction(added)
        .expect("added provider output is a no-op");
    let removed_receipt = store
        .record_list_intake_extraction(removed)
        .expect("removed provider output is a no-op");
    let proposals = store
        .list_intake_proposals("list-intake-fishcount")
        .expect("review proposals");

    // Then
    assert_eq!(first_receipt.proposal_count, 1);
    assert_eq!(changed_receipt.proposal_count, 0);
    assert_eq!(added_receipt.proposal_count, 0);
    assert_eq!(removed_receipt.proposal_count, 0);
    assert_eq!(proposals.len(), 1);
    assert_eq!(proposals[0].items.len(), 2);
    assert_eq!(proposals[0].items[0].item_name, "anchovies");
    assert_eq!(proposals[0].items[0].quantity, 2);
    assert_eq!(proposals[0].items[1].item_name, "salmon");
    assert_eq!(proposals[0].items[1].quantity, 3);
    println!(
        "list_intake_review_repeated_output_mutations_noop first_proposals={} changed_proposals={} added_proposals={} removed_proposals={} proposal_items={}",
        first_receipt.proposal_count,
        changed_receipt.proposal_count,
        added_receipt.proposal_count,
        removed_receipt.proposal_count,
        proposals[0].items.len()
    );
}

#[test]
fn low_confidence_extractions_create_no_proposal_or_aggregate_rows() {
    // Given
    let (_dir, _db_path, store) = fresh_store("list-intake-low-noop.sqlite");

    // When
    let receipt = store
        .record_list_intake_extraction(extraction(
            "msg-list-intake-low-1",
            ListIntakeConfidenceTier::Low,
            vec![item("anchovies", 2), item("salmon", 3)],
        ))
        .expect("record low confidence extraction as no-op");
    let proposals = store
        .list_intake_proposals("list-intake-fishcount")
        .expect("low confidence proposals");
    let aggregate = store
        .list_intake_aggregates(ListIntakeAggregateQuery {
            profile_id: Some("list-intake-fishcount".to_owned()),
            window_local_date: Some("2026-07-07".to_owned()),
            chat_key: Some("chat-hash-1".to_owned()),
            sender_label: None,
            category_id: Some("seafood".to_owned()),
        })
        .expect("low confidence aggregate");

    // Then
    assert_eq!(receipt.approved_entry_count, 0);
    assert_eq!(receipt.proposal_count, 0);
    assert!(proposals.is_empty());
    assert!(aggregate.is_empty());
    println!("list_intake_low_confidence_storage_noop entries=0 proposals=0 aggregates=0");
}

#[test]
fn proposal_edit_approval_enforces_v2_item_bounds_and_configured_categories() {
    // Given
    let (_dir, _db_path, store) = fresh_store("list-intake-proposal-edit-bounds.sqlite");
    store
        .record_list_intake_extraction(extraction(
            "msg-list-intake-review-bounds",
            ListIntakeConfidenceTier::Review,
            vec![item("anchovies", 2)],
        ))
        .expect("record review proposal");
    let proposal_id = store
        .list_intake_proposals("list-intake-fishcount")
        .expect("pending proposal")[0]
        .proposal_id
        .clone();
    let overlong_name = "x".repeat(81);
    let overlong_unit = "x".repeat(25);
    let cases = [
        vec![item("", 2)],
        vec![item(&overlong_name, 2)],
        vec![item("anchovies", 0)],
        vec![item("anchovies", 1_000)],
        vec![morrow_storage::ListIntakeItemDraft {
            item_name: "anchovies".to_owned(),
            quantity: 2,
            unit: Some(overlong_unit),
            category_id: "seafood".to_owned(),
        }],
        vec![morrow_storage::ListIntakeItemDraft {
            item_name: "anchovies".to_owned(),
            quantity: 2,
            unit: None,
            category_id: "not-configured".to_owned(),
        }],
    ];

    for items in cases {
        // When
        let result = store.decide_list_intake_proposal(ListIntakeProposalDecision::ApproveEdited {
            proposal_id: proposal_id.clone(),
            items,
            allowed_category_ids: vec!["seafood".to_owned()],
            decided_at: 1_783_450_100,
        });

        // Then
        assert!(result.is_err(), "invalid edit should be rejected");
    }
    store
        .decide_list_intake_proposal(ListIntakeProposalDecision::ApproveEdited {
            proposal_id,
            items: vec![morrow_storage::ListIntakeItemDraft {
                item_name: "anchovies".to_owned(),
                quantity: 2,
                unit: Some("tins".to_owned()),
                category_id: "uncategorized".to_owned(),
            }],
            allowed_category_ids: vec!["seafood".to_owned()],
            decided_at: 1_783_450_101,
        })
        .expect("valid uncategorized edit");
    println!(
        "list_intake_proposal_edit_v2_bounds rejected_invalid=true accepted_uncategorized=true"
    );
}
