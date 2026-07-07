#[path = "list_intake/sqlite.rs"]
mod sqlite;
#[path = "list_intake/support.rs"]
mod support;

use std::sync::{Arc, Barrier};
use std::thread;

use morrow_storage::{ListIntakeAggregateQuery, ListIntakeConfidenceTier};

use sqlite::sqlite_rows;
use support::{extraction, fresh_store, item};

#[test]
fn sender_labels_are_stable_and_unknown_sender_uses_unknown_bucket() {
    // Given
    let (_dir, db_path, store) = fresh_store("list-intake-senders.sqlite");
    let first = extraction(
        "msg-list-intake-sender-1",
        ListIntakeConfidenceTier::AutoAggregate,
        vec![item("anchovies", 2)],
    );
    let mut second = extraction(
        "msg-list-intake-sender-2",
        ListIntakeConfidenceTier::AutoAggregate,
        vec![item("anchovies", 3)],
    );
    second.sender_key = None;
    let mut third = extraction(
        "msg-list-intake-sender-3",
        ListIntakeConfidenceTier::AutoAggregate,
        vec![item("anchovies", 4)],
    );
    third.sender_key = Some("sender-hash-bob".to_owned());

    // When
    store
        .record_list_intake_extraction(first.clone())
        .expect("record sender");
    store
        .record_list_intake_extraction(first)
        .expect("record duplicate sender upsert conflict");
    store
        .record_list_intake_extraction(third)
        .expect("record second sender");
    store
        .record_list_intake_extraction(second)
        .expect("record unknown sender");
    let aggregate = store
        .list_intake_aggregates(ListIntakeAggregateQuery {
            profile_id: Some("list-intake-fishcount".to_owned()),
            window_local_date: Some("2026-07-07".to_owned()),
            chat_key: Some("chat-hash-1".to_owned()),
            sender_label: None,
            category_id: Some("seafood".to_owned()),
        })
        .expect("aggregate senders");
    let sender_label_rows = sqlite_rows(
        &db_path,
        "SELECT sender_label, COUNT(*)
         FROM list_intake_sender_labels
         GROUP BY sender_label
         ORDER BY sender_label;",
    );

    // Then
    assert_eq!(aggregate.len(), 3);
    assert_eq!(aggregate[0].sender_label, "Sender 1");
    assert_eq!(aggregate[0].total_quantity, 2);
    assert_eq!(aggregate[1].sender_label, "Sender 2");
    assert_eq!(aggregate[1].total_quantity, 4);
    assert_eq!(aggregate[2].sender_label, "Unknown sender");
    assert_eq!(aggregate[2].total_quantity, 3);
    assert_eq!(
        sender_label_rows,
        vec![
            vec!["Sender 1".to_owned(), "1".to_owned()],
            vec!["Sender 2".to_owned(), "1".to_owned()],
            vec!["Unknown sender".to_owned(), "1".to_owned()]
        ]
    );
    println!(
        "list_intake_sender_label_stability sender_groups={} duplicate_upsert_conflict_stable=true unknown_bucket=true",
        aggregate.len()
    );
}

#[test]
fn sender_label_upsert_is_stable_when_same_sender_records_concurrently() {
    // Given
    let (_dir, db_path, store) = fresh_store("list-intake-concurrent-senders.sqlite");
    let barrier = Arc::new(Barrier::new(3));
    let first = extraction(
        "msg-list-intake-concurrent-sender-1",
        ListIntakeConfidenceTier::AutoAggregate,
        vec![item("anchovies", 2)],
    );
    let second = extraction(
        "msg-list-intake-concurrent-sender-2",
        ListIntakeConfidenceTier::AutoAggregate,
        vec![item("anchovies", 3)],
    );

    // When
    let first_store = store.clone();
    let first_barrier = Arc::clone(&barrier);
    let first_writer = thread::spawn(move || {
        first_barrier.wait();
        first_store
            .record_list_intake_extraction(first)
            .map(|receipt| (receipt.approved_entry_count, receipt.proposal_count))
            .map_err(|err| err.to_string())
    });
    let second_store = store.clone();
    let second_barrier = Arc::clone(&barrier);
    let second_writer = thread::spawn(move || {
        second_barrier.wait();
        second_store
            .record_list_intake_extraction(second)
            .map(|receipt| (receipt.approved_entry_count, receipt.proposal_count))
            .map_err(|err| err.to_string())
    });
    barrier.wait();
    let first_receipt = first_writer
        .join()
        .expect("join first concurrent sender writer")
        .expect("record first concurrent sender");
    let second_receipt = second_writer
        .join()
        .expect("join second concurrent sender writer")
        .expect("record second concurrent sender");
    let aggregate = store
        .list_intake_aggregates(ListIntakeAggregateQuery {
            profile_id: Some("list-intake-fishcount".to_owned()),
            window_local_date: Some("2026-07-07".to_owned()),
            chat_key: Some("chat-hash-1".to_owned()),
            sender_label: None,
            category_id: Some("seafood".to_owned()),
        })
        .expect("aggregate concurrent sender entries");
    let sender_label_rows = sqlite_rows(
        &db_path,
        "SELECT sender_label, COUNT(*)
         FROM list_intake_sender_labels
         GROUP BY sender_label
         ORDER BY sender_label;",
    );
    let entry_label_rows = sqlite_rows(
        &db_path,
        "SELECT sender_label, COUNT(*), SUM(quantity)
         FROM list_intake_entries
         GROUP BY sender_label
         ORDER BY sender_label;",
    );

    // Then
    assert_eq!(aggregate.len(), 1);
    assert_eq!(aggregate[0].sender_label, "Sender 1");
    assert_eq!(aggregate[0].total_quantity, 5);
    assert_eq!(aggregate[0].entry_count, 2);
    assert_eq!(
        sender_label_rows,
        vec![vec!["Sender 1".to_owned(), "1".to_owned()]]
    );
    assert_eq!(
        entry_label_rows,
        vec![vec!["Sender 1".to_owned(), "2".to_owned(), "5".to_owned()]]
    );
    println!(
        "list_intake_concurrent_sender_label_upsert stable_label={} label_rows={} entry_rows={} first_receipt={:?} second_receipt={:?}",
        aggregate[0].sender_label,
        sender_label_rows.len(),
        entry_label_rows.len(),
        first_receipt,
        second_receipt
    );
}
