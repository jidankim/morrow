use crate::ids::CandidateId;
use crate::privacy::summarize_privacy;
use crate::sqlite_cli::{row_value, sql_text, Sqlite};
use crate::types::{
    AuditEntry, CandidateDraft, CandidateState, ExternalObjectMapping, ExternalSource,
    PrivacySummary, QuietLogDraft, ReplayStream,
};
use crate::validation::{
    transition_allowed, validate_candidate_draft, validate_mapping, validate_quiet_log,
};
use crate::StorageError;

mod candidates;
mod feedback_eval_cases;
mod feedback_eval_records;
mod feedback_eval_sql;
mod feedback_eval_validation;
mod feedback_privacy_metadata;
mod provider_route_invalidation;
mod provider_routes;
mod sync_scheduler;

const QUIET_LOG_RETENTION_SECONDS: i64 = 30 * 24 * 60 * 60;

#[derive(Debug, Clone)]
pub struct Store {
    pub(crate) sqlite: Sqlite,
    pub(crate) db_path: std::path::PathBuf,
}

impl Store {
    pub fn table_names(&self) -> Result<Vec<String>, StorageError> {
        self.sqlite.query_first_column(
            "SELECT name FROM sqlite_schema WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name;",
        )
    }

    pub fn create_candidate(&self, draft: CandidateDraft) -> Result<CandidateId, StorageError> {
        validate_candidate_draft(&draft)?;
        let candidate_id = CandidateId::derive(
            draft.kind,
            &draft.chat_guid,
            &draft.anchor_message_guid,
            &draft.normalized_time,
        );
        let sql = format!(
            "BEGIN;
             INSERT OR IGNORE INTO candidates
             (id, kind, state, chat_guid, anchor_message_guid, title, confidence_millis,
              normalized_time, candidate_version, current_reason, created_at, updated_at)
             VALUES ({id}, {kind}, 'queued', {chat}, {message}, {title}, {confidence},
                     {normalized}, 1, 'created', {created_at}, {created_at});
             INSERT OR IGNORE INTO evidence
             (candidate_id, message_guid, role, excerpt, message_timestamp, created_at)
             VALUES ({id}, {message}, 'anchor', {excerpt}, {created_at}, {created_at});
             COMMIT;",
            id = sql_text(candidate_id.as_str())?,
            kind = sql_text(draft.kind.as_str())?,
            chat = sql_text(&draft.chat_guid)?,
            message = sql_text(&draft.anchor_message_guid)?,
            title = sql_text(&draft.title)?,
            confidence = draft.confidence_millis,
            normalized = sql_text(&draft.normalized_time)?,
            created_at = draft.observed_at,
            excerpt = sql_text(&draft.evidence_excerpt)?,
        );
        self.sqlite.execute(&sql)?;
        Ok(candidate_id)
    }

    pub fn transition_candidate(
        &self,
        candidate_id: &CandidateId,
        to_state: CandidateState,
        reason: &str,
        observed_at: i64,
    ) -> Result<(), StorageError> {
        crate::validation::validate_text("reason", reason, 240)?;
        let from_state = self.candidate_state(candidate_id)?;
        if !transition_allowed(from_state, to_state) {
            return Err(StorageError::InvalidTransition {
                from: from_state.as_str().to_owned(),
                to: to_state.as_str().to_owned(),
            });
        }
        let sql = format!(
            "BEGIN;
             UPDATE candidates SET state = {to_state}, current_reason = {reason}, updated_at = {observed_at}
             WHERE id = {id};
             INSERT INTO audit_log (candidate_id, from_state, to_state, reason, created_at)
             VALUES ({id}, {from_state}, {to_state}, {reason}, {observed_at});
             COMMIT;",
            id = sql_text(candidate_id.as_str())?,
            from_state = sql_text(from_state.as_str())?,
            to_state = sql_text(to_state.as_str())?,
            reason = sql_text(reason)?,
        );
        self.sqlite.execute(&sql)
    }

    pub fn audit_entries(
        &self,
        candidate_id: &CandidateId,
    ) -> Result<Vec<AuditEntry>, StorageError> {
        let sql = format!(
            "SELECT from_state, to_state, reason FROM audit_log WHERE candidate_id = {} ORDER BY id;",
            sql_text(candidate_id.as_str())?
        );
        self.sqlite
            .query_rows(&sql)?
            .into_iter()
            .map(|row| {
                let from_state = row_value(&row, 0, "audit.from_state")?;
                let to_state = row_value(&row, 1, "audit.to_state")?;
                let reason = row_value(&row, 2, "audit.reason")?.to_owned();
                Ok(AuditEntry {
                    from_state: CandidateState::parse(from_state)?,
                    to_state: CandidateState::parse(to_state)?,
                    reason,
                })
            })
            .collect()
    }

