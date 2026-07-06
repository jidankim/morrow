# Diagnostic Testing

This guide is for a trusted technical tester using a short-term diagnostic Morrow artifact. A trusted technical tester is someone comfortable using Finder right-click Open or System Settings Privacy & Security approval when macOS blocks an app from an unidentified developer, but not expected to run Terminal commands.

The diagnostic artifact is host-architecture only. Use it only on the same Mac architecture recorded in the handoff manifest; it is not a universal beta build and does not claim Intel or Apple Silicon compatibility unless that exact host architecture is listed.

The primary artifact is the diagnostic zip containing `Morrow.app` and `Morrow-diagnostic-tester-note.md`. The app is ad-hoc signed and not notarized, so macOS may show security warnings or block the first launch. If blocked, open it with Finder right-click Open, or approve Morrow in System Settings Privacy & Security and then open it again.

The diagnostic artifact must not be redistributed. Do not share the diagnostic zip, app, tester note, manifest, checksums, or optional sidecar artifacts. This artifact is only for the named trusted diagnostic tester and does not replace the signed, notarized beta DMG release path.

Physical iPhone testing without a paid Apple Developer Program is a separate source-build flow. It is documented in `docs/iphone-physical-device-testing.md` and does not use this macOS diagnostic zip.

## Grant macOS Permissions

Morrow needs two separate macOS permissions for real Messages-to-Calendar QA:

- Full Disk Access for `Morrow.app` so it can read local Messages metadata after you explicitly select chats.
- Calendar access for Morrow so it can create proposed EventKit items during real QA.

If Morrow opens Full Disk Access settings, macOS still requires you to manually enable or add `Morrow.app`. Restart Morrow after changing the permission, then retry chat discovery.

## Codex CLI For Provider-Backed Sync Now

Install Codex CLI and run `codex login` only when you are testing provider-backed `Sync Now`. Codex CLI is a user-owned runtime prerequisite; Morrow detects whether it is ready and does not manage the CLI login.

## Manual QA Flow

1. Open Morrow from the diagnostic `Morrow.app`.
2. Grant Full Disk Access and Calendar access to `Morrow.app`.
3. Complete onboarding: choose Calendar source, set reference timezone, confirm Codex provider readiness if testing provider-backed `Sync Now`, and select exactly one test chat.
4. Send a scheduling-shaped message in that selected chat, using a future meeting with a concrete time.
5. Run `Sync Now`.
6. Open Calendar and confirm a new `Morrow Proposed` event appears.
7. Inspect the event: it should be free/transparent, have no alerts, no guests, no invite emails, and include Morrow metadata in notes.
8. Move the event to a real calendar to approve it, or delete it from `Morrow Proposed` to reject it.
9. Run `Sync Now` again and confirm candidate state and pending count reconcile correctly.

Package launch by itself only proves that the app opens. Real Messages-to-Calendar QA still requires Messages access, provider readiness for candidate extraction, Calendar access, and EventKit readback and cleanup.

## Maintainer/source-build only

`npm install`, Rust, Xcode/Command Line Tools, `SDKROOT`, and Tauri dev commands such as `npm run tauri:dev` are for maintainers building or debugging Morrow from source. They are not prerequisites for diagnostic testers.
