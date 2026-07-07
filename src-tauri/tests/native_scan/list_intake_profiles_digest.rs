use super::list_intake_profiles_support::{
    chat_scope_all, list_intake_proposal_count, profile_json, provider_json, request_with_profiles,
    ListIntakeFixture, ListIntakeProvider,
};
use morrow_storage::{ListIntakeProposalDecision, Store};
use serde_json::{json, Value};

use super::support::query_sqlite;

#[test]
fn list_intake_profiles_aggregate_only_does_not_create_digest_reminder() -> Result<(), String> {
    let fixture = ListIntakeFixture::with_text("2 salmon, 3 tuna")?;
    let provider = ListIntakeProvider::new(provider_json(900));
    let request = request_with_profiles(vec![profile_json(true, true, chat_scope_all())])?;

    let result = fixture.scan(request, &provider)?;

    assert_eq!(provider.list_calls(), 1);
    assert_eq!(result.created_external_proposal_count, 0);
    assert_eq!(digest_candidate_count(&fixture.store_path)?, 0);
    println!(
        "list_intake_digest_aggregate_only provider_calls={} digest_candidates={} external_reminders={}",
        provider.list_calls(),
        digest_candidate_count(&fixture.store_path)?,
        result.created_external_proposal_count
    );
    Ok(())
}

#[test]
fn list_intake_profiles_daily_digest_reminder_creates_one_group_digest() -> Result<(), String> {
    let fixture = ListIntakeFixture::with_text("2 salmon, 3 tuna")?;
    let provider = ListIntakeProvider::new(provider_json(900));
    let request = request_with_profiles(vec![digest_profile_json()])?;

    let first = fixture.scan(request.clone(), &provider)?;
    let second = fixture.scan(request, &provider)?;

    let expected_key = expected_digest_key(&fixture.store_path)?;
    let payload = digest_candidate_payload(&fixture.store_path)?;
    assert_eq!(provider.list_calls(), 2);
    assert_eq!(first.created_external_proposal_count, 1);
    assert_eq!(second.created_external_proposal_count, 0);
    assert_eq!(digest_candidate_count(&fixture.store_path)?, 1);
    assert_eq!(payload.anchor_message_guid, expected_key);
    assert_eq!(
        payload.title,
        "Fish count daily digest: Sender 1 fish - salmon 2, tuna 3"
    );
    assert_eq!(payload.normalized_time, "2026-06-26T09:00:00[Asia/Seoul]");
    assert!(!payload.anchor_message_guid.contains("+15555550103"));
    assert!(!payload.title.contains("+15555550103"));
    println!(
        "list_intake_digest_explicit candidates=1 title=\"{}\" due={} idempotency_key={}",
        payload.title, payload.normalized_time, payload.anchor_message_guid
    );
    Ok(())
}

#[test]
fn list_intake_profiles_daily_digest_uses_rows_from_approved_review_proposal() -> Result<(), String>
{
    let fixture = ListIntakeFixture::with_text("2 salmon, 3 tuna")?;
    let provider = ListIntakeProvider::new(provider_json(700));
    let request = request_with_profiles(vec![digest_profile_json()])?;

    let first = fixture.scan(request.clone(), &provider)?;
    approve_pending_list_intake_proposal(&fixture.store_path)?;
    let second = fixture.scan(request, &provider)?;

    let payload = digest_candidate_payload(&fixture.store_path)?;
    assert_eq!(provider.list_calls(), 2);
    assert_eq!(first.created_external_proposal_count, 0);
    assert_eq!(second.created_external_proposal_count, 1);
    assert_eq!(list_intake_proposal_count(&fixture.store_path)?, 1);
    assert_eq!(digest_candidate_count(&fixture.store_path)?, 1);
    assert_eq!(
        payload.title,
        "Fish count daily digest: Sender 1 fish - salmon 2, tuna 3"
    );
    assert_eq!(payload.normalized_time, "2026-06-26T09:00:00[Asia/Seoul]");
    println!(
        "list_intake_digest_approved_review proposals=1 digest_candidates=1 title=\"{}\" due={}",
        payload.title, payload.normalized_time
    );
    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
struct DigestPayload {
    anchor_message_guid: String,
    title: String,
    normalized_time: String,
}

fn digest_profile_json() -> Value {
    let mut profile = profile_json(true, true, chat_scope_all());
    profile["outputPolicy"] = json!("dailyDigestReminder");
    profile["digestReminder"] = json!({
        "dueTimeLocal": "09:00",
        "dateOffsetDays": 1,
        "outputPolicyVersion": "list-intake-digest-v1"
    });
    profile
}

fn approve_pending_list_intake_proposal(db_path: &std::path::Path) -> Result<(), String> {
    let store = Store::open(db_path).map_err(|error| error.to_string())?;
    let proposal = store
        .list_intake_proposals("list-intake-fishcount")
        .map_err(|error| error.to_string())?
        .into_iter()
        .next()
        .ok_or_else(|| "missing list-intake proposal".to_owned())?;
    store
        .decide_list_intake_proposal(ListIntakeProposalDecision::ApproveEdited {
            proposal_id: proposal.proposal_id,
            items: proposal.items,
            allowed_category_ids: vec!["fish".to_owned()],
            decided_at: 1_782_352_500,
        })
        .map_err(|error| error.to_string())
}

fn digest_candidate_count(db_path: &std::path::Path) -> Result<i64, String> {
    query_sqlite(
        db_path,
        "SELECT COUNT(*) FROM candidates
         WHERE kind = 'task_reminder'
           AND anchor_message_guid LIKE '%|list-intake-digest-v1';",
    )?
    .trim()
    .parse::<i64>()
    .map_err(|error| error.to_string())
}

fn digest_candidate_payload(db_path: &std::path::Path) -> Result<DigestPayload, String> {
    let raw = query_sqlite(
        db_path,
        "SELECT anchor_message_guid || char(9) || title || char(9) || normalized_time
         FROM candidates
         WHERE kind = 'task_reminder'
           AND anchor_message_guid LIKE '%|list-intake-digest-v1'
         ORDER BY id;",
    )?;
    let line = raw
        .lines()
        .next()
        .ok_or_else(|| "missing digest candidate".to_owned())?;
    let fields = line.split('\t').collect::<Vec<_>>();
    if fields.len() != 3 {
        return Err(format!("unexpected digest payload: {line}"));
    }
    Ok(DigestPayload {
        anchor_message_guid: fields[0].to_owned(),
        title: fields[1].to_owned(),
        normalized_time: fields[2].to_owned(),
    })
}

fn expected_digest_key(db_path: &std::path::Path) -> Result<String, String> {
    let raw = query_sqlite(
        db_path,
        "SELECT profile_id || '|' || profile_version || '|' || examples_hash
                || '|' || window_start_unix_seconds || '|' || chat_key
                || '|' || sender_label || '|' || category_id
         FROM list_intake_entries
         GROUP BY profile_id, profile_version, examples_hash, window_start_unix_seconds,
                  chat_key, sender_label, category_id;",
    )?;
    let line = raw
        .lines()
        .next()
        .ok_or_else(|| "missing aggregate entry group".to_owned())?;
    let fields = line.split('|').collect::<Vec<_>>();
    if fields.len() != 7 {
        return Err(format!("unexpected aggregate group: {line}"));
    }
    Ok(format!(
        "{}|{}|{}|{}|chat:{};sender:{};category:{}|list-intake-digest-v1",
        fields[0], fields[1], fields[2], fields[3], fields[4], fields[5], fields[6]
    ))
}