    pub fn record_quiet_log(&self, draft: QuietLogDraft) -> Result<(), StorageError> {
        validate_quiet_log(&draft)?;
        let expires_at = draft.created_at + QUIET_LOG_RETENTION_SECONDS;
        let sql = format!(
            "INSERT INTO quiet_logs
             (chat_guid, anchor_message_guid, reason, excerpt, provider_diagnostic, created_at, expires_at)
             VALUES ({chat}, {message}, {reason}, {excerpt}, {provider_diagnostic}, {created_at}, {expires_at});",
            chat = sql_text(&draft.chat_guid)?,
            message = sql_text(&draft.anchor_message_guid)?,
            reason = sql_text(&draft.reason)?,
            excerpt = sql_text(&draft.excerpt)?,
            provider_diagnostic = draft
                .provider_diagnostic
                .as_deref()
                .map(sql_text)
                .transpose()?
                .unwrap_or_else(|| "NULL".to_owned()),
            created_at = draft.created_at,
        );
        self.sqlite.execute(&sql)
    }

    pub fn expire_quiet_logs(&self, now: i64) -> Result<i64, StorageError> {
        let cutoff = now - QUIET_LOG_RETENTION_SECONDS;
        let sql = format!("DELETE FROM quiet_logs WHERE created_at < {cutoff}; SELECT changes();");
        self.sqlite.query_scalar_i64(&sql)
    }

    pub fn quiet_log_count(&self) -> Result<i64, StorageError> {
        self.sqlite
            .query_scalar_i64("SELECT COUNT(*) FROM quiet_logs;")
    }

    pub fn next_replay_candidates(
        &self,
        stream: ReplayStream,
        limit: usize,
    ) -> Result<Vec<CandidateId>, StorageError> {
        let source = match stream {
            ReplayStream::CalendarProposals => ExternalSource::Calendar,
        };
        let sql = format!(
            "SELECT c.id
             FROM candidates c
             LEFT JOIN external_object_mappings m
               ON m.candidate_id = c.id AND m.source = {source}
             WHERE c.state = 'visible'
               AND c.kind = 'calendar_event'
               AND m.id IS NULL
             ORDER BY c.created_at, c.id
             LIMIT {limit};",
            source = sql_text(source.as_str())?,
        );
        self.sqlite
            .query_first_column(&sql)?
            .into_iter()
            .map(|raw| CandidateId::from_storage(&raw))
            .collect()
    }

    pub fn upsert_external_mapping(
        &self,
        mapping: ExternalObjectMapping,
    ) -> Result<(), StorageError> {
        validate_mapping(&mapping)?;
        if let Some(existing) = self.existing_external_owner(&mapping)? {
            if existing != mapping.candidate_id.as_str() {
                return Err(StorageError::ExternalMappingConflict {
                    external_object_id: mapping.external_object_id,
                });
            }
            return Ok(());
        }
        let sql = format!(
            "INSERT INTO external_object_mappings
             (candidate_id, source, external_object_id, external_source_id, mapped_at)
             VALUES ({id}, {source}, {object_id}, {source_id}, {mapped_at})
             ON CONFLICT(candidate_id, source) DO UPDATE SET
               external_object_id = excluded.external_object_id,
               external_source_id = excluded.external_source_id,
               mapped_at = excluded.mapped_at;",
            id = sql_text(mapping.candidate_id.as_str())?,
            source = sql_text(mapping.source.as_str())?,
            object_id = sql_text(&mapping.external_object_id)?,
            source_id = sql_text(&mapping.external_source_id)?,
            mapped_at = mapping.mapped_at,
        );
        self.sqlite.execute(&sql)
    }

    pub fn replay_cursor(&self, stream: ReplayStream) -> Result<i64, StorageError> {
        let sql = format!(
            "SELECT cursor_value FROM replay_cursors WHERE stream = {};",
            sql_text(stream.as_str())?
        );
        self.sqlite.query_scalar_i64(&sql)
    }

    pub fn advance_replay_cursor(
        &self,
        stream: ReplayStream,
        cursor_value: i64,
        updated_at: i64,
    ) -> Result<(), StorageError> {
        let sql = format!(
            "INSERT INTO replay_cursors (stream, cursor_value, updated_at)
             VALUES ({stream}, {cursor_value}, {updated_at})
             ON CONFLICT(stream) DO UPDATE SET
               cursor_value = excluded.cursor_value,
               updated_at = excluded.updated_at;",
            stream = sql_text(stream.as_str())?,
        );
        self.sqlite.execute(&sql)
    }

    pub fn privacy_summary(&self) -> Result<PrivacySummary, StorageError> {
        summarize_privacy(&self.sqlite, self.table_names()?.len())
    }

    fn existing_external_owner(
        &self,
        mapping: &ExternalObjectMapping,
    ) -> Result<Option<String>, StorageError> {
        let sql = format!(
            "SELECT candidate_id FROM external_object_mappings
             WHERE source = {source} AND external_object_id = {object_id};",
            source = sql_text(mapping.source.as_str())?,
            object_id = sql_text(&mapping.external_object_id)?,
        );
        let owners = self.sqlite.query_first_column(&sql)?;
        Ok(owners.into_iter().next())
    }
}
