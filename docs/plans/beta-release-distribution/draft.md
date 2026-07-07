---
slug: beta-release-distribution
status: high-accuracy-review-in-progress
intent: clear
review_required: true
pending-action: dual high-accuracy review
approach: Enable a tester-ready macOS beta distribution path for Morrow: produce a signed/notarized app/DMG release artifact, document tester setup, and keep Codex CLI as a detected runtime prerequisite while removing Rust/Xcode/Tauri dev tooling from the tester path.
---

# Draft: beta-release-distribution

## Components (topology ledger)
<!-- Lock the SHAPE before depth. One row per top-level component that can succeed or fail independently. -->
<!-- id | outcome (one line) | status: active|deferred | evidence path -->
release-artifact | A beta tester can download/install both Morrow.app and a DMG instead of cloning/building the repo. | active | README.md:64, src-tauri/tauri.conf.json:28, package.json:12, Context7 Tauri docs: v2 distribute/build/bundle
signing-distribution | macOS Gatekeeper path is addressed with signing/notarization or an explicit unsigned-beta fallback decision. | active | Context7 Tauri docs: v2 distribute/sign/macos and distribute/dmg
runtime-prereqs | Codex CLI, Full Disk Access, and Calendar access are treated as tester/runtime gates with in-app/docs guidance. | active | README.md:20, README.md:46, README.md:53, src/SettingsProviderCredentialSection.tsx:46, src/SettingsProviderCredentialSection.tsx:127, src-tauri/src/native_bridge/codex_auth/readiness.rs:3
build-prereqs | Rust, Xcode/Command Line Tools, and Tauri build dependencies are retained only for maintainers/CI/builders, not beta testers. | active | README.md:34, README.md:37, package.json:11, package.json:12, Tauri docs: tauri build generates bundles/installers
release-qa | The plan verifies installer launch, app identity/permissions guidance, Codex readiness states, and a source-build fallback. | active | README.md:58, README.md:159, scripts/readme-full-disk-access-recovery-qa.mjs:20

## Open assumptions (announced defaults)
<!-- Record any default you adopt instead of asking, so the user can veto it at the gate. -->
<!-- assumption | adopted default | rationale | reversible? -->
tester-install-model | Ship a macOS app bundle/DMG as the primary beta path; keep source clone + `npm run tauri:dev` only as a developer fallback. | This directly removes the Rust/Xcode burden from testers and matches Tauri's macOS distribution model. | reversible
codex-cli-policy | Do not bundle or auto-install Codex CLI in Morrow v0.1 beta; detect it, provide clear install/login guidance, and require the user-owned `codex login` session for provider-backed Sync Now. | The app already promises not to own/store/delete global Codex credentials; Codex login is a user account/security action. | reversible
build-tool-policy | Do not require testers to install Rust, Xcode, or Tauri CLI when using the packaged beta. Require these only for maintainers/CI and source-build testers. | Rust/Xcode are build-time dependencies; a shipped `.app`/`.dmg` should contain the compiled app. | reversible
updater-policy | Defer automatic in-app updates entirely for v0.1 beta. | Tauri updater artifacts require extra signing/update-channel decisions; first beta uses manual downloads. | reversible
test-strategy | Use tests-after plus agent-executed QA for packaging/docs/readiness changes. | Existing app behavior exists; the release task mostly wires distribution and documentation rather than introducing a new domain model. | reversible

## Findings (cited - path:lines)
- `src-tauri/tauri.conf.json:28` currently has `"bundle": { "active": false }`, so release bundling is not enabled yet.
- `package.json:12` exposes `npm run tauri:build`, but the current config prevents producing normal bundle artifacts.
- Tauri v2 docs say `tauri build` builds release mode and generates bundles/installers when bundling is active; docs also show direct macOS distribution with `tauri bundle --bundles app,dmg`.
- Tauri v2 docs describe DMG as the common outside-App-Store macOS installer, wrapping the app bundle in a drag-to-Applications window.
- Tauri v2 signing docs route macOS direct distribution through code signing identity and CI/local signing configuration; notarization/stapling are part of the build/signing surface.
- `README.md:34` and `README.md:37` list `npm install`, `SDKROOT`, and `RUSTFLAGS` for local source/Tauri commands, which should not be tester requirements for a packaged beta.
- `README.md:20` and `README.md:53` state the production provider path uses the user's existing Codex CLI ChatGPT login and requires `codex login`.
- `src/SettingsProviderCredentialSection.tsx:46` tells users Morrow uses the local Codex CLI ChatGPT login and stores no provider tokens.
- `src/SettingsProviderCredentialSection.tsx:127` already branches missing CLI to "Install Codex CLI, then run codex login."
- `src-tauri/src/native_bridge/codex_auth/readiness.rs:3` checks `codex login status` and reports missing CLI, not logged in, timeout, unknown failure, or ready.
- OpenAI Codex manual lines 2490-2709 confirm Codex CLI supports ChatGPT/API-key sign-in, caches login details locally, and treats cached auth as sensitive user credentials.
- OpenAI Codex manual lines 7248-7271 identify Codex CLI as a local terminal app available on macOS/Windows/Linux.
- Earlier planning saw unrelated dirty worktree changes, but `git status --short --branch` was clean when this approved plan was written. The execution plan still requires workers to re-check status and avoid overwriting future unrelated changes.

