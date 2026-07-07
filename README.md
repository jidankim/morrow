# Morrow

Morrow is a Mac-local scheduling assistant prototype. It is intended to scan explicitly selected Messages threads, turn scheduling-shaped messages into typed candidates, and create proposed items in native Calendar/Reminders surfaces for user approval.

## Current QA Status

This repository is prepared as a v0.1 source snapshot. Local OMO planning and evidence artifacts are intentionally ignored, so keep three different QA layers separate:

1. Synthetic pipeline QA passes: fake Messages fixtures can become typed candidates and fake Calendar/Reminders proposals.
2. Real Calendar surface QA passes: this machine can create, read back, and clean up a synthetic EventKit event in `Morrow Proposed`.
3. Real Messages discovery, selected-chat scanning, the Codex provider path, and EventKit proposal creation are wired through the production Tauri path. "Production-wired" means the app can use the native Tauri path after local permissions and Codex CLI login are complete. It does not mean real Messages-to-Calendar event creation is verified end to end yet: live QA still needs a real environment that allows EventKit readback and cleanup after `Sync Now` creates a proposed event from a real message.
4. The local Phase 4 approval/correction evidence loop is covered by local smoke artifacts only. It proves sanitized local evidence for accepted, rejected, pending-edited, unknown, and `user_correction` outcomes with no live network, vendor, Messages, Calendar, or Reminders access.
5. The local Phase 5 trajectory eval hardening is covered by final-smoke artifacts only. It proves fixture-backed multi-step Messages-to-Calendar approval trajectory scoring, collateral-damage checks, replay scoring, privacy inspection, canary rejection, and cleanup under `.omo/evidence/phase-5-messages-calendar-approval-trajectory-eval/final-smoke`. It does not prove a full live Messages-to-Calendar approval trajectory. The correction UI remains future work, cloud telemetry remains future work, deployed rollout remains future work, the full live Messages-to-Calendar approval trajectory eval remains future work, and live receipt status remains PASS or sanitized BLOCKED.
6. Phase 6 cloud eval and monitoring is covered by smoke artifacts under `.omo/evidence/phase-6-cloud-eval-monitoring/final-smoke`. It proves deterministic fixture-backed cloud eval monitoring inputs, version-sliced metrics, dashboard artifacts, release gates, alert receipts, privacy/canary rejection, negative matrix coverage, and cleanup. The smoke command is `scripts/run-cloud-eval-monitoring-smoke.sh --out-dir .omo/evidence/phase-6-cloud-eval-monitoring/final-smoke --assert-canary-rejection`. Required artifacts include `.omo/evidence/phase-6-cloud-eval-monitoring/final-smoke/summary.txt`, `.omo/evidence/phase-6-cloud-eval-monitoring/final-smoke/preflight-report.json`, `.omo/evidence/phase-6-cloud-eval-monitoring/final-smoke/eval-job-input.json`, `.omo/evidence/phase-6-cloud-eval-monitoring/final-smoke/eval-slices.json`, `.omo/evidence/phase-6-cloud-eval-monitoring/final-smoke/metrics.json`, `.omo/evidence/phase-6-cloud-eval-monitoring/final-smoke/cloud-eval-monitoring-report.json`, `.omo/evidence/phase-6-cloud-eval-monitoring/final-smoke/cloud-eval-monitoring-dashboard.md`, `.omo/evidence/phase-6-cloud-eval-monitoring/final-smoke/dashboard-summary.txt`, `.omo/evidence/phase-6-cloud-eval-monitoring/final-smoke/release-gate.json`, `.omo/evidence/phase-6-cloud-eval-monitoring/final-smoke/release-gate.md`, `.omo/evidence/phase-6-cloud-eval-monitoring/final-smoke/alerts.json`, `.omo/evidence/phase-6-cloud-eval-monitoring/final-smoke/privacy-inspect.txt`, `.omo/evidence/phase-6-cloud-eval-monitoring/final-smoke/canary-rejection.txt`, `.omo/evidence/phase-6-cloud-eval-monitoring/final-smoke/negative-matrix.json`, `.omo/evidence/phase-6-cloud-eval-monitoring/final-smoke/cleanup-receipt.txt`, and `.omo/evidence/phase-6-cloud-eval-monitoring/final-smoke/command-logs/`. This cloud eval and monitoring is separate from deployed rollout, correction UI, full live Messages-to-Calendar approval completion, and automatic release promotion. The correction UI remains future work, deployed rollout remains future work, automatic release promotion remains future work, and the full live Messages-to-Calendar approval trajectory eval remains future work. The backend observable is no live backend, provider network, EventKit, Messages, Calendar, Reminders, Phoenix, Langfuse, vendor backend, deployment, or cloud upload surface.

