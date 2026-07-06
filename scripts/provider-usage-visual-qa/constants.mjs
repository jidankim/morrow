import os from "node:os"
import process from "node:process"

export const requiredStates = ["loaded", "empty", "error"]
export const viewportWidths = [375, 768, 1280]
export const screenshotPrefix = "provider-usage"
export const workspaceRoot = process.cwd()
export const evidenceRoot = ".omo/evidence/ai-provider-usage-dashboard"

export const bannedStrings = [
  "quiet_reason",
  "candidate_evidence_excerpt",
  "evidence_payload_hash",
  "reference_observed",
  "reference_timezone",
  "candidate_title",
  "provider_json",
  "provider request",
  "provider response",
  "message body",
  "access_token",
  "refresh_token",
  "cookie",
  "secret",
  "billing",
  "telemetry",
  "analytics",
  "raw prompt",
  "parser_time:2026-07-01T09:30:00",
  "[Asia/Seoul]",
  workspaceRoot,
  os.homedir()
]

export const bannedPatternRules = [
  { label: "raw parser_time route", pattern: /parser_time:[^\s<>"']+/u },
  { label: "bracketed IANA timezone", pattern: /\[[A-Za-z_]+\/[A-Za-z_]+\]/u },
  { label: "local absolute path", pattern: /(?:\/Users\/|\/home\/|[A-Za-z]:\\Users\\)/u },
  { label: "token-like secret", pattern: /(?:access_token|refresh_token|sk-[A-Za-z0-9_-]+)/u }
]
