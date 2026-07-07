export const ALLOWED_STATES = new Set([
  "beta",
  "staged",
  "default_available",
  "paused",
  "rolled_back",
  "blocked",
]);

export const FAIL_CLOSED_STATES = new Set(["paused", "rolled_back", "blocked"]);
export const ALLOWED_TRANSITIONS = new Set(["beta->staged", "staged->default_available"]);

export const REQUIRED_KILL_SWITCHES = [
  "telemetry",
  "upload",
  "remote_config",
  "staged_promotion",
  "default_availability",
  "model_router_changes",
  "provider_changes",
];

export const OUT_OF_SCOPE_MODES = new Set([
]);
