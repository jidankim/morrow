CREATE TABLE IF NOT EXISTS list_intake_sender_labels (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    profile_id TEXT NOT NULL,
    window_start_unix_seconds INTEGER NOT NULL,
    chat_key TEXT NOT NULL,
    sender_key TEXT,
    sender_key_bucket TEXT NOT NULL,
    sender_label TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE(profile_id, window_start_unix_seconds, chat_key, sender_key_bucket)
);

CREATE TABLE IF NOT EXISTS list_intake_proposals (
    proposal_id TEXT PRIMARY KEY,
    idempotency_key TEXT NOT NULL UNIQUE,
    status TEXT NOT NULL CHECK (status IN ('pending', 'approved', 'rejected')),
    profile_id TEXT NOT NULL,
    profile_version TEXT NOT NULL,
    examples_hash TEXT NOT NULL,
    message_guid TEXT NOT NULL,
    evidence_pointer TEXT NOT NULL,
    chat_key TEXT NOT NULL,
    sender_key TEXT,
    sender_label_id INTEGER NOT NULL REFERENCES list_intake_sender_labels(id) ON DELETE RESTRICT,
    sender_label TEXT NOT NULL,
    window_local_date TEXT NOT NULL,
    window_timezone TEXT NOT NULL,
    window_start_unix_seconds INTEGER NOT NULL,
    source_item_index INTEGER NOT NULL CHECK (source_item_index >= 0),
    quantity INTEGER NOT NULL CHECK (quantity > 0),
    item_name TEXT NOT NULL,
    unit TEXT,
    category_id TEXT NOT NULL,
    confidence_tier TEXT NOT NULL CHECK (confidence_tier IN ('review')),
    confidence_millis INTEGER NOT NULL CHECK (confidence_millis BETWEEN 0 AND 1000),
    decided_at INTEGER,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS list_intake_proposal_items (
    item_id TEXT PRIMARY KEY,
    idempotency_key TEXT NOT NULL UNIQUE,
    proposal_id TEXT NOT NULL REFERENCES list_intake_proposals(proposal_id) ON DELETE CASCADE,
    source_item_index INTEGER NOT NULL CHECK (source_item_index >= 0),
    quantity INTEGER NOT NULL CHECK (quantity > 0),
    item_name TEXT NOT NULL,
    unit TEXT,
    category_id TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE(proposal_id, source_item_index)
);

CREATE TABLE IF NOT EXISTS list_intake_entries (
    entry_id TEXT PRIMARY KEY,
    idempotency_key TEXT NOT NULL UNIQUE,
    profile_id TEXT NOT NULL,
    profile_version TEXT NOT NULL,
    examples_hash TEXT NOT NULL,
    source_proposal_id TEXT REFERENCES list_intake_proposals(proposal_id) ON DELETE SET NULL,
    message_guid TEXT NOT NULL,
    evidence_pointer TEXT NOT NULL,
    chat_key TEXT NOT NULL,
    sender_key TEXT,
    sender_label_id INTEGER NOT NULL REFERENCES list_intake_sender_labels(id) ON DELETE RESTRICT,
    sender_label TEXT NOT NULL,
    window_local_date TEXT NOT NULL,
    window_timezone TEXT NOT NULL,
    window_start_unix_seconds INTEGER NOT NULL,
    source_item_index INTEGER NOT NULL CHECK (source_item_index >= 0),
    quantity INTEGER NOT NULL CHECK (quantity > 0),
    item_name TEXT NOT NULL,
    unit TEXT,
    category_id TEXT NOT NULL,
    confidence_tier TEXT NOT NULL CHECK (confidence_tier IN ('auto_aggregate', 'review')),
    confidence_millis INTEGER NOT NULL CHECK (confidence_millis BETWEEN 0 AND 1000),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_list_intake_entries_aggregate
ON list_intake_entries (
    profile_id,
    window_local_date,
    chat_key,
    sender_label,
    category_id,
    item_name,
    unit
);

CREATE INDEX IF NOT EXISTS idx_list_intake_proposals_profile
ON list_intake_proposals (profile_id, status, created_at, proposal_id);

CREATE INDEX IF NOT EXISTS idx_list_intake_proposal_items_proposal
ON list_intake_proposal_items (proposal_id, source_item_index);
