use super::dependencies::{CountingProvider, UnavailableTestProvider};
use super::provider_route_support::assert_provider_route_dump_hides;
use super::scheduling_intent_support::{scan_weak_calendar_with_provider, CalendarTitleProvider};

const COFFEE_SYNC_MESSAGE: &str =
    "Morrow QA live receipt test: coffee sync on 2026-07-17 at 9:30 AM for 30 minutes";
const ONE_ON_ONE_MESSAGE: &str =
    "Morrow QA live receipt test: 1 on 1 meeting on 2026-07-17 at 9:30 AM for 30 minutes";
const LIVE_RECEIPT_TIME: &str = "2026-07-17T09:30:00[Asia/Seoul]";

#[test]
fn production_scan_creates_calendar_proposal_for_coffee_sync_meridiem_message() -> Result<(), String>
{
    // Given
    let provider =
        CountingProvider::new(CalendarTitleProvider::new("coffee sync", LIVE_RECEIPT_TIME));

    // When
    let outcome = scan_weak_calendar_with_provider(COFFEE_SYNC_MESSAGE, &provider)?;

    // Then
    assert_eq!(outcome.provider_calls, 1);
    assert_eq!(outcome.created_titles, ["coffee sync"]);
    assert_eq!(outcome.external_mappings, 1);
    assert_eq!(outcome.route_rows, 1);
    println!(
        "coffee_sync_provider_route_native provider_calls={} external_mappings={} route_rows={}",
        outcome.provider_calls, outcome.external_mappings, outcome.route_rows
    );
    Ok(())
}

#[test]
fn production_scan_routes_one_on_one_meeting_to_provider_title() -> Result<(), String> {
    // Given
    let provider = CountingProvider::new(CalendarTitleProvider::new(
        "1 on 1 meeting",
        LIVE_RECEIPT_TIME,
    ));

    // When
    let outcome = scan_weak_calendar_with_provider(ONE_ON_ONE_MESSAGE, &provider)?;

    // Then
    assert_eq!(outcome.provider_calls, 1);
    assert_eq!(outcome.created_titles, ["1 on 1 meeting"]);
    assert_eq!(outcome.external_mappings, 1);
    assert_eq!(outcome.route_rows, 1);
    assert!(
        outcome.ledger_dump.contains("|Messages event candidate|"),
        "{}",
        outcome.ledger_dump
    );
    assert_provider_route_dump_hides(
        &outcome.ledger_dump,
        &[
            ONE_ON_ONE_MESSAGE,
            "1 on 1 meeting",
            "{\"kind\"",
            "\"title\"",
            "\"calendar_event\"",
        ],
    );
    println!(
        "one_on_one_provider_route_native provider_calls={} visible_title=\"{}\" external_mappings={} route_rows={} ledger={}",
        outcome.provider_calls,
        outcome.created_titles.join(","),
        outcome.external_mappings,
        outcome.route_rows,
        outcome.ledger_dump.trim()
    );
    Ok(())
}

#[test]
fn production_scan_falls_back_to_one_on_one_title_when_provider_unavailable() -> Result<(), String>
{
    // Given
    let provider = CountingProvider::new(UnavailableTestProvider);

    // When
    let outcome = scan_weak_calendar_with_provider(ONE_ON_ONE_MESSAGE, &provider)?;

    // Then
    assert_eq!(outcome.provider_calls, 1);
    assert_eq!(outcome.created_titles, ["1 on 1 meeting"]);
    assert_eq!(outcome.external_mappings, 1);
    assert_eq!(outcome.route_rows, 0);
    println!(
        "one_on_one_provider_unavailable_fallback provider_calls={} external_mappings={} route_rows={}",
        outcome.provider_calls, outcome.external_mappings, outcome.route_rows
    );
    Ok(())
}

#[test]
fn production_scan_falls_back_to_calendar_proposal_when_coffee_sync_provider_unavailable(
) -> Result<(), String> {
    // Given
    let provider = CountingProvider::new(UnavailableTestProvider);

    // When
    let outcome = scan_weak_calendar_with_provider(COFFEE_SYNC_MESSAGE, &provider)?;

    // Then
    assert_eq!(outcome.provider_calls, 1);
    assert_eq!(outcome.created_titles, ["coffee sync"]);
    assert_eq!(outcome.external_mappings, 1);
    assert_eq!(outcome.route_rows, 0);
    println!(
        "coffee_sync_provider_unavailable_fallback provider_calls={} external_mappings={} route_rows={}",
        outcome.provider_calls, outcome.external_mappings, outcome.route_rows
    );
    Ok(())
}