Because of that, do not treat synthetic e2e success as proof that a real message has created a real Calendar event.

## Packaged Beta Testing

Packaged beta testers should start with `docs/beta-testing.md`: download and open the DMG, drag/install `Morrow.app`, open Morrow, grant Full Disk Access and Calendar access, use Morrow's Install Codex CLI action only after explicit confirmation when validating provider-backed Sync Now, use Start Codex login for browser sign-in, then run the manual QA flow.

## What Is Needed For Message To Calendar QA

To run a true manual QA pass from Messages to Calendar, the app needs all of these pieces working in the production Tauri path:

- Messages discovery and selected-thread scanning after Full Disk Access is granted.
- A configured LLM/provider path that can return strict scheduling candidates. For the no-API-key prototype path, Morrow uses the user's existing Codex CLI ChatGPT login through `codex exec`; the app can help install the CLI after explicit confirmation and can launch Start Codex login for browser sign-in. Codex owns the login, and Morrow does not store provider tokens or read, manage, logout, or delete the user's Codex session.
- A native Calendar proposal adapter in `scan_selected_chats` that creates EventKit events, not only local external-object mappings.
- macOS Calendar permission for the app or terminal process running real EventKit QA.
- At least one explicitly selected chat, a reference timezone, and setup marked complete.

No OAuth is required for native Messages discovery. Messages discovery is a local macOS read of the Messages database, so Full Disk Access is the relevant prerequisite for discovery and selected-chat scanning. Provider auth only belongs to scheduling-candidate extraction, and Calendar access is a separate macOS gate for EventKit proposal creation.

The latest-message body preview is not part of the default MVP. Discovery rows should remain limited to privacy-safe metadata such as a sanitized chat label, participant count, latest activity timestamp, and selected/verified state.

## Maintainer/source-build only Local Setup

The commands in this section are for maintainers building or debugging Morrow from source. Packaged beta testers should use the DMG flow in `docs/beta-testing.md` instead.

Install dependencies:

```bash
npm install
```

Rust and Xcode/Command Line Tools are maintainer prerequisites for source builds and local Tauri debugging.

For Tauri/Rust commands on this machine, use the same SDK/linker environment as the package scripts:

```bash
SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk
RUSTFLAGS="-C linker=/Library/Developer/CommandLineTools/usr/bin/cc"
```

### iPhone/Tauri iOS status

`package.json` includes developer-only Tauri iOS scripts for the feasibility spike. They use `rustup` when it is already installed, or `nix shell nixpkgs#rustup` when this environment only has Nix-provided Rust:

- `npm run tauri:ios:init` runs `tauri ios init`.
- `npm run tauri:ios:dev` runs `tauri ios dev`.
- `npm run tauri:ios:build:simulator` runs `tauri ios build --target aarch64-sim --no-sign`.
- `npm run tauri:ios:build:device` runs `tauri ios build`.
- `npm run tauri:ios:build:app-store` runs `tauri ios build --export-method app-store-connect`.

Current status: G1 iOS init/config passes in this environment after installing the required Rust iOS targets, XcodeGen, libimobiledevice, and CocoaPods. G2 simulator build also passes repeatably with Xcode 16.4 and the iOS 18.6 simulator runtime, producing `src-tauri/gen/apple/build/arm64-sim/Morrow.app`; the rebuilt app installs and launches on the iPhone 16 Pro simulator, where the Morrow UI renders. These scripts do not establish physical-device deploy, signing, App Store/TestFlight, or EventKit runtime readiness, and this repository is not App Store ready. They also do not provide a passive iPhone Messages reader or any private API path.

For no-paid-Developer-Program physical iPhone testing, use the source-build flow in `docs/iphone-physical-device-testing.md`. That path requires the tester to build from source on their own Mac and deploy through Xcode/free provisioning to a connected iPhone; it is not an AirDrop, shared IPA, or Mac-style diagnostic zip flow.

### Build and run the packaged app locally

