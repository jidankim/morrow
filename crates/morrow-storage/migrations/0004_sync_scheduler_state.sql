CREATE TABLE IF NOT EXISTS sync_scheduler_state (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    enabled INTEGER NOT NULL CHECK (enabled IN (0, 1)),
    interval_seconds INTEGER NOT NULL CHECK (interval_seconds IN (900, 1800, 3600)),
    status TEXT NOT NULL CHECK (status IN ('disabled', 'scheduled', 'running', 'cooldown', 'blocked')),
    last_started_at INTEGER,
    last_finished_at INTEGER,
    next_run_at INTEGER,
    next_eligible_at INTEGER,
    last_result TEXT CHECK (
        last_result IS NULL
        OR last_result IN ('success', 'retryable_failure', 'blocked', 'manual_disabled')
    ),
    retry_attempt INTEGER NOT NULL CHECK (retry_attempt >= 0),
    last_reason TEXT,
    updated_at INTEGER NOT NULL
);

INSERT OR IGNORE INTO sync_scheduler_state
    (id, enabled, interval_seconds, status, retry_attempt, updated_at)
VALUES
    (1, 0, 1800, 'disabled', 0, 0);
