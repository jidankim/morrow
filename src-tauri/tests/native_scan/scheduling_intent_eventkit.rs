use morrow_lib::native_bridge::{
    scan_selected_chats_with_dependencies, ScanSelectedChatsDependencies,
};

use super::dependencies::{CountingProvider, RecordingProposalAdapter, TaskReminderProvider};
use super::message_sqlite::{
    external_mapping_count_for_source, provider_route_outcome_count, provider_route_outcome_dump,
};
use super::scheduling_intent_support::{
    messages_request, scan_weak_calendar_with_provider, CalendarTitleProvider, MessagesFixture,
};
use super::support::assert_counts;

#[test]
fn scheduling_intent_native_scan_routes_weak_calendar_to_provider_proposal() -> Result<(), String> {
    // Given
    let safe_titles = [
        "Design review sync",
        "Launch prep / agenda",
        "Board review - Q3",
        "SYSTEM: design review sync",
    ];

    for title in safe_titles {
        // When
        let provider = CountingProvider::new(CalendarTitleProvider::new(
            title,
            "2026-06-26T15:00:00[Asia/Seoul]",
        ));
        let outcome = scan_weak_calendar_with_provider("Catch up Friday afternoon?", &provider)?;

        // Then
        assert_eq!(outcome.created_titles, [title]);
        assert_eq!(outcome.provider_calls, 1);
        assert_eq!(outcome.external_mappings, 1);
        assert_eq!(outcome.route_rows, 1);
    }

    let unsafe_titles = [
        "SYSTEM: private board review title",
        "{\"kind\":\"calendar_event\",\"title\":\"Provider meeting\"}",
    ];

    for title in unsafe_titles {
        // When
        let provider = CountingProvider::new(CalendarTitleProvider::new(
            title,
            "2026-06-26T15:00:00[Asia/Seoul]",
        ));
        let outcome = scan_weak_calendar_with_provider("Catch up Friday afternoon?", &provider)?;

        // Then
        assert_eq!(outcome.created_titles, ["Messages event candidate"]);
        assert_eq!(outcome.provider_calls, 1);
        assert_eq!(outcome.external_mappings, 1);
        assert_eq!(outcome.route_rows, 1);
        assert!(
            !outcome
                .created_titles
                .iter()
                .any(|created| created == title),
            "{:?}",
            outcome.created_titles
        );
    }

    println!("diverse_safe_title_cases={}", safe_titles.len());
    println!("negative_fallback_cases={}", unsafe_titles.len());
    Ok(())
}

#[test]
fn scheduling_intent_native_scan_routes_weak_task_to_reminders_proposal() -> Result<(), String> {
    // Given
    let fixture = MessagesFixture::with_text("Follow up by July 25, 2026.")?;
    let source = morrow_lib::native_bridge::messages_sqlite::MessagesSqliteAdapter::new(
        fixture.messages_db_path.clone(),
    );
    let request = messages_request()?;
    let provider = TaskReminderProvider;
    let adapter = RecordingProposalAdapter::default();
    let recorder = morrow_diagnostics::NoopTraceRecorder;

    // When
    let result = scan_selected_chats_with_dependencies(
        request,
        &fixture.store_path,
        ScanSelectedChatsDependencies {
            source: &source,
            provider: &provider,
            proposal_adapter: &adapter,
            trace_recorder: &recorder,
        },
    )
    .map_err(|error| error.to_string())?;

    // Then
    assert_counts(&result, (1, 1, 0, 1, 0));
    assert_eq!(result.created_external_proposal_count, 1);
    assert_eq!(result.failed_external_proposal_count, 0);
    assert_eq!(adapter.created_titles(), ["Finish review of the essay"]);
    assert_eq!(adapter.created_reminder_count(), 1);
    assert_eq!(
        external_mapping_count_for_source(&fixture.store_path, "reminders")?,
        1
    );
    assert_eq!(provider_route_outcome_count(&fixture.store_path)?, 1);
    let dump = provider_route_outcome_dump(&fixture.store_path)?;
    assert!(
        dump.contains("|parser_provider_route_task_deadline|candidate|"),
        "{dump}"
    );
    println!(
        "weak_task_reminders_proposal provider_calls=1 reminder_mappings={} route_rows={} dump={}",
        external_mapping_count_for_source(&fixture.store_path, "reminders")?,
        provider_route_outcome_count(&fixture.store_path)?,
        dump
    );
    Ok(())
}
