# Sans-IO Boundary Map

Status: Tasks 1-9 have been independently confirmed in start-work evidence. Final F1-F4/global verification is still pending, so this note records the current boundary shape but does not claim the whole plan complete.

## Topology

| ID | Functional core | Imperative adapter | Do not move into core |
| --- | --- | --- | --- |
| `C1-ui-shell-contracts` | TypeScript state contracts, zod parsing, sync result count parsing, and command request shaping. | `NativeShellBridge`, Tauri runtime detection, `invoke`/`listen`, and UI state subscriptions. | Tauri invoke/listen, browser runtime checks, React/UI state runtime effects, menu event subscriptions, token/keychain command effects. |
| `C2-scan-use-case-core` | Scan outcome planning, privacy-safe persistence intents, cap/replay selection, request/config derivation, and result count mapping. | `scan_selected_chats_with_dependencies` opens storage, ingests Messages, runs provider-backed detection, applies caps, persists outcomes, replays proposals, and reads feedback counts. | Messages reads, provider calls, `Store::open`, sqlite persistence, proposal adapter calls, filesystem/app paths, and native error plumbing. |
| `C3-messages-adapters` | `morrow-messages` typed values: `ChatGuid`, `MessageGuid`, `NativeBatch`, `MessageEvidence`, ingestion reports, and `MessagesDataSource` trait contract. | Messages SQLite/native adapters implement discovery and reads with protected DB access and row parsing. | Messages SQLite/protected DB access, sqlite CLI invocation, Full Disk Access handling, SQL strings, native permission mapping, and raw macOS Messages paths. |
| `C4-detection-provider-boundary` | Detection parser/pipeline contracts, provider request/response types, schema validation, and fake-provider-testable extraction flow. | OpenAI and Codex providers supply transport, credentials, payload delivery, subprocess/network behavior, and response retrieval. | OpenAI/Codex transport, API keys/keychain, network, subprocess runners, model/prompt policy, timeout/auth behavior, and token redaction surfaces. |
| `C5-reconcile-proposal-replay` | `reconcile_candidate` planning and proposal replay decision helpers for mapping-present/create/finalize/failure transitions. | EventKit/Reminders proposal adapters, external object creation, store transitions, recovery marking, and failure persistence. | EventKit/Reminders calls, external calendar/reminder mutation, `Store` state transitions, durable mapping writes, and native proposal adapter details. |
| `C6-storage-durability-edge` | Typed persistence intents and storage-facing decisions that can be tested without opening a database. | `Store`, sqlite CLI, migrations, visibility caps, feedback eval counts, quiet logs, candidate writes, and durable external mappings. | sqlite CLI/Store persistence, schema/migration/backend choices, DB file paths, app data paths, durability checks, and cleanup/delete effects. |

## Boundary Rules

- Keep the Functional core deterministic: given typed input, it returns typed decisions, plans, counts, reports, or validation errors without touching native services.
- Keep each Imperative adapter thin: parse at the edge, call the native service or persistence layer, convert failures into existing error contracts, and delegate decisions back to the core.
- Do not move into core: Tauri invoke/listen, EventKit/Reminders, Messages SQLite/protected DB access, sqlite CLI/Store persistence, OpenAI/Codex transport/keychain/network/subprocess, filesystem/app paths, or UI state runtime effects.
- Future work must preserve current Tauri command names, zod schemas, database schema, provider model/prompt behavior, privacy behavior, and user-visible UI behavior unless explicitly scoped.

## Phase 4 Evidence Boundary

The local Phase 4 approval/correction evidence loop is covered by local smoke artifacts, not by new native adapters or UI runtime behavior. The smoke command is:

```bash
scripts/run-human-approval-correction-smoke.sh --out-dir .omo/evidence/phase-4-human-approval-correction/final-smoke --assert-canary-rejection
```

The bounded real-surface receipt command is:

```bash
scripts/run-lifecycle-real-surface-qa.sh --out-dir .omo/evidence/phase-4-human-approval-correction/real-surface
```

The evidence boundary is:

- The smoke may prove sanitized local approval/correction trace, storage, and decision-evidence receipts under `.omo/evidence/phase-4-human-approval-correction/final-smoke/summary.txt`, `.omo/evidence/phase-4-human-approval-correction/final-smoke/human-approval-correction-report.json`, `.omo/evidence/phase-4-human-approval-correction/final-smoke/trace.jsonl`, `.omo/evidence/phase-4-human-approval-correction/final-smoke/storage-readback.json`, `.omo/evidence/phase-4-human-approval-correction/final-smoke/decision-evidence.json`, `.omo/evidence/phase-4-human-approval-correction/final-smoke/privacy-inspect.txt`, `.omo/evidence/phase-4-human-approval-correction/final-smoke/canary-rejection.txt`, and `.omo/evidence/phase-4-human-approval-correction/final-smoke/cleanup-receipt.txt`.
- It must not imply EventKit/Reminders mutation, Messages access, provider calls, cloud upload, or user-facing correction interaction; the smoke runs with no live network, vendor, Messages, Calendar, or Reminders access.
- The correction UI remains future work, cloud telemetry remains future work, and the full live Messages-to-Calendar approval trajectory eval remains future work.
- Real-surface status remains PASS or sanitized BLOCKED and is recorded separately under `.omo/evidence/phase-4-human-approval-correction/real-surface/summary.txt` and `.omo/evidence/phase-4-human-approval-correction/real-surface/cleanup-receipt.txt`.

## Phase 5 Evidence Boundary

The local Phase 5 trajectory eval hardening is covered by local smoke artifacts, not by new native adapters, cloud telemetry, deployed rollout, or UI runtime behavior. The smoke command is:

