use morrow_messages::TapbackKind;

use super::support::{
    assert_counts, batch, chat, fake_state, query_sqlite, raw_chat_with_participants, scan_request,
    temp_db,
};

#[test]
fn scan_persistence_applies_planned_candidate_and_feedback_once() -> Result<(), String> {
    // Given
    let (_dir, db_path) = temp_db("native-scan-persistence.sqlite")?;
    let messages = batch(vec![raw_chat_with_participants(
        "design-partners",
        "msg-design",
        3,
        &["p1", "p2", "p3"],
        "Let's meet 2026-07-15 14:00 at the private clinic.",
        Some(TapbackKind::Like),
    )?]);
    let request = scan_request(
        &[chat("design-partners", 3, &["p1", "p2", "p3"])],
        &[],
        true,
        0,
        0,
    )?;

    // When
    let result = fake_state(&db_path, messages)
        .scan_selected_chats_at(request, &db_path, &db_path)
        .map_err(|error| error.to_string())?;

    // Then
    assert_counts(&result, (1, 1, 0, 0, 1));
    assert_eq!(result.created_candidate_ids.len(), 1);
    assert_eq!(result.feedback_label_count, 1);
    assert_eq!(result.feature_snapshot_count, 1);
    let persisted_counts = query_sqlite(
        &db_path,
        "\
        SELECT (SELECT COUNT(*) FROM candidates) || '|' ||
               (SELECT COUNT(*) FROM feedback_events) || '|' ||
               (SELECT COUNT(*) FROM labels) || '|' ||
               (SELECT COUNT(*) FROM feature_snapshots);",
    )?;
    println!("persisted_counts={}", persisted_counts.trim());
    assert_eq!(persisted_counts.trim(), "1|1|1|1");
    Ok(())
}
