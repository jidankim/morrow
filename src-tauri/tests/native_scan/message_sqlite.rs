use std::{path::Path, process::Command};

pub(super) fn create_messages_fixture(db_path: &Path) -> Result<(), String> {
    let sql = format!(
        "
        CREATE TABLE chat (ROWID INTEGER PRIMARY KEY, guid TEXT NOT NULL, display_name TEXT);
        CREATE TABLE handle (ROWID INTEGER PRIMARY KEY, id TEXT NOT NULL);
        CREATE TABLE message (
            ROWID INTEGER PRIMARY KEY,
            guid TEXT NOT NULL,
            date INTEGER NOT NULL,
            text TEXT,
            attributedBody BLOB,
            handle_id INTEGER
        );
        CREATE TABLE chat_message_join (chat_id INTEGER NOT NULL, message_id INTEGER NOT NULL);
        CREATE TABLE chat_handle_join (chat_id INTEGER NOT NULL, handle_id INTEGER NOT NULL);
        INSERT INTO chat (ROWID, guid, display_name)
            VALUES (1, 'iMessage;-;+15555550103', 'Messages chat');
        INSERT INTO handle (ROWID, id) VALUES (3, '+15555550103');
        INSERT INTO chat_handle_join (chat_id, handle_id) VALUES (1, 3);
        INSERT INTO message (ROWID, guid, date, text, attributedBody, handle_id)
            VALUES (1, 'beta-provider-route', {}, 'Maybe meet tomorrow?', NULL, 3);
        INSERT INTO chat_message_join (chat_id, message_id) VALUES (1, 1);
        ",
        apple_nanoseconds(1_782_352_400)
    );
    run_sqlite(db_path, &sql)
}

pub(super) fn external_mapping_count(db_path: &Path) -> Result<i64, String> {
    external_mapping_count_for_source(db_path, "calendar")
}

pub(super) fn external_mapping_count_for_source(
    db_path: &Path,
    source: &str,
) -> Result<i64, String> {
    let source = match source {
        "calendar" => "calendar",
        "reminders" => "reminders",
        other => return Err(format!("unsupported external mapping source {other}")),
    };
    query_sqlite_i64(
        db_path,
        &format!("SELECT COUNT(*) FROM external_object_mappings WHERE source = '{source}';"),
    )
}

pub(super) fn candidate_external_receipt_count(db_path: &Path) -> Result<i64, String> {
    query_sqlite_i64(
        db_path,
        "SELECT COUNT(*) FROM candidates WHERE external_object_id IS NOT NULL AND external_source_id IS NOT NULL;",
    )
}

pub(super) fn provider_route_outcome_count(db_path: &Path) -> Result<i64, String> {
    query_sqlite_i64(db_path, "SELECT COUNT(*) FROM provider_route_outcomes;")
}

pub(super) fn provider_route_outcome_dump(db_path: &Path) -> Result<String, String> {
    query_sqlite(
        db_path,
        "SELECT route_fingerprint || '|' ||
                provider_route_contract_version || '|' ||
                provider_candidate_schema_version || '|' ||
                evidence_payload_hash || '|' ||
                provider_id || '|' ||
                model_id || '|' ||
                prompt_version || '|' ||
                source_excerpt_policy || '|' ||
                reference_observed || '|' ||
                reference_timezone || '|' ||
                threshold_millis || '|' ||
                parser_route || '|' ||
                outcome_kind || '|' ||
                IFNULL(candidate_title, '') || '|' ||
                IFNULL(candidate_evidence_excerpt, '') || '|' ||
                IFNULL(quiet_reason, '')
         FROM provider_route_outcomes
         ORDER BY route_fingerprint;",
    )
}

pub(super) fn provider_route_fingerprint_count(db_path: &Path) -> Result<i64, String> {
    query_sqlite_i64(
        db_path,
        "SELECT COUNT(DISTINCT route_fingerprint) FROM provider_route_outcomes;",
    )
}

pub(super) fn corrupt_provider_route_contract_version(db_path: &Path) -> Result<(), String> {
    run_sqlite(
        db_path,
        "UPDATE provider_route_outcomes
         SET provider_route_contract_version = 'provider-route-ledger-v0';",
    )
}

pub(super) fn corrupt_provider_candidate_schema_version(db_path: &Path) -> Result<(), String> {
    run_sqlite(
        db_path,
        "UPDATE provider_route_outcomes
         SET provider_candidate_schema_version = 'provider-candidate-schema-v0';",
    )
}

pub(super) fn update_provider_route_message_text(db_path: &Path, text: &str) -> Result<(), String> {
    run_sqlite(
        db_path,
        &format!(
            "UPDATE message SET text = {} WHERE guid = 'beta-provider-route';",
            sql_text(text)
        ),
    )
}

pub(super) fn candidate_reasons(db_path: &Path) -> Result<String, String> {
    query_sqlite(
        db_path,
        "SELECT current_reason FROM candidates UNION ALL SELECT reason FROM audit_log ORDER BY 1;",
    )
}

pub(super) fn install_external_mapping_failure_trigger(db_path: &Path) -> Result<(), String> {
    run_sqlite(
        db_path,
        "CREATE TRIGGER fail_external_mapping_insert BEFORE INSERT ON external_object_mappings BEGIN SELECT RAISE(FAIL, 'simulated post-create storage failure'); END;",
    )
}

pub(super) fn drop_external_mapping_failure_trigger(db_path: &Path) -> Result<(), String> {
    run_sqlite(db_path, "DROP TRIGGER fail_external_mapping_insert;")
}

pub(super) fn install_candidate_failure_trigger(db_path: &Path) -> Result<(), String> {
    run_sqlite(
        db_path,
        "CREATE TRIGGER fail_candidate_insert BEFORE INSERT ON candidates BEGIN SELECT RAISE(FAIL, 'simulated candidate persistence failure'); END;",
    )
}

pub(super) fn drop_candidate_failure_trigger(db_path: &Path) -> Result<(), String> {
    run_sqlite(db_path, "DROP TRIGGER fail_candidate_insert;")
}

pub(super) fn install_quiet_log_failure_trigger(db_path: &Path) -> Result<(), String> {
    run_sqlite(
        db_path,
        "CREATE TRIGGER fail_quiet_log_insert BEFORE INSERT ON quiet_logs BEGIN SELECT RAISE(FAIL, 'simulated quiet log persistence failure'); END;",
    )
}

pub(super) fn drop_quiet_log_failure_trigger(db_path: &Path) -> Result<(), String> {
    run_sqlite(db_path, "DROP TRIGGER fail_quiet_log_insert;")
}

fn run_sqlite(db_path: &Path, sql: &str) -> Result<(), String> {
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

fn query_sqlite(db_path: &Path, sql: &str) -> Result<String, String> {
    let output = Command::new("sqlite3")
        .arg("-batch")
        .arg("-noheader")
        .arg(db_path)
        .arg(sql)
        .output()
        .map_err(|error| error.to_string())?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_owned())
    }
}

fn query_sqlite_i64(db_path: &Path, sql: &str) -> Result<i64, String> {
    query_sqlite(db_path, sql)?
        .trim()
        .parse::<i64>()
        .map_err(|error| error.to_string())
}

fn sql_text(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

const fn apple_nanoseconds(unix_seconds: i64) -> i64 {
    (unix_seconds - 978_307_200) * 1_000_000_000
}