```bash
scripts/run-messages-calendar-approval-trajectory-eval-smoke.sh --out-dir .omo/evidence/phase-5-messages-calendar-approval-trajectory-eval/final-smoke --assert-canary-rejection
```

The bounded live receipt command is:

```bash
scripts/run-messages-calendar-approval-live-receipt.sh --out-dir .omo/evidence/phase-5-messages-calendar-approval-trajectory-eval/live-receipt
```

The evidence boundary is:

- The smoke may prove sanitized local trajectory trace, storage, decision-evidence, scoring, collateral-damage, replay, privacy, canary rejection, and cleanup receipts under `.omo/evidence/phase-5-messages-calendar-approval-trajectory-eval/final-smoke/summary.txt`, `.omo/evidence/phase-5-messages-calendar-approval-trajectory-eval/final-smoke/trajectory-report.json`, `.omo/evidence/phase-5-messages-calendar-approval-trajectory-eval/final-smoke/trace.jsonl`, `.omo/evidence/phase-5-messages-calendar-approval-trajectory-eval/final-smoke/storage-readback.json`, `.omo/evidence/phase-5-messages-calendar-approval-trajectory-eval/final-smoke/decision-evidence.json`, `.omo/evidence/phase-5-messages-calendar-approval-trajectory-eval/final-smoke/privacy-inspect.txt`, `.omo/evidence/phase-5-messages-calendar-approval-trajectory-eval/final-smoke/canary-rejection.txt`, and `.omo/evidence/phase-5-messages-calendar-approval-trajectory-eval/final-smoke/cleanup-receipt.txt`.
- It must not imply EventKit/Reminders mutation, Messages access, provider calls, cloud upload, deployed rollout, approval queue, auto-approval, or user-facing correction interaction; the backend observable is no live backend, provider network, EventKit, Messages, Calendar, Reminders, Phoenix, Langfuse, or vendor backend.
- The correction UI remains future work, cloud telemetry remains future work, deployed rollout remains future work, and the full live Messages-to-Calendar approval trajectory eval remains future work unless the live receipt summary is PASS.
- Live receipt status remains PASS or sanitized BLOCKED and is recorded separately under `.omo/evidence/phase-5-messages-calendar-approval-trajectory-eval/live-receipt/summary.txt`, `.omo/evidence/phase-5-messages-calendar-approval-trajectory-eval/live-receipt/cleanup-receipt.txt`, and `.omo/evidence/phase-5-messages-calendar-approval-trajectory-eval/live-receipt/privacy-inspect.txt`.
- A full live PASS requires explicit surface QA and mutation opt-ins, the selected real QA chat environment, the real Messages-to-Calendar QA runner, and a structured manual proof file for approval/reconcile/idempotency observations. Fixture receipts cannot set `full_live_claim_allowed=true`.

## Phase 6 Evidence Boundary

Phase 6 cloud eval and monitoring is covered by smoke artifacts, not by new native adapters, deployed rollout, correction UI, full live Messages-to-Calendar approval completion, or automatic release promotion. The smoke command is:

```bash
scripts/run-cloud-eval-monitoring-smoke.sh --out-dir .omo/evidence/phase-6-cloud-eval-monitoring/final-smoke --assert-canary-rejection
```

The evidence boundary is:

- The smoke may prove deterministic fixture-backed preflight, sanitized cloud eval monitoring inputs, eval job aggregation, eval slices, metrics, dashboard output, release-gate decisions, alerts, privacy inspection, canary rejection, negative matrix coverage, artifact freshness, and cleanup under `.omo/evidence/phase-6-cloud-eval-monitoring/final-smoke/summary.txt`, `.omo/evidence/phase-6-cloud-eval-monitoring/final-smoke/preflight-report.json`, `.omo/evidence/phase-6-cloud-eval-monitoring/final-smoke/eval-job-input.json`, `.omo/evidence/phase-6-cloud-eval-monitoring/final-smoke/eval-slices.json`, `.omo/evidence/phase-6-cloud-eval-monitoring/final-smoke/metrics.json`, `.omo/evidence/phase-6-cloud-eval-monitoring/final-smoke/cloud-eval-monitoring-report.json`, `.omo/evidence/phase-6-cloud-eval-monitoring/final-smoke/cloud-eval-monitoring-dashboard.md`, `.omo/evidence/phase-6-cloud-eval-monitoring/final-smoke/dashboard-summary.txt`, `.omo/evidence/phase-6-cloud-eval-monitoring/final-smoke/release-gate.json`, `.omo/evidence/phase-6-cloud-eval-monitoring/final-smoke/release-gate.md`, `.omo/evidence/phase-6-cloud-eval-monitoring/final-smoke/alerts.json`, `.omo/evidence/phase-6-cloud-eval-monitoring/final-smoke/privacy-inspect.txt`, `.omo/evidence/phase-6-cloud-eval-monitoring/final-smoke/canary-rejection.txt`, `.omo/evidence/phase-6-cloud-eval-monitoring/final-smoke/negative-matrix.json`, `.omo/evidence/phase-6-cloud-eval-monitoring/final-smoke/cleanup-receipt.txt`, and `.omo/evidence/phase-6-cloud-eval-monitoring/final-smoke/command-logs/`.
- It must not imply EventKit/Reminders mutation, Messages access, provider calls, deployed rollout, automatic release promotion, approval queue completion, cloud upload, or user-facing correction interaction; the backend observable is no live backend, provider network, EventKit, Messages, Calendar, Reminders, Phoenix, Langfuse, vendor backend, deployment, or cloud upload surface.
- The correction UI remains future work, deployed rollout remains future work, automatic release promotion remains future work, and the full live Messages-to-Calendar approval trajectory eval remains future work.
