use crate::feedback_eval::{
    EvalResult, EvalRun, EvalRunStatus, FeatureSnapshot, FeedbackEvalCounts, Label,
};
use crate::sqlite_cli::SqliteValue;
use crate::store::feedback_eval_sql::{
    bool_int, optional_i64, optional_label_type, optional_text, push_common_body_params,
    push_common_params, push_record_times,
};
use crate::store::feedback_eval_validation::{
    validate_eval_result, validate_eval_run, validate_key, validate_snapshot,
};
use crate::{StorageError, Store};

impl Store {
    pub fn record_feedback_event(&self, event: crate::FeedbackEvent) -> Result<(), StorageError> {
        validate_key("event_key", &event.event_key)?;
        let mut params = vec![
            SqliteValue::Text(&event.event_key),
            SqliteValue::Text(event.event_type.as_str()),
        ];
        push_common_params(&mut params, &event.meta)?;
        self.sqlite.execute_with_params(
            "INSERT OR IGNORE INTO feedback_events
             (event_key, event_type, schema_version, subject_type, subject_id, candidate_id,
              chat_guid, anchor_message_guid, diagnostics_trace_id, diagnostics_span_id,
              diagnostics_parent_span_id, diagnostics_chat_hash, diagnostics_message_hash,
              provider_model_prompt_version_id, source_excerpt_policy, label_source,
              privacy_tier, privacy_metadata_json, created_at, expires_at)
             VALUES (:p0, :p1, :p2, :p3, :p4, :p5, :p6, :p7, :p8, :p9, :p10,
              :p11, :p12, :p13, :p14, :p15, :p16, :p17, :p18, :p19);",
            &params,
        )
    }

    pub fn record_label(&self, label: Label) -> Result<(), StorageError> {
        validate_key("label_key", &label.label_key)?;
        let mut params = vec![
            SqliteValue::Text(&label.label_key),
            SqliteValue::Text(label.label_value.label_type().as_str()),
            SqliteValue::Text(label.label_value.as_str()),
        ];
        push_common_params(&mut params, &label.meta)?;
        self.sqlite.execute_with_params(
            "INSERT OR IGNORE INTO labels
             (label_key, label_type, label_value, schema_version, subject_type, subject_id,
              candidate_id, chat_guid, anchor_message_guid, diagnostics_trace_id,
              diagnostics_span_id, diagnostics_parent_span_id, diagnostics_chat_hash,
              diagnostics_message_hash, provider_model_prompt_version_id, source_excerpt_policy,
              label_source, privacy_tier, privacy_metadata_json, created_at, expires_at)
             VALUES (:p0, :p1, :p2, :p3, :p4, :p5, :p6, :p7, :p8, :p9, :p10,
              :p11, :p12, :p13, :p14, :p15, :p16, :p17, :p18, :p19, :p20);",
            &params,
        )
    }

    pub fn record_feature_snapshot(&self, snapshot: FeatureSnapshot) -> Result<(), StorageError> {
        validate_snapshot(&snapshot)?;
        let mut params = vec![SqliteValue::Text(&snapshot.snapshot_key)];
        push_common_body_params(&mut params, &snapshot.meta)?;
        params.extend([
            optional_text(snapshot.route.as_deref()),
            optional_text(snapshot.reason_code.as_deref()),
            optional_i64(snapshot.confidence_millis),
            optional_i64(snapshot.participant_count),
            optional_text(snapshot.tapback_signal.as_deref()),
            SqliteValue::Integer(bool_int(snapshot.sender_signal_available)),
            SqliteValue::Integer(bool_int(snapshot.context_window_available)),
            optional_text(snapshot.excerpt.as_deref()),
        ]);
        push_record_times(&mut params, &snapshot.meta);
        self.sqlite.execute_with_params(
            "INSERT OR IGNORE INTO feature_snapshots
             (snapshot_key, schema_version, subject_type, subject_id, candidate_id, chat_guid,
              anchor_message_guid, diagnostics_trace_id, diagnostics_span_id,
              diagnostics_parent_span_id, diagnostics_chat_hash, diagnostics_message_hash,
              provider_model_prompt_version_id, source_excerpt_policy, label_source,
              privacy_tier, privacy_metadata_json, route, reason_code, confidence_millis,
              participant_count, tapback_signal, sender_signal_available,
              context_window_available, excerpt, created_at, expires_at)
             VALUES (:p0, :p1, :p2, :p3, :p4, :p5, :p6, :p7, :p8, :p9, :p10,
              :p11, :p12, :p13, :p14, :p15, :p16, :p17, :p18, :p19, :p20,
              :p21, :p22, :p23, :p24, :p25, :p26);",
            &params,
        )
    }

