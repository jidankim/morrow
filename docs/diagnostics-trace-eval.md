# Diagnostics Trace/Eval Runbook

Morrow diagnostics are local private artifacts. They are not remote telemetry and do not require Phoenix, Langfuse, LangSmith, Braintrust, LiteLLM, Helicone, or any vendor backend.

## Local Layout

Diagnostics live beside the app data store:

- `<app-data>/diagnostics/traces/`: sanitized JSONL trace records. One record is one span-shaped decision event.
- `<app-data>/diagnostics/evals/`: local eval summaries and comparable experiment receipts.
- `<app-data>/diagnostics/exports/`: sanitized downstream viewer/importer payload captures.

The trace JSONL is the source of truth. Viewer/importer payloads are derived from it.

## Phase 2 Product Correlation Smoke

Phase 2 product correlation is implemented for local candidate and quiet summaries, including retained and deleted diagnostics states. It correlates durable local feedback/eval rows and retained sanitized diagnostics traces back to the product decision-evidence surface without introducing cloud telemetry or a second canonical trace store.

Run the Phase 2 smoke with local fake fixtures:

```bash
scripts/run-trace-candidate-correlation-smoke.sh --out-dir .omo/evidence/phase-2-trace-candidate-correlation/final-smoke --assert-canary-rejection
```

Expected artifacts:

- `.omo/evidence/phase-2-trace-candidate-correlation/final-smoke/summary.txt`
- `.omo/evidence/phase-2-trace-candidate-correlation/final-smoke/decision-evidence.json`
- `.omo/evidence/phase-2-trace-candidate-correlation/final-smoke/privacy-inspect.txt`
- `.omo/evidence/phase-2-trace-candidate-correlation/final-smoke/canary-rejection.txt`
- `.omo/evidence/phase-2-trace-candidate-correlation/final-smoke/cargo-decision-evidence.txt`
- `.omo/evidence/phase-2-trace-candidate-correlation/final-smoke/npm-decision-evidence.txt`
- `.omo/evidence/phase-2-trace-candidate-correlation/final-smoke/cleanup-receipt.txt`

The evidence receipt for the completed smoke is `.omo/evidence/phase-2-trace-candidate-correlation/final-smoke/summary.txt`.

Privacy rules for this phase:

- Correlation remains local-only and privacy-safe for candidate/quiet summaries and retained/deleted diagnostics states.
- Raw private content is not exported by the local diagnostics evidence flow.
- No raw prompt text, raw source content, provider JSON, native identifiers, or app-data paths are exposed in docs, UI, reports, logs, traces, exports, or evidence.
- Deleted, disabled, expired, missing, or unavailable diagnostics remain nonfatal trace-retention states rather than sync failures.

Remaining gaps:

- Phase 4 human approval/correction remains future work, including risk gates, correction capture, and auditable approval outcomes.
- Phase 5 trajectory-level eval hardening remains future work for multi-step Messages-to-Calendar approval paths, collateral-damage checks, and replay scoring.
- Cloud telemetry rollout remains future work; Phase 2 does not upload diagnostics or require Phoenix, Langfuse, LangSmith, Braintrust, LiteLLM, Helicone, or any vendor backend.

## Phase 3 Lifecycle Replay Coverage Smoke

Phase 3 lifecycle coverage is now backed by local smoke artifacts and real-surface PASS/BLOCKED receipts. It covers deterministic local evidence for superseded, rescheduled, cancelled, dry-run, commit-idempotent, and replay-run outcomes. The full Calendar write workflow is not claimed. A human approval workflow is not claimed. Cloud telemetry is not claimed. Trajectory-level Messages-to-Calendar approval eval remains future work.

Run the Phase 3 local smoke with:

```bash
scripts/run-lifecycle-replay-coverage-smoke.sh --out-dir .omo/evidence/phase-3-lifecycle-replay-coverage/final-smoke --assert-canary-rejection
```