Use this when a technical tester is building Morrow from source on their own Mac. This is different from sharing a downloaded beta or diagnostic artifact: the tester must have Node, Rust, and Xcode/Command Line Tools installed.

Build the app bundle:

```bash
npm run tauri:build
```

Open the built app bundle:

```bash
open -n -F src-tauri/target/release/bundle/macos/Morrow.app
```

If the bundle path changes, locate it with:

```bash
find src-tauri/target/release -name Morrow.app -print
```

For real Messages-to-Calendar QA, grant Full Disk Access and Calendar access to the built `Morrow.app`. If you are validating provider-backed Sync Now, Morrow can show Install Codex CLI, require explicit confirmation before installing, and then offer Start Codex login to launch the Codex browser sign-in flow.

macOS permissions needed for real-surface QA:

- Full Disk Access for the app when reading Messages.
- Full Disk Access for Terminal/Codex when running Messages QA from terminal.
- Calendar access for the app or Terminal/Codex when creating real EventKit test events.
- Reminders access only for Reminders QA.

Provider auth boundary:

- `codex login` is required for the Codex CLI session-backed provider path; Start Codex login only launches that Codex-owned browser flow.
- Codex owns the login. Morrow does not store provider tokens and does not read, manage, logout, or delete Codex credentials.
- `MORROW_REAL_QA_OPENAI_API_KEY` is not required for the target Messages-to-Calendar real QA path.
- Delete-all and privacy cleanup only remove Morrow-owned legacy provider credentials/markers, such as old Keychain entries created by Morrow. They leave the user's global Codex CLI login unchanged.
- Messages Full Disk Access and Calendar access remain independent macOS permissions; fixing one does not grant the other.

### Guided Full Disk Access Recovery

When native Messages discovery is denied or unavailable, Morrow can open Full Disk Access settings with the `Open Full Disk Access` action. Opening System Settings is only a shortcut: macOS still requires the user to manually enable or add Morrow before the app can read local Messages metadata.

Use the runtime target shown by Morrow when granting access:

- Bundled builds should enable or add `Morrow.app`.
- Unbundled dev or release runs may need the exact executable path shown by the app, such as `target/debug/morrow` or `target/release/morrow`.
- If macOS opens a file picker, press `Cmd+Shift+G`, paste the shown path, then choose the `morrow` executable.

After changing Full Disk Access, restart Morrow, then retry native discovery with `Retry chat discovery`. Terminal/Codex Full Disk Access only proves terminal QA and does not grant app access; grant the app bundle or executable that is actually running Morrow. The Messages smoke below intentionally prints only metadata and aggregate counts.

## Native Messages Discovery Onboarding QA

Use this flow when validating the native Messages setup surface:

1. Grant Full Disk Access to the Morrow app bundle or executable that is running the app. Grant Terminal/Codex only when running terminal QA.
2. Start the app and open the setup surface. Messages discovery should show a distinct state while Morrow checks local Messages access.
3. If discovery is denied or unavailable, use `Open Full Disk Access`, manually enable or add the shown Morrow target, restart Morrow, then use `Retry chat discovery`. The recovery copy should point to Full Disk Access when permission is denied.
4. When eligible chats appear, use the chat checkboxes to `Select at least one chat`. Newly selected chats keep `Ask before backfilling older messages` enabled by default.
5. Confirm the setup checklist shows Messages discovery ready, chat selection complete, and selected-chat verification complete after native chat discovery is ready and at least one selected chat is verified.
6. Confirm `Sync Now` is enabled by checking the `Sync Now` metric for `Enabled` and by checking that the `Sync Now` button is no longer disabled. If `Sync Now` is blocked, confirm the status explains the active discovery, selection, stale discovery, `Sync Now disabled`, or syncing blocker.
7. Treat any real event creation from Messages as out of scope unless the production provider/LLM path and Calendar/EventKit proposal adapter have also been verified in the same QA pass.

## QA Commands

Frontend and app shell:

```bash
npm test
npm run build
```

Calendar adapter invariants:

```bash
cd crates/morrow-calendar
cargo test
```

Native scan tests:

```bash
cd src-tauri
SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk \
RUSTFLAGS="-C linker=/Library/Developer/CommandLineTools/usr/bin/cc" \
cargo test --test native_scan
```

Synthetic MVP e2e:

