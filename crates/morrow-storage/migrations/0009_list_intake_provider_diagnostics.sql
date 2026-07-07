CREATE TABLE IF NOT EXISTS list_intake_provider_diagnostics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    diagnostic_key TEXT NOT NULL UNIQUE,
    profile_id TEXT NOT NULL,
    profile_version TEXT NOT NULL,
    examples_hash TEXT NOT NULL,
    message_hash TEXT NOT NULL,
    chat_key TEXT NOT NULL,
    provider_prompt_version TEXT NOT NULL,
    provider_schema_version TEXT NOT NULL,
    reason_code TEXT NOT NULL,
    retry_state TEXT NOT NULL CHECK (retry_state IN ('retry_available')),
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_list_intake_provider_diagnostics_profile
ON list_intake_provider_diagnostics (profile_id, created_at);
