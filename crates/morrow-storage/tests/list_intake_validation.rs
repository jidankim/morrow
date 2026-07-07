#[path = "list_intake/support.rs"]
mod support;

use morrow_storage::{
    assign_list_intake_category, ListIntakeCategoryRule, ListIntakeConfidenceTier,
    LIST_INTAKE_UNCATEGORIZED_CATEGORY_ID,
};

use support::{extraction, fresh_store, item, item_in_category};

#[test]
fn category_keywords_assign_whole_words_and_fall_back_to_uncategorized() {
    // Given
    let rules = vec![
        ListIntakeCategoryRule {
            category_id: "seafood".to_owned(),
            keywords: vec!["anchovy".to_owned(), "fish sauce".to_owned()],
        },
        ListIntakeCategoryRule {
            category_id: "produce".to_owned(),
            keywords: vec!["apple".to_owned()],
        },
    ];

    // When
    let anchovies = assign_list_intake_category("2 anchovy tins", &rules);
    let fish_sauce = assign_list_intake_category("thai fish sauce", &rules);
    let cowfish = assign_list_intake_category("cowfish candy", &rules);
    let unmatched = assign_list_intake_category("paper towels", &rules);

    // Then
    assert_eq!(anchovies, "seafood");
    assert_eq!(fish_sauce, "seafood");
    assert_eq!(cowfish, LIST_INTAKE_UNCATEGORIZED_CATEGORY_ID);
    assert_eq!(unmatched, LIST_INTAKE_UNCATEGORIZED_CATEGORY_ID);
    println!(
        "list_intake_category_assignment whole_word=true phrase=true fallback={}",
        LIST_INTAKE_UNCATEGORIZED_CATEGORY_ID
    );
}

#[test]
fn malformed_list_intake_payloads_and_private_sender_tokens_are_rejected() {
    // Given
    let (_dir, _db_path, store) = fresh_store("list-intake-malformed.sqlite");
    let mut empty_items = extraction(
        "msg-list-intake-empty",
        ListIntakeConfidenceTier::AutoAggregate,
        Vec::new(),
    );
    empty_items.sender_key = Some("sender-hash-safe".to_owned());
    let mut raw_sender = extraction(
        "msg-list-intake-raw-sender",
        ListIntakeConfidenceTier::AutoAggregate,
        vec![item("anchovies", 2)],
    );
    raw_sender.sender_key = Some("private@example.com".to_owned());
    let bad_quantity = extraction(
        "msg-list-intake-bad-quantity",
        ListIntakeConfidenceTier::AutoAggregate,
        vec![item("anchovies", 0)],
    );
    let bad_category = extraction(
        "msg-list-intake-bad-category",
        ListIntakeConfidenceTier::AutoAggregate,
        vec![item_in_category("anchovies", 2, "raw category token")],
    );

    // When
    let empty_err = store
        .record_list_intake_extraction(empty_items)
        .expect_err("empty payload rejected");
    let sender_err = store
        .record_list_intake_extraction(raw_sender)
        .expect_err("raw sender rejected");
    let quantity_err = store
        .record_list_intake_extraction(bad_quantity)
        .expect_err("bad quantity rejected");
    let category_err = store
        .record_list_intake_extraction(bad_category)
        .expect_err("bad category rejected");

    // Then
    assert!(empty_err.to_string().contains("items"));
    assert!(sender_err.to_string().contains("sender_key"));
    assert!(quantity_err.to_string().contains("quantity"));
    assert!(category_err.to_string().contains("category_id"));
    println!(
        "list_intake_malformed_payloads rejected_empty=true rejected_raw_sender=true rejected_quantity=true rejected_category=true"
    );
}
