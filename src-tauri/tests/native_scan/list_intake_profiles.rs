use super::list_intake_profiles_support::{
    chat_scope_all, chat_scope_selected, list_intake_aggregate_count, list_intake_entry_count,
    list_intake_proposal_count, list_intake_proposal_item_count, profile_json, provider_json,
    provider_json_salmon, request_with_profiles, ListIntakeFixture, ListIntakeProvider,
};
use super::message_sqlite::provider_route_outcome_count;
use super::support::query_sqlite;
use serde_json::json;

#[test]
fn list_intake_enabled_profile_auto_aggregates_bare_quantity_list() -> Result<(), String> {
    let fixture = ListIntakeFixture::with_text("2 salmon, 3 tuna")?;
    let provider = ListIntakeProvider::new(provider_json(900));
    let request = request_with_profiles(vec![profile_json(true, true, chat_scope_all())])?;

    let result = fixture.scan(request, &provider)?;

    let rows = list_intake_entry_count(&fixture.store_path)?;
    let proposals = list_intake_proposal_count(&fixture.store_path)?;
    let aggregates = list_intake_aggregate_count(&fixture.store_path)?;
    assert_eq!(provider.list_calls(), 1);
    assert!(rows > 0);
    assert_eq!(proposals, 0);
    assert!(aggregates > 0);
    assert_eq!(result.created_external_proposal_count, 0);
    assert_eq!(provider_route_outcome_count(&fixture.store_path)?, 0);
    println!(
        "list_intake_high provider_calls={} entry_rows={} proposal_rows={} aggregate_rows={} scheduling_route_rows={}",
        provider.list_calls(),
        rows,
        proposals,
        aggregates,
        provider_route_outcome_count(&fixture.store_path)?
    );
    Ok(())
}

#[test]
fn list_intake_native_boundary_recomputes_examples_hash_before_storage() -> Result<(), String> {
    let fixture = ListIntakeFixture::with_text("2 salmon, 3 tuna")?;
    let provider = ListIntakeProvider::new(provider_json(900));
    let mut profile = profile_json(true, true, chat_scope_all());
    profile["examplesHash"] = json!("caller-controlled-stale-hash");
    let request = request_with_profiles(vec![profile])?;

    let _result = fixture.scan(request, &provider)?;

    let stored_hash = query_sqlite(
        &fixture.store_path,
        "SELECT DISTINCT examples_hash FROM list_intake_entries;",
    )?;
    assert_eq!(provider.list_calls(), 1);
    assert!(stored_hash.trim().starts_with("list-intake-"));
    assert!(!stored_hash.contains("caller-controlled-stale-hash"));
    println!(
        "list_intake_examples_hash_recomputed stored_hash={} caller_hash_trusted=false",
        stored_hash.trim()
    );
    Ok(())
}

#[test]
fn list_intake_review_confidence_creates_proposal_without_aggregate() -> Result<(), String> {
    let fixture = ListIntakeFixture::with_text("2 salmon, 3 tuna")?;
    let provider = ListIntakeProvider::new(provider_json(700));
    let request = request_with_profiles(vec![profile_json(true, true, chat_scope_all())])?;

    let _result = fixture.scan(request, &provider)?;

    let proposals = list_intake_proposal_count(&fixture.store_path)?;
    let proposal_items = list_intake_proposal_item_count(&fixture.store_path)?;
    let aggregates = list_intake_aggregate_count(&fixture.store_path)?;
    assert_eq!(provider.list_calls(), 1);
    assert_eq!(proposals, 1);
    assert_eq!(proposal_items, 2);
    assert_eq!(aggregates, 0);
    println!(
        "list_intake_review provider_calls={} entry_rows={} proposal_rows={} proposal_item_rows={} aggregate_rows={}",
        provider.list_calls(),
        list_intake_entry_count(&fixture.store_path)?,
        proposals,
        proposal_items,
        aggregates
    );
    Ok(())
}

#[test]
fn list_intake_persists_internal_sender_key_not_display_alias() -> Result<(), String> {
    let fixture = ListIntakeFixture::with_text("2 salmon, 3 tuna")?;
    let provider = ListIntakeProvider::new(provider_json(900));
    let request = request_with_profiles(vec![profile_json(true, true, chat_scope_all())])?;

    let _result = fixture.scan(request, &provider)?;

    let sender_key = query_sqlite(
        &fixture.store_path,
        "SELECT DISTINCT sender_key FROM list_intake_entries;",
    )?;
    assert_eq!(provider.list_calls(), 1);
    assert!(sender_key.trim().starts_with("senderKey-"));
    assert!(!sender_key.contains("sender-alias"));
    println!("list_intake_sender_key_persistence internal_key_prefix=true alias_persisted=false");
    Ok(())
}