    pub fn create_eval_run(&self, run: EvalRun) -> Result<i64, StorageError> {
        validate_eval_run(&run)?;
        let params = [
            SqliteValue::Text(&run.run_key),
            SqliteValue::Text(run.status.as_str()),
            SqliteValue::Integer(run.schema_version),
            SqliteValue::Integer(run.started_at),
            optional_i64(run.finished_at),
            optional_text(run.report_path.as_deref()),
            SqliteValue::Integer(run.cases_evaluated),
            SqliteValue::Integer(run.cases_skipped),
            SqliteValue::Text(&run.skip_reasons_json),
            SqliteValue::Integer(run.approval_kept_rate_millis),
            SqliteValue::Integer(run.observed_rejection_rate_millis),
            SqliteValue::Integer(run.quiet_log_count),
            SqliteValue::Integer(run.provider_failure_count),
            SqliteValue::Integer(run.accepted_visible_ratio_millis),
            SqliteValue::Text(&run.confusion_counts_json),
        ];
        self.sqlite.execute_with_params(
            "INSERT OR IGNORE INTO eval_runs
             (run_key, status, schema_version, started_at, finished_at, report_path,
              cases_evaluated, cases_skipped, skip_reasons_json, approval_kept_rate_millis,
              observed_rejection_rate_millis, quiet_log_count, provider_failure_count,
              accepted_visible_ratio_millis, confusion_counts_json)
             VALUES (:p0, :p1, :p2, :p3, :p4, :p5, :p6, :p7, :p8, :p9, :p10,
              :p11, :p12, :p13, :p14);",
            &params,
        )?;
        self.sqlite.query_scalar_i64_with_params(
            "SELECT id FROM eval_runs WHERE run_key = :p0;",
            &[SqliteValue::Text(&run.run_key)],
        )
    }

    pub fn record_eval_result(&self, result: EvalResult) -> Result<(), StorageError> {
        validate_eval_result(&result)?;
        let params = [
            SqliteValue::Text(&result.result_key),
            SqliteValue::Integer(result.eval_run_id),
            SqliteValue::Text(&result.snapshot_key),
            SqliteValue::Text(&result.label_key),
            SqliteValue::Text(result.expected_label_type.as_str()),
            SqliteValue::Text(&result.expected_label_value),
            optional_label_type(result.actual_label_type),
            optional_text(result.actual_label_value.as_deref()),
            SqliteValue::Text(result.outcome.as_str()),
            optional_text(result.skip_reason.as_deref()),
            SqliteValue::Integer(result.created_at),
        ];
        self.sqlite.execute_with_params(
            "INSERT OR IGNORE INTO eval_results
             (result_key, eval_run_id, snapshot_key, label_key, expected_label_type,
              expected_label_value, actual_label_type, actual_label_value, outcome,
              skip_reason, created_at)
             VALUES (:p0, :p1, :p2, :p3, :p4, :p5, :p6, :p7, :p8, :p9, :p10);",
            &params,
        )
    }

    pub fn feedback_eval_counts(&self) -> Result<FeedbackEvalCounts, StorageError> {
        let latest_eval_status = self
            .sqlite
            .query_first_column(
                "SELECT status FROM eval_runs
                 ORDER BY COALESCE(finished_at, started_at) DESC, id DESC
                 LIMIT 1;",
            )?
            .first()
            .map(|status| EvalRunStatus::parse(status))
            .transpose()?;
        Ok(FeedbackEvalCounts {
            label_count: self
                .sqlite
                .query_scalar_i64("SELECT COUNT(*) FROM labels;")?,
            feature_snapshot_count: self
                .sqlite
                .query_scalar_i64("SELECT COUNT(*) FROM feature_snapshots;")?,
            latest_eval_status,
        })
    }
}
