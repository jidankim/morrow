use std::{path::Path, process::Command};

use morrow_lib::native_bridge::{
    messages_sqlite::{begin_sqlite_query_audit, sqlite_query_audit_snapshot},
    CodexAuthStatus, CodexExecRequest, CodexExecRun, CodexExecRunner, CodexProviderAuthReadiness,
    MessagesDiscoveryCommandReport, MessagesPreviewRequest, NativeBridgeState,
    ProductionScanCodexDependencies,
};
use morrow_messages::MessagesDiscoveryStatus;

use super::dependencies::RecordingProposalAdapter;
use super::message_sqlite::create_messages_fixture;
use super::support::{assert_counts, chat, scan_request};

#[derive(Debug)]
struct UnusedCodexRunner;

impl CodexExecRunner for UnusedCodexRunner {
    fn run_exec(&self, _request: &CodexExecRequest) -> CodexExecRun {
        panic!("unavailable-provider scan should not invoke Codex");
    }
}

#[test]
fn native_discovery_cache_resolves_selected_public_ids_without_all_chat_scan() -> Result<(), String>
{
    // Given
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let store_path = dir.path().join("morrow.sqlite");
    let messages_db_path = dir.path().join("chat.db");
    create_messages_fixture(&messages_db_path)?;
    let state = NativeBridgeState::default();
    let _audit = begin_sqlite_query_audit();

    let discovery = state
        .discover_messages_chats_at(&messages_db_path)
        .map_err(|error| error.to_string())?;
    if discovery.status() != MessagesDiscoveryStatus::Ready {
        return Err(format!("expected ready discovery, got {discovery:?}"));
    }
    let command_discovery = MessagesDiscoveryCommandReport::from_report(&discovery);
    let public_chat_id = command_discovery
        .chats
        .first()
        .map(|chat| chat.chat_id.clone())
        .ok_or_else(|| "missing discovered chat".to_owned())?;
    let participant_id = command_discovery
        .chats
        .first()
        .and_then(|chat| chat.participant_ids.first())
        .cloned()
        .ok_or_else(|| "missing discovered participant".to_owned())?;
    let runner = UnusedCodexRunner;
    let adapter = RecordingProposalAdapter::default();
    let request = scan_request(
        &[chat(public_chat_id.as_str(), 1, &[participant_id.as_str()])],
        &[],
        true,
        1,
        0,
    )?;

    // When
    let result = state
        .scan_selected_chats_at_with_codex_dependencies(
            request,
            &store_path,
            &messages_db_path,
            ProductionScanCodexDependencies {
                auth_readiness: unavailable_auth_readiness(),
                codex_runner: &runner,
                proposal_adapter: &adapter,
            },
        )
        .map_err(|error| error.to_string())?;
    let cached_scan_audit = sqlite_query_audit_snapshot();
    let preview = state
        .load_messages_chat_previews_at(
            &messages_db_path,
            &MessagesPreviewRequest {
                chat_ids: vec![public_chat_id.clone()],
            },
        )
        .map_err(|error| error.to_string())?;
    let cached_preview_audit = sqlite_query_audit_snapshot();

    let fallback_store_path = dir.path().join("morrow-fallback.sqlite");
    let fallback_state = NativeBridgeState::default();
    let fallback_request = scan_request(
        &[chat(public_chat_id.as_str(), 1, &[participant_id.as_str()])],
        &[],
        true,
        1,
        0,
    )?;
    fallback_state
        .scan_selected_chats_at_with_codex_dependencies(
            fallback_request,
            &fallback_store_path,
            &messages_db_path,
            ProductionScanCodexDependencies {
                auth_readiness: unavailable_auth_readiness(),
                codex_runner: &runner,
                proposal_adapter: &adapter,
            },
        )
        .map_err(|error| error.to_string())?;
    let fallback_audit = sqlite_query_audit_snapshot();

    let unknown_store_path = dir.path().join("morrow-unknown.sqlite");
    let unknown_request = scan_request(
        &[
            chat(public_chat_id.as_str(), 1, &[participant_id.as_str()]),
            chat(
                "messages-chat-00000000000000000000000000000000",
                1,
                &[participant_id.as_str()],
            ),
        ],
        &[],
        true,
        1,
        0,
    )?;
    let unknown_error = state
        .scan_selected_chats_at_with_codex_dependencies(
            unknown_request,
            &unknown_store_path,
            &messages_db_path,
            ProductionScanCodexDependencies {
                auth_readiness: unavailable_auth_readiness(),
                codex_runner: &runner,
                proposal_adapter: &adapter,
            },
        )
        .err()
        .ok_or_else(|| "unknown public chat id unexpectedly resolved".to_owned())?
        .to_string();
    let unknown_audit = sqlite_query_audit_snapshot();

    // Then
    assert_counts(&result, (0, 0, 1, 0, 0));
    assert_eq!(
        cached_scan_audit.all_chat_guids_sql_count, 0,
        "discovery cache should resolve selected public IDs without all_chat_guids_sql"
    );
    assert_eq!(preview.chats.len(), 1);
    assert_eq!(
        cached_preview_audit.all_chat_guids_sql_count, 0,
        "discovery cache should resolve preview public IDs without all_chat_guids_sql"
    );
    assert!(
        fallback_audit.all_chat_guids_sql_count > cached_preview_audit.all_chat_guids_sql_count,
        "scan without prior discovery should fall back to all_chat_guids_sql"
    );
    assert!(
        unknown_audit.all_chat_guids_sql_count > fallback_audit.all_chat_guids_sql_count,
        "mixed unknown public IDs should fall back before returning InvalidInput"
    );
    assert!(
        unknown_error.contains("unknown Messages chat id"),
        "{unknown_error}"
    );
    println!(
        "DISCOVERY_CACHE_AUDIT cached_all_chat_guids_sql={} fallback_all_chat_guids_sql={} unknown_all_chat_guids_sql={}",
        cached_preview_audit.all_chat_guids_sql_count,
        fallback_audit.all_chat_guids_sql_count,
        unknown_audit.all_chat_guids_sql_count,
    );
    Ok(())
}