#[test]
fn list_intake_gates_skip_provider_without_storage_side_effects() -> Result<(), String> {
    let cases = [
        (
            "disabled",
            "2 salmon, 3 tuna",
            vec![profile_json(false, true, chat_scope_all())],
        ),
        ("no_profile", "2 salmon, 3 tuna", Vec::new()),
        (
            "chat_scope_mismatch",
            "2 salmon, 3 tuna",
            vec![profile_json(
                true,
                true,
                chat_scope_selected("messages-chat-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
            )],
        ),
        (
            "scheduling_owned_default",
            "Meet 2026-07-15 14:00, 2 salmon",
            vec![profile_json(true, false, chat_scope_all())],
        ),
    ];

    for (name, text, profiles) in cases {
        let fixture = ListIntakeFixture::with_text(text)?;
        let provider = ListIntakeProvider::new(provider_json(900));
        let request = request_with_profiles(profiles)?;

        let _result = fixture.scan(request, &provider)?;

        assert_eq!(provider.list_calls(), 0, "{name}");
        assert_eq!(list_intake_entry_count(&fixture.store_path)?, 0, "{name}");
        assert_eq!(
            list_intake_proposal_count(&fixture.store_path)?,
            0,
            "{name}"
        );
        println!(
            "list_intake_gate case={name} provider_calls={} entry_rows={} proposal_rows={} scheduling_route_rows={}",
            provider.list_calls(),
            list_intake_entry_count(&fixture.store_path)?,
            list_intake_proposal_count(&fixture.store_path)?,
            provider_route_outcome_count(&fixture.store_path)?
        );
    }
    Ok(())
}

#[test]
fn list_intake_capture_from_scheduled_messages_allows_scheduling_owned_list() -> Result<(), String>
{
    let fixture = ListIntakeFixture::with_text("Meet 2026-07-15 14:00, 2 salmon")?;
    let provider = ListIntakeProvider::new(provider_json_salmon(900));
    let request = request_with_profiles(vec![profile_json(true, true, chat_scope_all())])?;

    let _result = fixture.scan(request, &provider)?;

    assert_eq!(provider.list_calls(), 1);
    assert!(list_intake_entry_count(&fixture.store_path)? > 0);
    assert_eq!(provider_route_outcome_count(&fixture.store_path)?, 0);
    println!(
        "list_intake_scheduling_capture provider_calls={} entry_rows={} proposal_rows={} scheduling_route_rows={}",
        provider.list_calls(),
        list_intake_entry_count(&fixture.store_path)?,
        list_intake_proposal_count(&fixture.store_path)?,
        provider_route_outcome_count(&fixture.store_path)?
    );
    Ok(())
}

#[test]
fn list_intake_provider_unavailable_and_rescan_are_side_effect_safe() -> Result<(), String> {
    let unavailable = ListIntakeProvider::unavailable();
    let unavailable_fixture = ListIntakeFixture::with_text("2 salmon, 3 tuna")?;
    let request = request_with_profiles(vec![profile_json(true, true, chat_scope_all())])?;
    let _result = unavailable_fixture.scan(request.clone(), &unavailable)?;
    assert_eq!(unavailable.list_calls(), 1);
    assert_eq!(list_intake_entry_count(&unavailable_fixture.store_path)?, 0);
    assert_eq!(
        list_intake_proposal_count(&unavailable_fixture.store_path)?,
        0
    );
    let unavailable_diagnostic = query_sqlite(
        &unavailable_fixture.store_path,
        "SELECT reason_code || '|' || retry_state || '|' || message_hash || '|' || chat_key
         FROM list_intake_provider_diagnostics;",
    )?;
    assert!(unavailable_diagnostic.contains("provider_unavailable|retry_available|message-hash-"));
    assert!(!unavailable_diagnostic.contains("+15555550103"));
    assert!(!unavailable_diagnostic.contains("beta-provider-route"));

    let idempotent_fixture = ListIntakeFixture::with_text("2 salmon, 3 tuna")?;
    let provider = ListIntakeProvider::new(provider_json(900));
    let _first = idempotent_fixture.scan(request.clone(), &provider)?;
    let _second = idempotent_fixture.scan(request, &provider)?;

    assert_eq!(provider.list_calls(), 2);
    assert_eq!(list_intake_entry_count(&idempotent_fixture.store_path)?, 2);
    assert_eq!(
        list_intake_proposal_count(&idempotent_fixture.store_path)?,
        0
    );
    println!(
        "list_intake_stale_state unavailable_calls={} idempotent_calls={} entry_rows={} proposal_rows={}",
        unavailable.list_calls(),
        provider.list_calls(),
        list_intake_entry_count(&idempotent_fixture.store_path)?,
        list_intake_proposal_count(&idempotent_fixture.store_path)?
    );
    Ok(())
}

#[test]
fn list_intake_review_confidence_rescan_keeps_single_proposal() -> Result<(), String> {
    let fixture = ListIntakeFixture::with_text("2 salmon, 3 tuna")?;
    let provider = ListIntakeProvider::new(provider_json(700));
    let request = request_with_profiles(vec![profile_json(true, true, chat_scope_all())])?;

    let _first = fixture.scan(request.clone(), &provider)?;
    let _second = fixture.scan(request, &provider)?;

    assert_eq!(provider.list_calls(), 2);
    assert_eq!(list_intake_entry_count(&fixture.store_path)?, 0);
    assert_eq!(list_intake_proposal_count(&fixture.store_path)?, 1);
    assert_eq!(list_intake_proposal_item_count(&fixture.store_path)?, 2);
    assert_eq!(list_intake_aggregate_count(&fixture.store_path)?, 0);
    println!(
        "list_intake_review_rescan provider_calls={} entry_rows={} proposal_rows={} proposal_item_rows={} aggregate_rows={}",
        provider.list_calls(),
        list_intake_entry_count(&fixture.store_path)?,
        list_intake_proposal_count(&fixture.store_path)?,
        list_intake_proposal_item_count(&fixture.store_path)?,
        list_intake_aggregate_count(&fixture.store_path)?
    );
    Ok(())
}
