PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS _morrow_migrations (
    version INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    applied_at INTEGER NOT NULL
);

INSERT OR IGNORE INTO _morrow_migrations (version, name, applied_at)
VALUES (1, 'init', strftime('%s', 'now'));

CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value_json TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS whitelist_chats (
    chat_guid TEXT PRIMARY KEY,
    display_name TEXT,
    participant_count INTEGER NOT NULL,
    enabled INTEGER NOT NULL CHECK (enabled IN (0, 1)),
    group_generation INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS candidates (
    id TEXT PRIMARY KEY,
    kind TEXT NOT NULL,
    state TEXT NOT NULL,
    chat_guid TEXT NOT NULL,
    anchor_message_guid TEXT NOT NULL,
    title TEXT NOT NULL,
    confidence_millis INTEGER NOT NULL CHECK (confidence_millis BETWEEN 0 AND 1000),
    normalized_time TEXT NOT NULL,
    external_object_id TEXT,
    external_source_id TEXT,
    candidate_version INTEGER NOT NULL DEFAULT 1,
    current_reason TEXT NOT NULL DEFAULT 'created',
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    expires_at INTEGER,
    UNIQUE(kind, chat_guid, anchor_message_guid, normalized_time)
);

CREATE TABLE IF NOT EXISTS evidence (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    candidate_id TEXT NOT NULL REFERENCES candidates(id) ON DELETE CASCADE,
    message_guid TEXT NOT NULL,
    role TEXT NOT NULL,
    excerpt TEXT NOT NULL,
    message_timestamp INTEGER NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS audit_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    candidate_id TEXT NOT NULL REFERENCES candidates(id) ON DELETE CASCADE,
    from_state TEXT NOT NULL,
    to_state TEXT NOT NULL,
    reason TEXT NOT NULL,
    details TEXT,
    observed_external_object_id TEXT,
    created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS suppressions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    chat_guid TEXT NOT NULL,
    anchor_message_guid TEXT,
    candidate_id TEXT REFERENCES candidates(id) ON DELETE SET NULL,
    reason TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    expires_at INTEGER
);

CREATE TABLE IF NOT EXISTS quiet_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    chat_guid TEXT NOT NULL,
    anchor_message_guid TEXT NOT NULL,
    reason TEXT NOT NULL,
    excerpt TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS external_object_mappings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    candidate_id TEXT NOT NULL REFERENCES candidates(id) ON DELETE CASCADE,
    source TEXT NOT NULL,
    external_object_id TEXT NOT NULL,
    external_source_id TEXT NOT NULL,
    mapped_at INTEGER NOT NULL,
    UNIQUE(candidate_id, source),
    UNIQUE(source, external_object_id)
);

CREATE TABLE IF NOT EXISTS provider_model_prompt_versions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    provider TEXT NOT NULL,
    model TEXT NOT NULL,
    prompt_version TEXT NOT NULL,
    schema_version INTEGER NOT NULL,
    active INTEGER NOT NULL CHECK (active IN (0, 1)),
    created_at INTEGER NOT NULL,
    UNIQUE(provider, model, prompt_version, schema_version)
);

CREATE TABLE IF NOT EXISTS replay_cursors (
    stream TEXT PRIMARY KEY,
    cursor_value INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

INSERT OR IGNORE INTO replay_cursors (stream, cursor_value, updated_at)
VALUES ('calendar_proposals', 0, strftime('%s', 'now'));
