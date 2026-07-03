# Morrow

Morrow is a Mac-local scheduling assistant prototype. It is intended to scan explicitly selected Messages threads, turn scheduling-shaped messages into typed candidates, and create proposed items in native Calendar/Reminders surfaces for user approval.

## Current QA Status

This repository is prepared as a v0.1 source snapshot. Local OMO planning and evidence artifacts are intentionally ignored, so keep three different QA layers separate:

1. Synthetic pipeline QA passes: fake Messages fixtures can become typed candidates and fake Calendar/Reminders proposals.
2. Real Calendar surface QA passes: this machine can create, read back, and clean up a synthetic EventKit event in `Morrow Proposed`.
3. Real Messages discovery, selected-chat scanning, the Codex provider path, and EventKit proposal creation are wired through the production Tauri path. "Production-wired" means the app can use the native Tauri path after local permissions and Codex CLI login are complete. It does not mean real Messages-to-Calendar event creation is verified end to end yet: live QA still needs a real environment that allows EventKit readback and cleanup after `Sync Now` creates a proposed event from a real message.

Because of that, do not treat synthetic e2e success as proof that a real message has created a real Calendar event.

## Packaged Beta Testing

Packaged beta testers should start with `docs/beta-testing.md`: download and open the DMG, drag/install `Morrow.app`, open Morrow, grant Full Disk Access and Calendar access, install Codex CLI and run `codex login` only when validating provider-backed `Sync Now`, then run the manual QA flow.

## What Is Needed For Message To Calendar QA

To run a true manual QA pass from Messages to Calendar, the app needs all of these pieces working in the production Tauri path:

- Messages discovery and selected-thread scanning after Full Disk Access is granted.
- A configured LLM/provider path that can return strict scheduling candidates. For the no-API-key prototype path, Morrow uses the user's existing Codex CLI ChatGPT login through `codex exec`; run `codex login` first and keep the Codex CLI installed. Morrow does not own, store, print, or delete global Codex tokens or sessions.
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

For real Messages-to-Calendar QA, grant Full Disk Access and Calendar access to the built `Morrow.app`. If you are validating provider-backed `Sync Now`, install Codex CLI and run `codex login` before opening Morrow.

macOS permissions needed for real-surface QA:

- Full Disk Access for the app when reading Messages.
- Full Disk Access for Terminal/Codex when running Messages QA from terminal.
- Calendar access for the app or Terminal/Codex when creating real EventKit test events.
- Reminders access only for Reminders QA.

Provider auth boundary:

- `codex login` is required for the Codex CLI session-backed provider path.
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

This path must use the local Codex CLI session and must not require an OpenAI API key. It can still block on external setup: missing Codex CLI login, missing Full Disk Access for Messages, missing Calendar access, or missing real QA chat environment variables.

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
