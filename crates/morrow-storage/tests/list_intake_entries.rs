#[path = "list_intake/sqlite.rs"]
mod sqlite;
#[path = "list_intake/support.rs"]
mod support;

use morrow_storage::{ListIntakeAggregateQuery, ListIntakeConfidenceTier};

use sqlite::sqlite_rows;
use support::{extraction, fresh_store, item};

#[test]
fn approved_list_intake_entries_aggregate_idempotently() {
    // Given
    let (_dir, db_path, store) = fresh_store("list-intake-approved.sqlite");
    let auto = extraction(
        "msg-list-intake-approved-1",
        ListIntakeConfidenceTier::AutoAggregate,
        vec![item("anchovies", 2), item("anchovies", 3)],
    );

    // When
    let first = store
        .record_list_intake_extraction(auto.clone())
        .expect("record auto aggregate");
    let duplicate = store
        .record_list_intake_extraction(auto)
        .expect("record duplicate auto aggregate");
    let aggregate = store
        .list_intake_aggregates(ListIntakeAggregateQuery {
            profile_id: Some("list-intake-fishcount".to_owned()),
            window_local_date: Some("2026-07-07".to_owned()),
            chat_key: Some("chat-hash-1".to_owned()),
            sender_label: None,
            category_id: Some("seafood".to_owned()),
        })
        .expect("aggregate list intake");
    let raw_sender_tokens = sqlite_rows(
        &db_path,
        "SELECT COUNT(*)
         FROM list_intake_entries
         WHERE sender_label LIKE '%@%'
            OR sender_label LIKE '%+1555%'
            OR sender_key LIKE '%@%';",
    );

    // Then
    assert_eq!(first.approved_entry_count, 2);
    assert_eq!(first.proposal_count, 0);
    assert_eq!(duplicate.approved_entry_count, 0);
    assert_eq!(duplicate.proposal_count, 0);
    assert_eq!(aggregate.len(), 1);
    assert_eq!(aggregate[0].item_name, "anchovies");
    assert_eq!(aggregate[0].total_quantity, 5);
    assert_eq!(aggregate[0].entry_count, 2);
    assert_eq!(aggregate[0].sender_label, "Sender 1");
    assert_eq!(raw_sender_tokens, vec![vec!["0".to_owned()]]);
    println!(
        "aggregate_2_plus_3_anchovies_equals_5 approved_rows={} duplicate_rows={} aggregate_items={} raw_sender_tokens=0",
        first.approved_entry_count,
        duplicate.approved_entry_count,
        aggregate.len()
    );
}

#[test]
fn repeated_auto_aggregate_extraction_is_noop_when_provider_output_changes() {
    // Given
    let (_dir, _db_path, store) = fresh_store("list-intake-auto-output-change.sqlite");
    let first = extraction(
        "msg-list-intake-auto-output-change",
        ListIntakeConfidenceTier::AutoAggregate,
        vec![item("anchovies", 2), item("salmon", 3)],
    );
    let changed = extraction(
        "msg-list-intake-auto-output-change",
        ListIntakeConfidenceTier::AutoAggregate,
        vec![item("anchovies", 7), item("salmon", 3)],
    );
    let added = extraction(
        "msg-list-intake-auto-output-change",
        ListIntakeConfidenceTier::AutoAggregate,
        vec![item("anchovies", 2), item("salmon", 3), item("tuna", 4)],
    );
    let removed = extraction(
        "msg-list-intake-auto-output-change",
        ListIntakeConfidenceTier::AutoAggregate,
        vec![item("anchovies", 2)],
    );

    // When
    let first_receipt = store
        .record_list_intake_extraction(first)
        .expect("record first auto aggregate");
    let changed_receipt = store
        .record_list_intake_extraction(changed)
        .expect("changed provider output is a no-op");
    let added_receipt = store
        .record_list_intake_extraction(added)
        .expect("added provider output is a no-op");
    let removed_receipt = store
        .record_list_intake_extraction(removed)
        .expect("removed provider output is a no-op");
    let aggregate = store
        .list_intake_aggregates(ListIntakeAggregateQuery {
            profile_id: Some("list-intake-fishcount".to_owned()),
            window_local_date: Some("2026-07-07".to_owned()),
            chat_key: Some("chat-hash-1".to_owned()),
            sender_label: None,
            category_id: Some("seafood".to_owned()),
        })
        .expect("aggregate original auto output");

    // Then
    assert_eq!(first_receipt.approved_entry_count, 2);
    assert_eq!(changed_receipt.approved_entry_count, 0);
    assert_eq!(added_receipt.approved_entry_count, 0);
    assert_eq!(removed_receipt.approved_entry_count, 0);
    assert_eq!(aggregate.len(), 2);
    assert_eq!(aggregate[0].item_name, "anchovies");
    assert_eq!(aggregate[0].total_quantity, 2);
    assert_eq!(aggregate[1].item_name, "salmon");
    assert_eq!(aggregate[1].total_quantity, 3);
    println!(
        "list_intake_auto_repeated_output_mutations_noop first_rows={} changed_rows={} added_rows={} removed_rows={} aggregate_rows={}",
        first_receipt.approved_entry_count,
        changed_receipt.approved_entry_count,
        added_receipt.approved_entry_count,
        removed_receipt.approved_entry_count,
        aggregate.len()
    );
}
