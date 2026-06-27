# Morrow

Morrow is a Mac-local scheduling assistant prototype. It is intended to scan explicitly selected Messages threads, turn scheduling-shaped messages into typed candidates, and create proposed items in native Calendar/Reminders surfaces for user approval.

## Current QA Status

This repository is prepared as a v0.1 source snapshot. Local OMO planning and evidence artifacts are intentionally ignored, so keep three different QA layers separate:

1. Synthetic pipeline QA passes: fake Messages fixtures can become typed candidates and fake Calendar/Reminders proposals.
2. Real Calendar surface QA passes: this machine can create, read back, and clean up a synthetic EventKit event in `Morrow Proposed`.
3. Real Messages discovery and selected-chat scanning are wired through the production Tauri path, but real Messages to real Calendar event creation is not wired end to end yet: `Sync Now` still needs the production provider/LLM path and Calendar/EventKit proposal creation before it can create a real event from a real message.

Because of that, do not treat synthetic e2e success as proof that a real message has created a real Calendar event.

## What Is Needed For Message To Calendar QA

To run a true manual QA pass from Messages to Calendar, the app needs all of these pieces working in the production Tauri path:

- Messages discovery and selected-thread scanning after Full Disk Access is granted.
- A configured LLM/provider path that can return strict scheduling candidates. Codex OAuth, if used, belongs here as the personal-prototype LLM auth path. It is not needed for Calendar/EventKit write QA by itself.
- A native Calendar proposal adapter in `scan_selected_chats` that creates EventKit events, not only local external-object mappings.
- macOS Calendar permission for the app or terminal process running real EventKit QA.
- At least one explicitly selected chat, a reference timezone, and setup marked complete.

## Local Setup

Install dependencies:

```bash
npm install
```

For Tauri/Rust commands on this machine, use the same SDK/linker environment as the package scripts:

```bash
SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk
RUSTFLAGS="-C linker=/Library/Developer/CommandLineTools/usr/bin/cc"
```

macOS permissions needed for real-surface QA:

- Full Disk Access for the app when reading Messages.
- Full Disk Access for Terminal/Codex when running Messages QA from terminal.
- Calendar access for the app or Terminal/Codex when creating real EventKit test events.
- Reminders access only for Reminders QA.

To recover from a Messages permission denial, open System Settings, go to Privacy & Security, then Full Disk Access, and enable the terminal app or the signed Morrow app that will run the QA. Restart that app after changing the permission. The Messages smoke below intentionally prints only metadata and aggregate counts.

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

The real Messages discovery smoke does not prove true message-to-calendar QA. That still requires the production LLM/provider path and Calendar/EventKit event creation wiring in addition to Messages discovery.

## Manual QA Script Once Production Wiring Exists

1. Start the app with `npm run tauri:dev`.
2. Grant Full Disk Access and Calendar access to the app.
3. Complete onboarding: choose Calendar source, set reference timezone, configure LLM auth, and select exactly one test chat.
4. Send a scheduling-shaped message in that selected chat, for example a future meeting with a concrete time.
5. Run `Sync Now`.
6. Open Calendar and confirm a new `Morrow Proposed` event appears.
7. Inspect the event: it should be free/transparent, have no alerts, no guests, no invite emails, and include Morrow metadata in notes.
8. Move the event to a real calendar to approve, or delete it from `Morrow Proposed` to reject.
9. Run `Sync Now` again and confirm candidate state and pending count reconcile correctly.
