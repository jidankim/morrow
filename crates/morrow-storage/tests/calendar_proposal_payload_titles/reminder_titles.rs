use super::calendar_proposal_payload_titles_support::stored_reminder_title;

#[test]
fn reminder_proposal_payload_preserves_safe_titles_and_redacts_private_markers() {
    // Given
    let safe_title = "Finish review of the essay";
    let private_cases = [
        "raw-reminder plan",
        "private reminder",
        "Reminder ops@example.com",
        "Reminder 555.111.2222",
        "Reminder (555) 111 2222",
        "{\"kind\":\"task_reminder\",\"title\":\"Provider reminder\"}",
    ];

    assert_eq!(
        stored_reminder_title("reminder-payload-safe-title.sqlite", safe_title),
        safe_title
    );
    for (index, title) in private_cases.iter().enumerate() {
        assert_eq!(
            stored_reminder_title(
                &format!("reminder-payload-private-title-{index}.sqlite"),
                title
            ),
            "Messages reminder candidate"
        );
    }
}
