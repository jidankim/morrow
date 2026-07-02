CREATE TABLE sync_scheduler_state_next (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    enabled INTEGER NOT NULL CHECK (enabled IN (0, 1)),
    interval_seconds INTEGER NOT NULL CHECK (interval_seconds >= 60 AND interval_seconds % 60 = 0),
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

INSERT INTO sync_scheduler_state_next
    (id, enabled, interval_seconds, status, last_started_at, last_finished_at,
     next_run_at, next_eligible_at, last_result, retry_attempt, last_reason, updated_at)
SELECT
    id, enabled, interval_seconds, status, last_started_at, last_finished_at,
    next_run_at, next_eligible_at, last_result, retry_attempt, last_reason, updated_at
FROM sync_scheduler_state;

DROP TABLE sync_scheduler_state;

ALTER TABLE sync_scheduler_state_next RENAME TO sync_scheduler_state;