Expected smoke artifacts:

- `.omo/evidence/phase-3-lifecycle-replay-coverage/final-smoke/summary.txt`
- `.omo/evidence/phase-3-lifecycle-replay-coverage/final-smoke/lifecycle-report.json`
- `.omo/evidence/phase-3-lifecycle-replay-coverage/final-smoke/trace.jsonl`
- `.omo/evidence/phase-3-lifecycle-replay-coverage/final-smoke/storage-readback.json`
- `.omo/evidence/phase-3-lifecycle-replay-coverage/final-smoke/storage-readback-source.json`
- `.omo/evidence/phase-3-lifecycle-replay-coverage/final-smoke/storage-readback.sqlite`
- `.omo/evidence/phase-3-lifecycle-replay-coverage/final-smoke/privacy-inspect.txt`
- `.omo/evidence/phase-3-lifecycle-replay-coverage/final-smoke/canary-rejection.txt`
- `.omo/evidence/phase-3-lifecycle-replay-coverage/final-smoke/cleanup-receipt.txt`
- `.omo/evidence/phase-3-lifecycle-replay-coverage/final-smoke/command-log-pass-counts.txt`
- `.omo/evidence/phase-3-lifecycle-replay-coverage/final-smoke/command-logs/`

The recorded smoke receipt reports `result: PASS`, local fixture/fake execution only, no live backend, no live vendor, and no live network. The trace artifact contains the six Phase 3 operation names: `candidate_superseded`, `candidate_rescheduled`, `candidate_cancelled`, `calendar_dry_run`, `calendar_commit_idempotency`, and `replay_run`.

Run the bounded real-surface QA wrapper with:

```bash
scripts/run-lifecycle-real-surface-qa.sh --out-dir .omo/evidence/phase-3-lifecycle-replay-coverage/real-surface
```

Expected real-surface artifacts:

- `.omo/evidence/phase-3-lifecycle-replay-coverage/real-surface/summary.txt`
- `.omo/evidence/phase-3-lifecycle-replay-coverage/real-surface/calendar-status.txt`
- `.omo/evidence/phase-3-lifecycle-replay-coverage/real-surface/reminders-status.txt`
- `.omo/evidence/phase-3-lifecycle-replay-coverage/real-surface/cleanup-receipt.txt`

Real-surface status semantics:

- `PASS`: the underlying Calendar or Reminders runner proved write/read/cleanup behavior for that surface.
- `BLOCKED`: OS permission or tooling stopped the run before mutation, or cleanup is complete; the receipt names the required user action and rerun command.
- `FAIL`: the surface was runnable and behavior or cleanup proof failed. Any `FAIL` exits nonzero.

The current receipts record `PASS or BLOCKED per surface exits 0; any FAIL exits nonzero`. Calendar and Reminders are both `BLOCKED` by host permissions, with blocked-before-mutation proof in their receipts. No full real-surface pass is claimed because at least one surface is BLOCKED. Reminders real-surface acceptance is create/read/delete QA only; reschedule/cancel lifecycle coverage is not claimed by that wrapper.

Privacy rules for Phase 3:

- No raw prompt text, raw message bodies, provider JSON, native identifiers, app-data paths, or unredacted candidate titles are exposed in traces, storage readbacks, lifecycle reports, logs, docs, screenshots, or evidence.
- The smoke uses local fixtures/fakes and synthetic privacy surfaces; it does not call live Codex, OpenAI, EventKit, network, or vendor backends.
- Privacy inspection passes only when forbidden content is absent, and canary rejection passes only when the injected raw canary is rejected before sanitization.

Remaining gaps:

- Phase 4 human approval/correction loop remains future work, including risk gates, correction capture, and auditable approval outcomes.
- Phase 5 trajectory eval hardening remains future work for multi-step Messages-to-Calendar approval paths, collateral-damage checks, and replay scoring.
- The full Messages -> Calendar -> approval trajectory eval remains future work.
- Cloud telemetry remains future work; Phase 3 does not upload diagnostics or require Phoenix, Langfuse, LangSmith, Braintrust, LiteLLM, Helicone, or any vendor backend.

## Provider-Backed Messages-to-Calendar Flow

Production Messages-to-Calendar scanning uses the native Messages SQLite source, the local Codex CLI ChatGPT login through `codex exec`, and EventKit proposal replay. Codex provider readiness is a Sync Now prerequisite in the app shell for the production Messages-to-Calendar path.

The trace/eval smoke below remains an offline diagnostics and fixture-eval proof. It does not call Codex, OpenAI, Phoenix, Langfuse, any LLM backend, read the real Messages database, or create EventKit items. Runtime production scan maps to the diagnostics seam through `scan_selected_chats_with_dependencies`: production passes the release provider and EventKit adapter, and Phase 1 adds the local production trace sink behind `localDiagnosticsEnabled`. The Phase 1 smoke uses the native fake Codex production fixture to prove the provider path writes sanitized local JSONL without live backends.

Use the real QA runner only when the host has deliberate Messages, Codex CLI login, and Calendar test setup:

```bash
env -u MORROW_REAL_QA_OPENAI_API_KEY scripts/messages-calendar-real-qa.sh
```

The runner must not require `MORROW_REAL_QA_OPENAI_API_KEY`. Without Codex CLI readiness, the required QA chat variables, or macOS permissions, it records a sanitized `BLOCKED` receipt instead of requiring live secrets for completion.

## Phase 1 Production Provider Trace Sink Smoke

Run the production trace sink smoke with the local fake Codex fixture:

```bash
scripts/run-production-trace-sink-smoke.sh --out-dir .omo/evidence/phase-1-production-provider-trace-sink/final-smoke
```

Expected artifacts:

- `.omo/evidence/phase-1-production-provider-trace-sink/final-smoke/trace.jsonl`
- `.omo/evidence/phase-1-production-provider-trace-sink/final-smoke/privacy-inspect.txt`
- `.omo/evidence/phase-1-production-provider-trace-sink/final-smoke/summary.txt`
- `.omo/evidence/phase-1-production-provider-trace-sink/final-smoke/cleanup-receipt.txt`
- `.omo/evidence/phase-1-production-provider-trace-sink/final-smoke/cargo-test.txt`

Run the production trace sink canary rejection proof:

```bash
scripts/run-production-trace-sink-smoke.sh --out-dir .omo/evidence/phase-1-production-provider-trace-sink/final-smoke --assert-canary-rejection
```

That mode injects `[privacy canary literal]` into a synthetic diagnostics surface before sanitization and passes only when `scripts/privacy-inspect.sh` rejects it. The smoke does not call live Codex, OpenAI, network, calendar, Messages, Phoenix, Langfuse, or vendor backends.

## Feedback Eval Baseline

The committed feedback/eval DB layer is local-only evidence for scan outcomes. It records a privacy-bounded Feedback Eval Baseline for local scan feedback events, labels, feature snapshots, diagnostics trace linkage, and eval reports. It is not hosted telemetry and does not run live provider benchmarks.

The baseline tables are:

- `feedback_events`: one local feedback event per candidate-visible or quiet-stop outcome. Candidate events use provider scan metadata; quiet-log events map deterministic or provider stop reasons into local event types.
- `labels`: one or more local label records for the observed route or system outcome. Candidate labels record the detection route; quiet-log labels record mapped quiet/provider failure outcomes.
- `feature_snapshots`: one local feature snapshot per recorded candidate or quiet log. Snapshots keep route, reason code, confidence where available, participant/tapback metadata, and a bounded excerpt only.
- `eval_runs` and `eval_results`: local eval report state and per-case comparisons. Reports are generated from the local DB smoke flow and are not a hosted eval requirement.

