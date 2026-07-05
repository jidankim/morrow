use crate::caps::{plan_visibility, CapPlan, CapPolicy, QueuedProposal};
use crate::sqlite_cli::{row_value, sql_text};
use crate::types::{CandidateKind, CandidateState, ExternalSource};
use crate::{StorageError, Store};

impl Store {
    pub fn apply_visibility_caps(
        &self,
        policy: CapPolicy,
        observed_at: i64,
    ) -> Result<CapPlan, StorageError> {
        let proposals = self.queued_proposals()?;
        let plan = plan_visibility(&proposals, policy);
        for proposal in &plan.visible {
            self.transition_candidate(
                &proposal.candidate_id,
                CandidateState::CreatingExternal,
                "cap_selected_for_external_creation",
                observed_at,
            )?;
        }
        Ok(plan)
    }

    fn queued_proposals(&self) -> Result<Vec<QueuedProposal>, StorageError> {
        let rows = self.sqlite.query_rows(
            "SELECT id, kind, chat_guid, confidence_millis, normalized_time
             FROM candidates
             WHERE state = 'queued'
             ORDER BY created_at, id;",
        )?;
        rows.into_iter()
            .map(|row| {
                let candidate_id = row_value(&row, 0, "queued.id")?;
                let kind = CandidateKind::parse(row_value(&row, 1, "queued.kind")?)?;
                let chat_guid = row_value(&row, 2, "queued.chat_guid")?;
                let confidence = row_value(&row, 3, "queued.confidence_millis")?
                    .parse::<i64>()
                    .map_err(|error| StorageError::Sqlite {
                        message: format!("invalid queued confidence: {error}"),
                    })?;
                let normalized_time = row_value(&row, 4, "queued.normalized_time")?;
                QueuedProposal::with_kind(
                    candidate_id,
                    kind,
                    chat_guid,
                    confidence,
                    normalized_time,
                    false,
                )
            })
            .collect()
    }

    pub fn recoverable_external_proposals(&self) -> Result<Vec<QueuedProposal>, StorageError> {
        let sql = format!(
            "SELECT c.id, c.kind, c.chat_guid, c.confidence_millis, c.normalized_time
             FROM candidates c
             LEFT JOIN external_object_mappings calendar_mapping
               ON calendar_mapping.candidate_id = c.id AND calendar_mapping.source = {calendar_source}
             LEFT JOIN external_object_mappings reminder_mapping
               ON reminder_mapping.candidate_id = c.id AND reminder_mapping.source = {reminders_source}
             WHERE c.state = {state}
               AND (
                 (c.kind = {calendar_kind} AND calendar_mapping.id IS NULL)
                 OR (c.kind = {reminder_kind} AND reminder_mapping.id IS NULL)
               )
             ORDER BY c.updated_at, c.id;",
            calendar_source = sql_text(ExternalSource::Calendar.as_str())?,
            reminders_source = sql_text(ExternalSource::Reminders.as_str())?,
            state = sql_text(CandidateState::CreatingExternal.as_str())?,
            calendar_kind = sql_text(CandidateKind::CalendarEvent.as_str())?,
            reminder_kind = sql_text(CandidateKind::TaskReminder.as_str())?,
        );
        let rows = self.sqlite.query_rows(&sql)?;
        rows.into_iter()
            .map(|row| {
                let candidate_id = row_value(&row, 0, "recoverable.id")?;
                let kind = CandidateKind::parse(row_value(&row, 1, "recoverable.kind")?)?;
                let chat_guid = row_value(&row, 2, "recoverable.chat_guid")?;
                let confidence = row_value(&row, 3, "recoverable.confidence_millis")?
                    .parse::<i64>()
                    .map_err(|error| StorageError::Sqlite {
                        message: format!("invalid recoverable confidence: {error}"),
                    })?;
                let normalized_time = row_value(&row, 4, "recoverable.normalized_time")?;
                QueuedProposal::with_kind(
                    candidate_id,
                    kind,
                    chat_guid,
                    confidence,
                    normalized_time,
                    false,
                )
            })
            .collect()
    }
}
