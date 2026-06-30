CREATE TABLE IF NOT EXISTS feedback_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_key TEXT NOT NULL UNIQUE,
    event_type TEXT NOT NULL CHECK (event_type IN (
        'candidate_visible',
        'approved_by_move',
        'approved_by_copy',
        'rejected_by_delete',
        'pending_edited',
        'proposed_reminder_completed_resolved',
        'unknown_disappearance',
        'external_creation_failed',
        'deterministic_stop:unsupported_broad_content',
        'deterministic_stop:invalid_evidence',
        'deterministic_stop:no_scheduling_signal',
        'deterministic_stop:past_or_invalid_time',
        'confidence_below_threshold',
        'provider_unavailable',
        'provider_invalid_json',
        'provider_schema_rejected',
        'provider_hallucinated_evidence',
        'parser_provider_time_conflict'
    )),
    schema_version INTEGER NOT NULL,
    subject_type TEXT NOT NULL CHECK (subject_type IN ('candidate', 'quiet_log', 'eval_case')),
    subject_id TEXT NOT NULL,
    candidate_id TEXT REFERENCES candidates(id) ON DELETE SET NULL,
    chat_guid TEXT NOT NULL,
    anchor_message_guid TEXT NOT NULL,
    diagnostics_trace_id TEXT,
    diagnostics_span_id TEXT,
    diagnostics_parent_span_id TEXT,
    diagnostics_chat_hash TEXT,
    diagnostics_message_hash TEXT,
    provider_model_prompt_version_id INTEGER REFERENCES provider_model_prompt_versions(id) ON DELETE SET NULL,
    source_excerpt_policy TEXT NOT NULL CHECK (source_excerpt_policy IN ('include', 'hide')),
    label_source TEXT NOT NULL CHECK (label_source IN (
        'lifecycle',
        'quiet_log',
        'provider',
        'eval_runner',
        'manual_alpha'
    )),
    privacy_tier TEXT NOT NULL CHECK (privacy_tier IN (
        'internal_metadata',
        'hashed_identifier',
        'local_private'
    )),
    privacy_metadata_json TEXT NOT NULL DEFAULT '{}',
    created_at INTEGER NOT NULL,
    expires_at INTEGER
);

CREATE TABLE IF NOT EXISTS labels (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    label_key TEXT NOT NULL UNIQUE,
    label_type TEXT NOT NULL CHECK (label_type IN (
        'proposal_outcome',
        'detection_route',
        'field_quality',
        'system_outcome'
    )),
    label_value TEXT NOT NULL,
    schema_version INTEGER NOT NULL,
    subject_type TEXT NOT NULL CHECK (subject_type IN ('candidate', 'quiet_log', 'eval_case')),
    subject_id TEXT NOT NULL,
    candidate_id TEXT REFERENCES candidates(id) ON DELETE SET NULL,
    chat_guid TEXT NOT NULL,
    anchor_message_guid TEXT NOT NULL,
    diagnostics_trace_id TEXT,
    diagnostics_span_id TEXT,
    diagnostics_parent_span_id TEXT,
    diagnostics_chat_hash TEXT,
    diagnostics_message_hash TEXT,
    provider_model_prompt_version_id INTEGER REFERENCES provider_model_prompt_versions(id) ON DELETE SET NULL,
    source_excerpt_policy TEXT NOT NULL CHECK (source_excerpt_policy IN ('include', 'hide')),
    label_source TEXT NOT NULL CHECK (label_source IN (
        'lifecycle',
        'quiet_log',
        'provider',
        'eval_runner',
        'manual_alpha'
    )),
    privacy_tier TEXT NOT NULL CHECK (privacy_tier IN (
        'internal_metadata',
        'hashed_identifier',
        'local_private'
    )),
    privacy_metadata_json TEXT NOT NULL DEFAULT '{}',
    created_at INTEGER NOT NULL,
    expires_at INTEGER,
    CHECK (
        (label_type = 'proposal_outcome' AND label_value IN (
            'accepted',
            'rejected_observed',
            'pending_edited',
            'unknown'
        ))
        OR (label_type = 'detection_route' AND label_value IN (
            'deterministic_candidate',
            'provider_candidate',
            'quiet_stop',
            'provider_rejected',
            'provider_unavailable'
        ))
        OR (label_type = 'field_quality' AND label_value IN (
            'unknown',
            'title_edited',
            'time_edited',
            'kind_edited'
        ))
        OR (label_type = 'system_outcome' AND label_value IN (
            'ok',
            'failed_external_creation',
            'failed_provider',
            'failed_validation'
        ))
    )
);