Feedback rows can carry diagnostics trace linkage copied from a privacy-safe trace record: opaque trace/span ids, optional parent span id, and hashed chat/message identifiers. The feedback/eval validators reject malformed trace ids, malformed hashes, overlong fields, and hidden source excerpts that are not the redacted placeholder.

Run the local feedback/eval DB smoke with:

```bash
scripts/run-feedback-eval-smoke.sh --out-dir .omo/evidence/phase-0-feedback-eval-baseline/feedback-eval-smoke
```

That script runs the local reconcile E2E example, verifies `privacy_dataset_ready=true` and `feedback_eval_ready=true`, checks that labels and snapshots were recorded, requires a non-empty metrics report and feedback eval report, and confirms the invalid Delete All path preserves feedback/eval tables.

## Diagnostics Trace/Eval Smoke

Run the complete local flow from existing Morrow fixtures:

```bash
scripts/run-diagnostics-trace-eval-smoke.sh --out-dir .omo/evidence/task-12-final-flow
```

Expected artifacts:

- `.omo/evidence/task-12-final-flow/trace.jsonl`
- `.omo/evidence/task-12-final-flow/eval-summary.json`
- `.omo/evidence/task-12-final-flow/phoenix-payload.json`
- `.omo/evidence/task-12-final-flow/langfuse-payload.json`
- `.omo/evidence/task-12-final-flow/privacy-inspect.txt`
- `.omo/evidence/task-12-final-flow/delete-all.txt`

Run the privacy canary rejection proof:

```bash
scripts/run-diagnostics-trace-eval-smoke.sh --out-dir .omo/evidence/task-12-final-flow --assert-canary-rejection
```

That mode injects `[privacy canary literal]` into synthetic diagnostics before sanitization and passes only when `scripts/privacy-inspect.sh` rejects it.

## Eval Command

The smoke script uses the local eval runner:

```bash
cargo run --manifest-path crates/morrow-detection/Cargo.toml --example detection_eval -- \
  --fixtures crates/morrow-detection/fixtures/golden_conversations.json \
  --out .omo/evidence/task-12-final-flow/eval-summary.json \
  --trace-out .omo/evidence/task-12-final-flow/trace.jsonl
```

The summary must include dataset family, dataset version, dataset provenance, trace schema version, provider id, model id, prompt identity, scenario pass/fail totals, false-positive/false-negative counts, route/reason confusion counts, unsafe-action count, privacy leakage count, prompt-injection resilience count, and canary scan result.

## Privacy Inspection

Diagnostics are scanned with DB and log artifacts:

```bash
scripts/privacy-inspect.sh <morrow-db> <log-dir> <forbidden-tokens-file> <app-data>/diagnostics
```

The diagnostics directory argument is optional for legacy callers, but required for trace/eval/export privacy proof.

Forbidden trace/eval/export content:

- Raw prompt text, raw message text, full provider JSON, raw model responses, embeddings, unredacted title text, reversible native identifiers, and calendar details that can identify user content.
- Forbidden field names include `raw_text`, `prompt`, `response`, `raw_json`, `embedding`, `provider_json`, `full_message`, `raw_title`, `title_text`, `full_title`, and `unredacted_title`.

Allowed content is metadata only: opaque trace/span ids, sanitized component/operation/decision/outcome values, reason codes, privacy tier, title hash/status, provider/model/prompt version identity, and aggregate eval metrics.

## Delete All

Delete All treats diagnostics artifacts and Morrow-owned legacy provider credentials as private Morrow data. It removes only Morrow-owned diagnostics subdirectories under the resolved app data root:

- `diagnostics/traces`
- `diagnostics/evals`
- `diagnostics/exports`

It must not delete unrelated sibling directories such as manual exports outside those subdirectories. Receipts include `diagnosticsArtifactsDeleted` and provider credential cleanup details in `providerCredentialDeletes`. Delete All leaves the user's global Codex CLI login unchanged.