#[test]
fn native_discovery_cache_rejects_stale_public_id_when_same_db_path_changes() -> Result<(), String>
{
    // Given
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let store_path = dir.path().join("morrow-stale-cache.sqlite");
    let messages_db_path = dir.path().join("chat.db");
    create_messages_fixture(&messages_db_path)?;
    let state = NativeBridgeState::default();
    let _audit = begin_sqlite_query_audit();

    let discovery = state
        .discover_messages_chats_at(&messages_db_path)
        .map_err(|error| error.to_string())?;
    if discovery.status() != MessagesDiscoveryStatus::Ready {
        return Err(format!("expected ready discovery, got {discovery:?}"));
    }
    let command_discovery = MessagesDiscoveryCommandReport::from_report(&discovery);
    let public_chat_id = command_discovery
        .chats
        .first()
        .map(|chat| chat.chat_id.clone())
        .ok_or_else(|| "missing discovered chat".to_owned())?;
    let participant_id = command_discovery
        .chats
        .first()
        .and_then(|chat| chat.participant_ids.first())
        .cloned()
        .ok_or_else(|| "missing discovered participant".to_owned())?;
    rename_messages_fixture_chat(&messages_db_path, "iMessage;-;+15555550104")?;
    let runner = UnusedCodexRunner;
    let adapter = RecordingProposalAdapter::default();
    let request = scan_request(
        &[chat(public_chat_id.as_str(), 1, &[participant_id.as_str()])],
        &[],
        true,
        1,
        0,
    )?;

    // When
    let stale_error = state
        .scan_selected_chats_at_with_codex_dependencies(
            request,
            &store_path,
            &messages_db_path,
            ProductionScanCodexDependencies {
                auth_readiness: unavailable_auth_readiness(),
                codex_runner: &runner,
                proposal_adapter: &adapter,
            },
        )
        .err()
        .ok_or_else(|| "stale public chat id unexpectedly resolved from cache".to_owned())?
        .to_string();
    let stale_audit = sqlite_query_audit_snapshot();

    // Then
    assert!(
        stale_error.contains("unknown Messages chat id"),
        "{stale_error}"
    );
    assert!(
        stale_audit.all_chat_guids_sql_count > 0,
        "stale cache should fall back through all_chat_guids_sql"
    );
    println!(
        "DISCOVERY_CACHE_STALE_AUDIT stale_all_chat_guids_sql={}",
        stale_audit.all_chat_guids_sql_count,
    );
    Ok(())
}

fn unavailable_auth_readiness() -> CodexProviderAuthReadiness {
    CodexProviderAuthReadiness {
        status: CodexAuthStatus::NotLoggedIn,
        ready: false,
        command_surface: "codex login status".to_owned(),
        command_output_redacted: true,
        diagnostic: "test provider unavailable".to_owned(),
    }
}

fn rename_messages_fixture_chat(db_path: &Path, chat_guid: &str) -> Result<(), String> {
    let sql = format!(
        "UPDATE chat SET guid = '{}' WHERE ROWID = 1;",
        chat_guid.replace('\'', "''")
    );
    let output = Command::new("sqlite3")
        .arg(db_path)
        .arg(sql)
        .output()
        .map_err(|error| error.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_owned())
    }
}
