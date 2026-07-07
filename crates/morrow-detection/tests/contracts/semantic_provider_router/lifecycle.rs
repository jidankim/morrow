use super::support::assert_provider_rejection;
use super::TestResult;

#[test]
fn update_014_routes_lifecycle_but_rejects_creation() -> TestResult {
    for (anchor, reason) in [
        ("other-message", "provider_hallucinated_evidence"),
        ("msg-update-014", "provider_unsupported_lifecycle"),
    ] {
        let response = format!(
            r#"{{"kind":"event_reschedule","title":"Client call",
             "confidence_millis":900,
             "normalized_time":"2026-07-08T10:00:00[Asia/Seoul]",
             "anchor_message_guid":"{anchor}",
             "evidence_message_guids":["{anchor}"]}}"#,
        );
        assert_provider_rejection(
            "msg-update-014",
            "The client call moved from Tuesday to Wednesday.",
            &response,
            reason,
        )?;
    }
    Ok(())
}

#[test]
fn update_like_text_rejects_provider_calendar_event_mislabel() -> TestResult {
    let response = r#"{"kind":"calendar_event","title":"Client call",
        "confidence_millis":900,
        "normalized_time":"2026-07-08T10:00:00[Asia/Seoul]",
        "anchor_message_guid":"msg-update-mislabel-calendar",
        "evidence_message_guids":["msg-update-mislabel-calendar"]}"#;

    assert_provider_rejection(
        "msg-update-mislabel-calendar",
        "The client call moved from Tuesday to Wednesday.",
        response,
        "provider_unsupported_lifecycle",
    )
}

#[test]
fn update_like_text_rejects_provider_task_reminder_mislabel() -> TestResult {
    let response = r#"{"kind":"task_reminder","title":"Client call",
        "confidence_millis":820,
        "normalized_time":"2026-07-08T09:00:00[Asia/Seoul]",
        "anchor_message_guid":"msg-update-mislabel-reminder",
        "evidence_message_guids":["msg-update-mislabel-reminder"]}"#;

    assert_provider_rejection(
        "msg-update-mislabel-reminder",
        "The client call moved from Tuesday to Wednesday.",
        response,
        "provider_unsupported_lifecycle",
    )
}
