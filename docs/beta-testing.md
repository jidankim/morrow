# Beta Testing

Packaged beta testers should use the signed, notarized DMG from the release handoff. You do not need to clone the repository or run source-build commands to test the packaged app.

Trusted diagnostic testing uses a separate ad-hoc signed, not notarized, host-architecture-only zip artifact documented in `docs/diagnostic-testing.md`. Diagnostic artifacts are not signed/notarized beta artifacts and must not be redistributed.

## Install The Packaged App

1. Download the beta DMG.
2. Open the DMG.
3. Drag `Morrow.app` into Applications or another folder you use for test apps.
4. Open Morrow from the installed `Morrow.app`.

## Grant macOS Permissions

Morrow needs two separate macOS permissions for real Messages-to-Calendar QA:

- Full Disk Access for `Morrow.app` so it can read local Messages metadata after you explicitly select chats.
- Calendar access for Morrow so it can create proposed EventKit items during real QA.

If Morrow opens Full Disk Access settings, macOS still requires you to manually enable or add `Morrow.app`. Restart Morrow after changing the permission, then retry chat discovery.

## Codex CLI For Provider-Backed Sync Now

Install Codex CLI and run `codex login` only when you are testing provider-backed `Sync Now`. Codex CLI is a user-owned runtime prerequisite; Morrow detects whether it is ready and does not manage the CLI login.

## Manual QA flow

1. Open Morrow.
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

`npm install`, Rust, Xcode/Command Line Tools, `SDKROOT`, and Tauri dev commands such as `npm run tauri:dev` are for maintainers building or debugging Morrow from source. They are not prerequisites for packaged beta testers.