## Decisions (with rationale)
- The next release task should handle Codex CLI as a runtime prerequisite, not as a bundled dependency: in-app readiness detection, docs, and QA must prove the missing/not-logged-in/ready states.
- The next release task should handle Rust/Xcode by making them unnecessary for testers: produce/install a packaged app. It should still document Rust/Xcode/Command Line Tools as maintainer/source-build prerequisites.
- The next release task must require signed/notarized app and DMG artifacts for tester distribution. Unsigned artifacts may be local diagnostics only and must not satisfy beta-release success.
- Automatic updates are out of scope for v0.1 beta. Tauri updater artifacts are available, but they add channel/signature/hosting decisions that are not required to stop testers from cloning the repo.

## Scope IN
- Enable/build a macOS beta artifact path that produces both `.app` and `.dmg` artifacts from the current Tauri app.
- Add or update scripts/docs so maintainers can build the artifact repeatably.
- Add release QA that verifies a packaged app launches and reports the correct runtime identity/permissions guidance.
- Keep Codex CLI readiness as an app-tested/runtime-tested prerequisite, including missing CLI and not logged-in states.
- Update tester-facing instructions so packaged-beta testers do not install Rust, Xcode, Tauri CLI, or clone the repo.
- Preserve source-build instructions for developer fallback.

## Scope OUT (Must NOT have)
- Must not auto-install Codex CLI without explicit user approval in app UX.
- Must not bundle or copy the user's `~/.codex` credentials, token files, or keychain entries.
- Must not make Morrow own, delete, print, or migrate global Codex credentials.
- Must not require beta testers using the packaged app to install Rust, Xcode, or Tauri CLI.
- Must not overwrite the current unrelated dirty worktree changes.
- Must not claim real Messages-to-Calendar production QA solely from packaging/build success.
- Must not add automatic updater/channel infrastructure in this plan.

## Open questions
- Resolved by approved defaults: signed/notarized is the release gate; unsigned artifacts are diagnostics only.
- Resolved by approved defaults: auto-updates are out of scope for v0.1 beta; use manual downloads.

## Approval gate
status: approved-and-written
pending action: dual high-accuracy review of `.omo/plans/beta-release-distribution.md`.
Metis receipt: `019f1908-ba4f-7572-9b4c-86ec7b35d5ac` returned ITERATE; issues folded into the written plan by requiring both `.app` and `.dmg`, making signing/notarization the release gate, deferring updater, adding exact artifact/signing/Codex/docs/cleanup QA, and updating stale dirty-worktree context.

## High-accuracy review receipts
- Requested: 2026-07-01 Asia/Seoul by user message "high accuracy plan review".
- Native Momus: first pass ITERATE, session `019f1925-a1ca-7290-8c50-3003290a5301`. Findings: Todo 5 missing exact failure invocation/evidence consolidation; Todo 9 missing exact manifest failure invocation; F4 wrote `rg` input instead of captured evidence.
- Independent Codex CLI: first blocked by escalation reviewer because sending local plan/draft contents to external Codex/OpenAI service needed explicit user approval. User explicitly approved. Retry in disposable workspace `/private/tmp/morrow-plan-review-workspace` with isolated `CODEX_HOME=/private/tmp/morrow-plan-review-codex-home` reached Codex CLI but failed with 401 Unauthorized because isolated `CODEX_HOME` has no auth. Default Codex login is available in the user's normal Codex home, but it was not used because the high-accuracy workflow requires isolated CODEX_HOME.
- Native Momus re-review: OKAY, session `019f1928-b937-7d22-ba6a-b82644e26f42`. Summary: prior findings fixed; todos 1-9 have references, acceptance criteria, happy/failure QA, evidence paths, and commit guidance; dependency, signing/notarization, Codex runtime-only, updater-out-of-scope, and final DMG/app launch cleanup gates are coherent.
- Fix/retry summary: patched `.omo/plans/beta-release-distribution.md` to add exact Todo 5 evidence command and forbidden-token failure check, exact Todo 9 temp-manifest failure command, and F4 scope scan command that writes evidence through `tee`. Native Momus approved after re-review. Independent Codex CLI review remains blocked pending isolated authentication.
