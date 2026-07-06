#[derive(Debug)]
pub(super) struct LiveSurfaceAttempt {
    surface: &'static str,
    target_hint: &'static str,
}

#[derive(Debug)]
pub(super) struct LiveSurfaceBlocked {
    pub(super) surface: &'static str,
    pub(super) boundary: &'static str,
    pub(super) sanitized_target: &'static str,
    pub(super) reason: &'static str,
}

impl LiveSurfaceAttempt {
    pub(super) const fn messages_sqlite_probe() -> Self {
        Self {
            surface: "messages_sqlite",
            target_hint: "/Users/example/Library/Messages/chat.db",
        }
    }
}

pub(super) fn attempt_test_only_live_surface(
    attempt: LiveSurfaceAttempt,
) -> Result<(), LiveSurfaceBlocked> {
    let sanitized_target = if attempt.target_hint.starts_with("/Users/")
        || attempt.target_hint.starts_with("/private/")
    {
        "[REDACTED_LOCAL_PATH]"
    } else {
        attempt.target_hint
    };
    Err(LiveSurfaceBlocked {
        surface: attempt.surface,
        boundary: "local_runner_live_surface_guard",
        sanitized_target,
        reason: "local trajectory runner forbids Messages SQLite/provider/network/EventKit access",
    })
}
