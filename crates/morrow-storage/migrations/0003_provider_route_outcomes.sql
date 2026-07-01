CREATE TABLE IF NOT EXISTS provider_route_outcomes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    route_fingerprint TEXT NOT NULL UNIQUE,
    provider_route_contract_version TEXT NOT NULL,
    provider_candidate_schema_version TEXT NOT NULL,
    evidence_payload_hash TEXT NOT NULL,
    provider_id TEXT NOT NULL,
    model_id TEXT NOT NULL,
    prompt_version TEXT NOT NULL,
    source_excerpt_policy TEXT NOT NULL CHECK (source_excerpt_policy IN ('include', 'hide')),
    reference_observed TEXT NOT NULL,
    reference_timezone TEXT NOT NULL,
    threshold_millis INTEGER NOT NULL CHECK (threshold_millis BETWEEN 0 AND 1000),
    parser_route TEXT NOT NULL,
    outcome_kind TEXT NOT NULL CHECK (outcome_kind IN ('candidate', 'quiet')),
    candidate_kind TEXT,
    candidate_title TEXT CHECK (
        candidate_title IS NULL OR candidate_title = 'Messages event candidate'
    ),
    candidate_confidence_millis INTEGER CHECK (
        candidate_confidence_millis IS NULL
        OR candidate_confidence_millis BETWEEN 0 AND 1000
    ),
    candidate_normalized_time TEXT,
    candidate_evidence_excerpt TEXT,
    quiet_reason TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    CHECK (
        (
            outcome_kind = 'candidate'
            AND candidate_kind IS NOT NULL
            AND candidate_title IS NOT NULL
            AND candidate_confidence_millis IS NOT NULL
            AND candidate_normalized_time IS NOT NULL
            AND candidate_evidence_excerpt IS NOT NULL
            AND quiet_reason IS NULL
        )
        OR (
            outcome_kind = 'quiet'
            AND candidate_kind IS NULL
            AND candidate_title IS NULL
            AND candidate_confidence_millis IS NULL
            AND candidate_normalized_time IS NULL
            AND candidate_evidence_excerpt IS NULL
            AND quiet_reason IS NOT NULL
        )
    )
);
