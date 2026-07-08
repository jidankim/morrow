use crate::feedback_eval::{FeatureSnapshot, FeedbackEvent, Label};
use crate::sqlite_cli::SqliteValue;
use crate::store::feedback_eval_sql::{
    bool_int, optional_i64, optional_text, push_common_body_params, push_common_params,
    push_record_times,
};
use crate::store::feedback_eval_validation::{validate_key, validate_snapshot};
use crate::{StorageError, Store};

const EVENT_PARAM_COUNT: usize = 20;
const LABEL_PARAM_COUNT: usize = 21;
const SNAPSHOT_PARAM_COUNT: usize = 27;

impl Store {
    pub fn record_candidate_feedback_batch(
        &self,
        snapshot: FeatureSnapshot,
        event: FeedbackEvent,
        label: Label,
    ) -> Result<(), StorageError> {
        self.record_feedback_batch(&snapshot, &event, &[label])
    }

    pub fn record_quiet_feedback_batch(
        &self,
        snapshot: FeatureSnapshot,
        event: FeedbackEvent,
        labels: Vec<Label>,
    ) -> Result<(), StorageError> {
        self.record_feedback_batch(&snapshot, &event, &labels)
    }

    fn record_feedback_batch(
        &self,
        snapshot: &FeatureSnapshot,
        event: &FeedbackEvent,
        labels: &[Label],
    ) -> Result<(), StorageError> {
        let mut script = String::from(".bail on\nBEGIN IMMEDIATE;\n");
        let mut params = Vec::new();
        append_snapshot_insert(&mut script, &mut params, snapshot)?;
        append_event_insert(&mut script, &mut params, event)?;
        for label in labels {
            append_label_insert(&mut script, &mut params, label)?;
        }
        script.push_str("COMMIT;\n");
        self.sqlite.execute_with_params(&script, &params)
    }
}

fn append_event_insert<'a>(
    script: &mut String,
    params: &mut Vec<SqliteValue<'a>>,
    event: &'a FeedbackEvent,
) -> Result<(), StorageError> {
    validate_key("event_key", &event.event_key)?;
    let start = params.len();
    params.extend([
        SqliteValue::Text(&event.event_key),
        SqliteValue::Text(event.event_type.as_str()),
    ]);
    push_common_params(params, &event.meta)?;
    debug_assert_eq!(params.len() - start, EVENT_PARAM_COUNT);
    script.push_str(
        "INSERT OR IGNORE INTO feedback_events
         (event_key, event_type, schema_version, subject_type, subject_id, candidate_id,
          chat_guid, anchor_message_guid, diagnostics_trace_id, diagnostics_span_id,
          diagnostics_parent_span_id, diagnostics_chat_hash, diagnostics_message_hash,
          provider_model_prompt_version_id, source_excerpt_policy, label_source,
          privacy_tier, privacy_metadata_json, created_at, expires_at)
         VALUES ",
    );
    push_placeholders(script, start, EVENT_PARAM_COUNT);
    script.push_str(";\n");
    Ok(())
}

fn append_label_insert<'a>(
    script: &mut String,
    params: &mut Vec<SqliteValue<'a>>,
    label: &'a Label,
) -> Result<(), StorageError> {
    validate_key("label_key", &label.label_key)?;
    let start = params.len();
    params.extend([
        SqliteValue::Text(&label.label_key),
        SqliteValue::Text(label.label_value.label_type().as_str()),
        SqliteValue::Text(label.label_value.as_str()),
    ]);
    push_common_params(params, &label.meta)?;
    debug_assert_eq!(params.len() - start, LABEL_PARAM_COUNT);
    script.push_str(
        "INSERT OR IGNORE INTO labels
         (label_key, label_type, label_value, schema_version, subject_type, subject_id,
          candidate_id, chat_guid, anchor_message_guid, diagnostics_trace_id,
          diagnostics_span_id, diagnostics_parent_span_id, diagnostics_chat_hash,
          diagnostics_message_hash, provider_model_prompt_version_id, source_excerpt_policy,
          label_source, privacy_tier, privacy_metadata_json, created_at, expires_at)
         VALUES ",
    );
    push_placeholders(script, start, LABEL_PARAM_COUNT);
    script.push_str(";\n");
    Ok(())
}

fn append_snapshot_insert<'a>(
    script: &mut String,
    params: &mut Vec<SqliteValue<'a>>,
    snapshot: &'a FeatureSnapshot,
) -> Result<(), StorageError> {
    validate_snapshot(snapshot)?;
    let start = params.len();
    params.push(SqliteValue::Text(&snapshot.snapshot_key));
    push_common_body_params(params, &snapshot.meta)?;
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
    push_record_times(params, &snapshot.meta);
    debug_assert_eq!(params.len() - start, SNAPSHOT_PARAM_COUNT);
    script.push_str(
        "INSERT OR IGNORE INTO feature_snapshots
         (snapshot_key, schema_version, subject_type, subject_id, candidate_id, chat_guid,
          anchor_message_guid, diagnostics_trace_id, diagnostics_span_id,
          diagnostics_parent_span_id, diagnostics_chat_hash, diagnostics_message_hash,
          provider_model_prompt_version_id, source_excerpt_policy, label_source,
          privacy_tier, privacy_metadata_json, route, reason_code, confidence_millis,
          participant_count, tapback_signal, sender_signal_available,
          context_window_available, excerpt, created_at, expires_at)
         VALUES ",
    );
    push_placeholders(script, start, SNAPSHOT_PARAM_COUNT);
    script.push_str(";\n");
    Ok(())
}

fn push_placeholders(script: &mut String, start: usize, count: usize) {
    script.push('(');
    for index in 0..count {
        if index > 0 {
            script.push_str(", ");
        }
        script.push_str(":p");
        script.push_str(&(start + index).to_string());
    }
    script.push(')');
}