CREATE TABLE IF NOT EXISTS feature_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    snapshot_key TEXT NOT NULL UNIQUE,
    schema_version INTEGER NOT NULL,
    subject_type TEXT NOT NULL CHECK (subject_type IN ('candidate', 'quiet_log', 'eval_case')),
    subject_id TEXT NOT NULL,
    candidate_id TEXT REFERENCES candidates(id) ON DELETE SET NULL,
    chat_guid TEXT NOT NULL,
    anchor_message_guid TEXT NOT NULL,
    diagnostics_trace_id TEXT,
    diagnostics_span_id TEXT,
    diagnostics_parent_span_id TEXT,
    diagnostics_chat_hash TEXT,
    diagnostics_message_hash TEXT,
    provider_model_prompt_version_id INTEGER REFERENCES provider_model_prompt_versions(id) ON DELETE SET NULL,
    source_excerpt_policy TEXT NOT NULL CHECK (source_excerpt_policy IN ('include', 'hide')),
    label_source TEXT NOT NULL CHECK (label_source IN (
        'lifecycle',
        'quiet_log',
        'provider',
        'eval_runner',
        'manual_alpha'
    )),
    privacy_tier TEXT NOT NULL CHECK (privacy_tier IN (
        'internal_metadata',
        'hashed_identifier',
        'local_private'
    )),
    privacy_metadata_json TEXT NOT NULL DEFAULT '{}',
    route TEXT,
    reason_code TEXT,
    confidence_millis INTEGER CHECK (confidence_millis IS NULL OR confidence_millis BETWEEN 0 AND 1000),
    participant_count INTEGER CHECK (participant_count IS NULL OR participant_count >= 0),
    tapback_signal TEXT,
    sender_signal_available INTEGER NOT NULL DEFAULT 0 CHECK (sender_signal_available IN (0, 1)),
    context_window_available INTEGER NOT NULL DEFAULT 0 CHECK (context_window_available IN (0, 1)),
    excerpt TEXT,
    created_at INTEGER NOT NULL,
    expires_at INTEGER
);

CREATE TABLE IF NOT EXISTS eval_runs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    run_key TEXT NOT NULL UNIQUE,
    status TEXT NOT NULL CHECK (status IN ('passed', 'needs_review', 'failed')),
    schema_version INTEGER NOT NULL,
    started_at INTEGER NOT NULL,
    finished_at INTEGER,
    report_path TEXT,
    cases_evaluated INTEGER NOT NULL DEFAULT 0 CHECK (cases_evaluated >= 0),
    cases_skipped INTEGER NOT NULL DEFAULT 0 CHECK (cases_skipped >= 0),
    skip_reasons_json TEXT NOT NULL DEFAULT '{}',
    approval_kept_rate_millis INTEGER NOT NULL DEFAULT 0 CHECK (approval_kept_rate_millis BETWEEN 0 AND 1000),
    observed_rejection_rate_millis INTEGER NOT NULL DEFAULT 0 CHECK (observed_rejection_rate_millis BETWEEN 0 AND 1000),
    quiet_log_count INTEGER NOT NULL DEFAULT 0 CHECK (quiet_log_count >= 0),
    provider_failure_count INTEGER NOT NULL DEFAULT 0 CHECK (provider_failure_count >= 0),
    accepted_visible_ratio_millis INTEGER NOT NULL DEFAULT 0 CHECK (accepted_visible_ratio_millis BETWEEN 0 AND 1000),
    confusion_counts_json TEXT NOT NULL DEFAULT '{}'
);

CREATE TABLE IF NOT EXISTS eval_results (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    result_key TEXT NOT NULL UNIQUE,
    eval_run_id INTEGER NOT NULL REFERENCES eval_runs(id) ON DELETE CASCADE,
    snapshot_key TEXT NOT NULL REFERENCES feature_snapshots(snapshot_key) ON DELETE CASCADE,
    label_key TEXT NOT NULL REFERENCES labels(label_key) ON DELETE CASCADE,
    expected_label_type TEXT NOT NULL,
    expected_label_value TEXT NOT NULL,
    actual_label_type TEXT,
    actual_label_value TEXT,
    outcome TEXT NOT NULL CHECK (outcome IN ('match', 'mismatch', 'skipped')),
    skip_reason TEXT,
    created_at INTEGER NOT NULL
);
