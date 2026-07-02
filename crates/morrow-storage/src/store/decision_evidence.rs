use crate::feedback_eval::FeedbackLabelValue;
use crate::sqlite_cli::{row_value, sql_text};
use crate::store::Store;
use crate::{
    CandidateId, CandidateKind, CandidateState, FeedbackLabelType, FeedbackPrivacyTier,
    FeedbackSourceExcerptPolicy, FeedbackSubjectType, StorageError,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionEvidenceSubjectType {
    Candidate,
    QuietLog,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionEvidenceTraceRetention {
    NotChecked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionEvidenceSummary {
    pub subject_type: DecisionEvidenceSubjectType,
    pub subject_id: String,
    pub candidate_id: Option<CandidateId>,
    pub candidate_state: Option<CandidateState>,
    pub candidate_kind: Option<CandidateKind>,
    pub route: Option<String>,
    pub reason_code: Option<String>,
    pub confidence_millis: Option<i64>,
    pub label_type: FeedbackLabelType,
    pub label_value: FeedbackLabelValue,
    pub source_excerpt_policy: FeedbackSourceExcerptPolicy,
    pub privacy_tier: FeedbackPrivacyTier,
    pub diagnostics_trace_id: Option<String>,
    pub diagnostics_span_id: Option<String>,
    pub diagnostics_chat_hash_present: bool,
    pub diagnostics_message_hash_present: bool,
    pub created_at: i64,
    pub trace_retention: DecisionEvidenceTraceRetention,
}

impl Store {
    pub fn recent_decision_evidence(
        &self,
        limit: usize,
    ) -> Result<Vec<DecisionEvidenceSummary>, StorageError> {
        self.query_decision_evidence("", limit)
    }

    pub fn decision_evidence_for_candidate_ids(
        &self,
        candidate_ids: &[CandidateId],
        limit: usize,
    ) -> Result<Vec<DecisionEvidenceSummary>, StorageError> {
        if candidate_ids.is_empty() {
            return self.recent_decision_evidence(limit);
        }
        if limit == 0 {
            return Ok(Vec::new());
        }
        let quoted_ids = candidate_ids
            .iter()
            .map(|candidate_id| sql_text(candidate_id.as_str()))
            .collect::<Result<Vec<_>, _>>()?;
        let filter = format!(
            " AND s.subject_type = 'candidate' AND s.candidate_id IN ({})",
            quoted_ids.join(", ")
        );
        self.query_decision_evidence(&filter, limit)
    }

    fn query_decision_evidence(
        &self,
        extra_filter: &str,
        limit: usize,
    ) -> Result<Vec<DecisionEvidenceSummary>, StorageError> {
        let sql = decision_evidence_sql(extra_filter, limit);
        self.sqlite
            .query_rows(&sql)?
            .into_iter()
            .map(row_to_summary)
            .collect()
    }
}

fn decision_evidence_sql(extra_filter: &str, limit: usize) -> String {
    format!(
        "SELECT
           s.subject_type, s.subject_id, s.candidate_id, c.state, c.kind,
           s.route, s.reason_code, s.confidence_millis, l.label_type, l.label_value,
           s.source_excerpt_policy, s.privacy_tier, s.diagnostics_trace_id,
           s.diagnostics_span_id, s.diagnostics_chat_hash IS NOT NULL,
           s.diagnostics_message_hash IS NOT NULL, s.created_at
         FROM feature_snapshots s
         JOIN labels l
           ON l.id = (
             SELECT latest.id
             FROM labels latest
             WHERE latest.subject_type = s.subject_type
               AND latest.subject_id = s.subject_id
             ORDER BY latest.id DESC
             LIMIT 1
           )
         LEFT JOIN candidates c ON c.id = s.candidate_id
         WHERE s.subject_type IN ('candidate', 'quiet_log')
           {extra_filter}
         ORDER BY s.created_at DESC, s.id DESC
         LIMIT {limit};"
    )
}

fn row_to_summary(row: Vec<String>) -> Result<DecisionEvidenceSummary, StorageError> {
    let label_type = FeedbackLabelType::parse(row_value(&row, 8, "label.label_type")?)?;
    let label_value = FeedbackLabelValue::parse(
        label_type,
        row_value(&row, 9, "label.label_value")?,
        "label_value",
    )?;
    Ok(DecisionEvidenceSummary {
        subject_type: subject_type(row_value(&row, 0, "snapshot.subject_type")?)?,
        subject_id: row_value(&row, 1, "snapshot.subject_id")?.to_owned(),
        candidate_id: optional_candidate(row_value(&row, 2, "snapshot.candidate_id")?)?,
        candidate_state: optional_candidate_state(row_value(&row, 3, "candidate.state")?)?,
        candidate_kind: optional_candidate_kind(row_value(&row, 4, "candidate.kind")?)?,
        route: optional_cell(row_value(&row, 5, "snapshot.route")?),
        reason_code: optional_cell(row_value(&row, 6, "snapshot.reason_code")?),
        confidence_millis: optional_i64(row_value(&row, 7, "snapshot.confidence_millis")?)?,
        label_type,
        label_value,
        source_excerpt_policy: FeedbackSourceExcerptPolicy::parse(row_value(
            &row,
            10,
            "snapshot.source_excerpt_policy",
        )?)?,
        privacy_tier: FeedbackPrivacyTier::parse(row_value(&row, 11, "snapshot.privacy_tier")?)?,
        diagnostics_trace_id: optional_cell(row_value(&row, 12, "snapshot.diagnostics_trace_id")?),
        diagnostics_span_id: optional_cell(row_value(&row, 13, "snapshot.diagnostics_span_id")?),
        diagnostics_chat_hash_present: row_bool(&row, 14, "diagnostics_chat_hash_present")?,
        diagnostics_message_hash_present: row_bool(&row, 15, "diagnostics_message_hash_present")?,
        created_at: row_i64(&row, 16, "snapshot.created_at")?,
        trace_retention: DecisionEvidenceTraceRetention::NotChecked,
    })
}

fn subject_type(raw: &str) -> Result<DecisionEvidenceSubjectType, StorageError> {
    match FeedbackSubjectType::parse(raw)? {
        FeedbackSubjectType::Candidate => Ok(DecisionEvidenceSubjectType::Candidate),
        FeedbackSubjectType::QuietLog => Ok(DecisionEvidenceSubjectType::QuietLog),
        FeedbackSubjectType::EvalCase => Err(StorageError::InvalidInput {
            field: "subject_type",
            reason: "decision evidence excludes eval_case subjects".to_owned(),
        }),
    }
}

fn optional_cell(value: &str) -> Option<String> {
    match value {
        "" => None,
        value => Some(value.to_owned()),
    }
}

fn optional_candidate(value: &str) -> Result<Option<CandidateId>, StorageError> {
    match optional_cell(value) {
        Some(value) => CandidateId::from_storage(&value).map(Some),
        None => Ok(None),
    }
}

fn optional_candidate_state(value: &str) -> Result<Option<CandidateState>, StorageError> {
    match optional_cell(value) {
        Some(value) => CandidateState::parse(&value).map(Some),
        None => Ok(None),
    }
}

fn optional_candidate_kind(value: &str) -> Result<Option<CandidateKind>, StorageError> {
    match optional_cell(value) {
        Some(value) => CandidateKind::parse(&value).map(Some),
        None => Ok(None),
    }
}

fn optional_i64(value: &str) -> Result<Option<i64>, StorageError> {
    match optional_cell(value) {
        Some(value) => value
            .parse::<i64>()
            .map(Some)
            .map_err(|err| StorageError::Sqlite {
                message: format!("expected integer optional_i64: {err}"),
            }),
        None => Ok(None),
    }
}

fn row_i64(row: &[String], index: usize, field: &'static str) -> Result<i64, StorageError> {
    row_value(row, index, field)?
        .parse::<i64>()
        .map_err(|err| StorageError::Sqlite {
            message: format!("expected integer {field}: {err}"),
        })
}

fn row_bool(row: &[String], index: usize, field: &'static str) -> Result<bool, StorageError> {
    match row_value(row, index, field)? {
        "0" => Ok(false),
        "1" => Ok(true),
        other => Err(StorageError::Sqlite {
            message: format!("expected boolean {field}, got {other}"),
        }),
    }
}