```bash
cd crates/morrow-reconcile
MORROW_E2E_DB=/tmp/morrow-e2e-workflow.sqlite \
MORROW_E2E_DELETE_DB=/tmp/morrow-e2e-delete.sqlite \
MORROW_E2E_METRICS_REPORT=/tmp/morrow-e2e-metrics.txt \
cargo run --example mvp_e2e
```

Real Calendar EventKit smoke:

```bash
crates/morrow-calendar/scripts/real_calendar_qa.sh
```

This creates a synthetic future event in `Morrow Proposed`, verifies free/transparent availability, no alarms, no guests, no invite semantics, notes metadata, video URL, readback by identifier, and then deletes the synthetic event.

Calendar lifecycle smoke:

```bash
crates/morrow-calendar/scripts/calendar_lifecycle_real_qa.sh
```

Use this when validating proposed-item approval/rejection behavior against the real Calendar surface.

Real Messages discovery smoke:

```bash
scripts/messages-discovery-real-qa.sh
```

This opens `~/Library/Messages/chat.db` through `sqlite3 -readonly` with `PRAGMA query_only = ON`, reports schema compatibility, aggregate row/chat counts, timestamp conversion mode, and whether the metadata-only discovery query can run. It must end with either `PASS messages_discovery_real_qa` or `BLOCKED: Full Disk Access required`. The output must not include message bodies, raw handles, phone numbers, emails, or chat transcripts.

Privacy inspection for the real Messages smoke evidence:

```bash
scripts/privacy-inspect.sh /tmp/morrow-native-messages-discovery.sqlite .omo/evidence/native-messages-discovery .omo/evidence/native-messages-discovery-forbidden-tokens.txt
```

The real Messages discovery smoke does not prove true message-to-calendar QA. That still requires a live Codex provider run plus EventKit readback and cleanup in addition to Messages discovery.

Real Messages-to-Calendar QA through the Codex CLI provider:

```bash
env -u MORROW_REAL_QA_OPENAI_API_KEY scripts/messages-calendar-real-qa.sh
```

Phase 4 local approval/correction evidence smoke:

```bash
scripts/run-human-approval-correction-smoke.sh --out-dir .omo/evidence/phase-4-human-approval-correction/final-smoke --assert-canary-rejection
```

Expected Phase 4 smoke artifacts:

- `.omo/evidence/phase-4-human-approval-correction/final-smoke/summary.txt`
- `.omo/evidence/phase-4-human-approval-correction/final-smoke/human-approval-correction-report.json`
- `.omo/evidence/phase-4-human-approval-correction/final-smoke/trace.jsonl`
- `.omo/evidence/phase-4-human-approval-correction/final-smoke/storage-readback.json`
- `.omo/evidence/phase-4-human-approval-correction/final-smoke/decision-evidence.json`
- `.omo/evidence/phase-4-human-approval-correction/final-smoke/privacy-inspect.txt`
- `.omo/evidence/phase-4-human-approval-correction/final-smoke/canary-rejection.txt`
- `.omo/evidence/phase-4-human-approval-correction/final-smoke/cleanup-receipt.txt`

Phase 4 real-surface receipt command:

```bash
scripts/run-lifecycle-real-surface-qa.sh --out-dir .omo/evidence/phase-4-human-approval-correction/real-surface
```

Expected Phase 4 real-surface receipts include `.omo/evidence/phase-4-human-approval-correction/real-surface/summary.txt` and `.omo/evidence/phase-4-human-approval-correction/real-surface/cleanup-receipt.txt`. A PASS receipt proves that bounded surface; a sanitized BLOCKED receipt means host permission or tooling stopped the run before mutation or after cleanup. A FAIL receipt is not acceptable.

This path must use the local Codex CLI session and must not require an OpenAI API key. It can still block on external setup: missing Codex CLI login, missing Full Disk Access for Messages, missing Calendar access, or missing real QA chat environment variables.

Phase 5 local Messages-to-Calendar approval trajectory eval smoke:

```bash
scripts/run-messages-calendar-approval-trajectory-eval-smoke.sh --out-dir .omo/evidence/phase-5-messages-calendar-approval-trajectory-eval/final-smoke --assert-canary-rejection
```

Expected Phase 5 local smoke artifacts:

- `.omo/evidence/phase-5-messages-calendar-approval-trajectory-eval/final-smoke/summary.txt`
- `.omo/evidence/phase-5-messages-calendar-approval-trajectory-eval/final-smoke/trajectory-report.json`
- `.omo/evidence/phase-5-messages-calendar-approval-trajectory-eval/final-smoke/trace.jsonl`
- `.omo/evidence/phase-5-messages-calendar-approval-trajectory-eval/final-smoke/storage-readback.json`
- `.omo/evidence/phase-5-messages-calendar-approval-trajectory-eval/final-smoke/decision-evidence.json`
- `.omo/evidence/phase-5-messages-calendar-approval-trajectory-eval/final-smoke/privacy-inspect.txt`
- `.omo/evidence/phase-5-messages-calendar-approval-trajectory-eval/final-smoke/canary-rejection.txt`
- `.omo/evidence/phase-5-messages-calendar-approval-trajectory-eval/final-smoke/cleanup-receipt.txt`

The Phase 5 local smoke reports `result: PASS` only for the local evidence boundary: required trajectory case families, command pass counts, collateral-damage checks, replay scoring, privacy inspection, canary rejection, and cleanup. The backend observable is no live backend, provider network, EventKit, Messages, Calendar, Reminders, Phoenix, Langfuse, or vendor backend; the smoke uses local fixtures/fakes and synthetic privacy surfaces.

Phase 5 live receipt command:

```bash
scripts/run-messages-calendar-approval-live-receipt.sh --out-dir .omo/evidence/phase-5-messages-calendar-approval-trajectory-eval/live-receipt
```

Expected Phase 5 live receipt artifacts include `.omo/evidence/phase-5-messages-calendar-approval-trajectory-eval/live-receipt/summary.txt`, `.omo/evidence/phase-5-messages-calendar-approval-trajectory-eval/live-receipt/cleanup-receipt.txt`, and `.omo/evidence/phase-5-messages-calendar-approval-trajectory-eval/live-receipt/privacy-inspect.txt`. A PASS receipt is required before claiming a full live Messages-to-Calendar approval trajectory pass. A sanitized BLOCKED receipt must keep `full_live_claim_allowed=false` and must not be treated as live trajectory completion.

Full live PASS requires explicit mutation opt-in, a controlled real QA chat, a future scheduling message expectation, and a manual proof file for the approval/reconcile/idempotency observations. Create the proof file only after the manual QA flow below has actually been completed against the controlled test chat:

```bash
cat > /tmp/morrow-phase5-live-proof.env <<'EOF'
schema=phase5_messages_calendar_approval_manual_proof_v1
approval_or_rejection_observed=PASS
reconcile_observed=PASS
idempotency_observed=PASS
cleanup_confirmed=PASS
privacy_confirmed=PASS
EOF

MORROW_REAL_QA_CHAT_PUBLIC_ID=<selected-test-chat-id> \
MORROW_REAL_QA_EXPECTED_TITLE_CONTAINS=<expected-title-fragment> \
MORROW_REAL_QA_FUTURE_ISO_LOCAL=<YYYY-MM-DDTHH:MM:SS> \
MORROW_APPROVAL_LIVE_RECEIPT_ALLOW_SURFACE_QA=true \
MORROW_APPROVAL_LIVE_RECEIPT_ALLOW_MUTATION=true \
MORROW_APPROVAL_LIVE_RECEIPT_MANUAL_PROOF_FILE=/tmp/morrow-phase5-live-proof.env \
scripts/run-messages-calendar-approval-live-receipt.sh --out-dir .omo/evidence/phase-5-messages-calendar-approval-trajectory-eval/live-receipt
```

## Manual QA Flow Once Production Wiring Exists

1. For packaged beta QA, open the installed `Morrow.app`. For local source-build QA, run `npm run tauri:build` and open `src-tauri/target/release/bundle/macos/Morrow.app`. For unbundled maintainer debugging, start the app with `npm run tauri:dev`.
2. Grant Full Disk Access and Calendar access to the app.
3. Complete onboarding: choose Calendar source, set reference timezone, confirm Codex provider readiness, and select exactly one test chat.
4. Send a scheduling-shaped message in that selected chat, for example a future meeting with a concrete time.
5. Run `Sync Now`.
6. Open Calendar and confirm a new `Morrow Proposed` event appears.
7. Inspect the event: it should be free/transparent, have no alerts, no guests, no invite emails, and include Morrow metadata in notes.
8. Move the event to a real calendar to approve, or delete it from `Morrow Proposed` to reject.
9. Run `Sync Now` again and confirm candidate state and pending count reconcile correctly.
