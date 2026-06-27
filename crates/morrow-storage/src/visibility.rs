use crate::caps::{plan_visibility, CapPlan, CapPolicy, QueuedProposal};
use crate::sqlite_cli::row_value;
use crate::types::{CandidateKind, CandidateState};
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
                let kind = parse_candidate_kind(row_value(&row, 1, "queued.kind")?)?;
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
}

fn parse_candidate_kind(raw: &str) -> Result<CandidateKind, StorageError> {
    match raw {
        "calendar_event" => Ok(CandidateKind::CalendarEvent),
        "task_reminder" => Ok(CandidateKind::TaskReminder),
        "event_update" => Ok(CandidateKind::EventUpdate),
        "event_reschedule" => Ok(CandidateKind::EventReschedule),
        "event_cancellation" => Ok(CandidateKind::EventCancellation),
        "reminder_update" => Ok(CandidateKind::ReminderUpdate),
        "reminder_reschedule" => Ok(CandidateKind::ReminderReschedule),
        "reminder_cancellation" => Ok(CandidateKind::ReminderCancellation),
        other => Err(StorageError::InvalidInput {
            field: "candidate_kind",
            reason: format!("unknown kind {other}"),
        }),
    }
}