The feedback/eval baseline also has Delete All preservation coverage: the smoke command runs the storage test that verifies an invalid Delete All transaction does not drop or corrupt feedback/eval tables.

## Viewer/Importer Commands

Phoenix is optional, downstream, CLI-only, and dev-only:

```bash
cargo run --manifest-path crates/morrow-diagnostics/Cargo.toml --example export_phoenix -- \
  --input .omo/evidence/task-12-final-flow/trace.jsonl \
  --out .omo/evidence/task-12-final-flow/phoenix-payload.json
```

Langfuse is optional, downstream, CLI-only, and dev-only:

```bash
cargo run --manifest-path crates/morrow-diagnostics/Cargo.toml --example export_langfuse -- \
  --input .omo/evidence/task-12-final-flow/trace.jsonl \
  --out .omo/evidence/task-12-final-flow/langfuse-payload.json
```

Both commands write local sanitized payload captures only. Missing credentials, missing backends, malformed downstream payloads, or vendor outages must not change detection, candidate creation, quiet logs, sync, Delete All, or calendar behavior.

## Dataset Provenance

Use Morrow-owned fixtures first:

- Primary: `crates/morrow-detection/fixtures/golden_conversations.json`
- First adversarial family: `crates/morrow-detection/fixtures/adversarial/`

Benchmark inspiration is used for fixture design, not as a required corpus:

- ToolSandbox: stateful trajectory and milestone scoring.
- AppWorld: executable state and collateral-damage checks.
- ClawsBench/OpenClaw: productivity workflow safety and unsafe-action metrics.
- AgentDojo/AgentDyn: prompt-injection and untrusted-context robustness.
- CalBench: future scheduling-agent coordination, fairness, and privacy inspiration.
- BFCL and temporal datasets such as TempEval/TimeBank/MATRES/TB-Dense: deferred until tool/API-call or temporal-reasoning evaluation becomes a product need.

## Deferred Choices

- LiteLLM and Helicone are deferred until Morrow has live model routing or gateway traffic.
- LangSmith hosted/enterprise workflow is deferred.
- Braintrust is an eval workflow reference for datasets, scorers, and experiments, not a dependency.

## Lifecycle And Reserved Trace Operations

The trace schema now exercises the Phase 3 lifecycle/replay operation names locally while reserving correction vocabulary for future approval/correction work:

- `candidate_superseded`
- `candidate_rescheduled`
- `candidate_cancelled`
- `calendar_dry_run`
- `calendar_commit_idempotency`
- `replay_run`

Additional reserved names remain schema vocabulary only in this layer:

- `poll_empty`
- `cursor_advanced`
- `user_correction`

The full Messages -> Calendar -> approval trajectory eval remains future work. Current local diagnostics, Phase 1 production trace sink, Phase 2 product correlation, Phase 3 lifecycle replay smoke, and feedback/eval receipts prove the local trace substrate plus local candidate/quiet/lifecycle correlation, not an end-to-end approval lifecycle benchmark.

## Non-Goals

- No live SetFit, MiniLM, CatBoost, embedding, OOD, or LLM cascade.
- No vendor backend required for local smoke, CI, runtime scans, evals, Delete All, or privacy inspection.
- No raw prompt, raw message, provider JSON, embeddings, model responses, or unredacted candidate titles in traces, evals, exports, logs, or screenshots.
- No agent-native control-plane CLI.
- No human correction UI or correction-training loop.
- The full Calendar write workflow is not claimed; Phase 3 dry-run and commit-idempotency evidence is local/fake or bounded by explicit real-surface PASS/BLOCKED receipts.
- No approval-bypassing mutation path.
- No Phase 4 human approval/correction loop.
- No completed trajectory-level Messages -> Calendar -> approval eval in this layer.
- No cloud telemetry rollout.
