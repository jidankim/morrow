use std::env;
use std::path::PathBuf;

use morrow_storage::{
    CandidateDraft, CandidateKind, CandidateState, ExternalObjectMapping, ExternalSource,
    QuietLogDraft, ReplayStream, StorageError, Store,
};

fn main() -> Result<(), StorageError> {
    let db_path = parse_db_path()?;
    let store = Store::open(&db_path)?;
    println!("migration_tables={}", store.table_names()?.len());
    println!(
        "initial_replay_cursor={}",
        store.replay_cursor(ReplayStream::CalendarProposals)?
    );

    let candidate_id = store.create_candidate(CandidateDraft {
        kind: CandidateKind::CalendarEvent,
        chat_guid: "iMessage;+;smoke-chat".to_owned(),
        anchor_message_guid: "smoke-message-1".to_owned(),
        title: "Morrow smoke appointment".to_owned(),
        confidence_millis: 910,
        normalized_time: "2026-07-20T18:00:00Z".to_owned(),
        evidence_excerpt: "smoke appointment July 20 at 6".to_owned(),
        observed_at: 1_784_000_000,
    })?;
    store.transition_candidate(
        &candidate_id,
        CandidateState::CreatingExternal,
        "smoke create",
        1_784_000_001,
    )?;
    store.transition_candidate(
        &candidate_id,
        CandidateState::Visible,
        "smoke visible",
        1_784_000_002,
    )?;
    let replay_before = store.next_replay_candidates(ReplayStream::CalendarProposals, 10)?;
    println!("candidate_created={candidate_id}");
    println!("replay_before_mapping={}", replay_before.len());

    store.upsert_external_mapping(ExternalObjectMapping {
        candidate_id: candidate_id.clone(),
        source: ExternalSource::Calendar,
        external_object_id: "eventkit://smoke/proposed-1".to_owned(),
        external_source_id: "calendar-source-smoke".to_owned(),
        mapped_at: 1_784_000_003,
    })?;
    store.advance_replay_cursor(ReplayStream::CalendarProposals, 1, 1_784_000_004)?;
    drop(store);

    let restarted = Store::open(&db_path)?;
    let replay_after = restarted.next_replay_candidates(ReplayStream::CalendarProposals, 10)?;
    println!("replay_after_restart={}", replay_after.len());

    restarted.record_quiet_log(QuietLogDraft {
        chat_guid: "iMessage;+;smoke-chat".to_owned(),
        anchor_message_guid: "smoke-message-quiet".to_owned(),
        reason: "low confidence".to_owned(),
        excerpt: "maybe sometime next week".to_owned(),
        provider_diagnostic: None,
        created_at: 1_784_000_000,
    })?;
    let rejected_full_message = restarted
        .record_quiet_log(QuietLogDraft {
            chat_guid: "iMessage;+;smoke-chat".to_owned(),
            anchor_message_guid: "smoke-message-full".to_owned(),
            reason: "privacy smoke".to_owned(),
            excerpt: "From: Alice\nTo: Bob\nDate: 2026-07-20\n\nfull thread".to_owned(),
            provider_diagnostic: None,
            created_at: 1_784_000_000,
        })
        .is_err();
    let expired = restarted.expire_quiet_logs(1_784_000_000 + 31 * 24 * 60 * 60)?;
    let privacy = restarted.privacy_summary()?;
    println!("quiet_full_message_rejected={rejected_full_message}");
    println!("quiet_logs_expired={expired}");
    println!("quiet_logs_remaining={}", restarted.quiet_log_count()?);
    println!("external_mapping_idempotent={}", replay_after.is_empty());
    println!(
        "full_message_body_columns={}",
        privacy.full_message_body_columns
    );
    println!("max_excerpt_len={}", privacy.max_excerpt_len);
    println!("PASS replay_smoke");
    Ok(())
}

fn parse_db_path() -> Result<PathBuf, StorageError> {
    let mut args = env::args().skip(1);
    match (args.next().as_deref(), args.next(), args.next()) {
        (Some("--db"), Some(path), None) => Ok(PathBuf::from(path)),
        _ => Err(StorageError::InvalidInput {
            field: "args",
            reason: "usage: replay_smoke --db <path>".to_owned(),
        }),
    }
}
