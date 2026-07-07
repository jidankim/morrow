# beta-release-distribution - Work Plan

## TL;DR (For humans)
**What you'll get:** A beta release path that produces both a macOS app bundle and a DMG, so testers can install Morrow without cloning the repo or running a dev server. Tester setup will cover app permissions and Codex CLI login, while build tools stay limited to maintainers and CI.

**Why this approach:** Rust, Xcode, and Tauri are build-time requirements; they should disappear from the normal beta-tester path once a packaged app exists. Codex CLI is different: it is a user-owned runtime provider and credential boundary, so Morrow should detect and guide it rather than bundle, install, or manage it.

**What it will NOT do:** It will not auto-install Codex CLI, copy or manage `~/.codex` credentials, require packaged-beta testers to install Rust/Xcode/Tauri CLI, add auto-updates, or claim real Messages-to-Calendar success from packaging alone.

**Effort:** Medium
**Risk:** Medium - macOS signing/notarization and desktop permission behavior are external gates that can block a release even when the code builds.
**Decisions to sanity-check:** Signed/notarized artifacts are the v0.1 beta release gate. Unsigned artifacts may be used only for local diagnostics and must not be shipped as tester builds. Auto-updates stay out of scope.

Your next move: start work with `$start-work .omo/plans/beta-release-distribution.md`, or request a high-accuracy plan review first. Full execution detail follows below.

---

> TL;DR (machine): Medium effort, medium risk. Enable macOS app+DMG beta packaging, signed/notarized release gating, Codex CLI runtime-prereq QA, tester docs, and desktop/DMG manual QA evidence.

## Scope
### Must have
- Tauri beta packaging path that produces both `Morrow.app` and a `.dmg` for macOS direct distribution.
- Repeatable package scripts for maintainers/CI that keep SDK/linker environment parity with existing Tauri scripts.
- Signed/notarized release gate for tester artifacts using `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD`, and `APPLE_TEAM_ID` or the equivalent Tauri-supported signing credentials.
- Explicit BLOCKED result when signing/notarization credentials are absent; unsigned artifacts are diagnostic-only and cannot satisfy beta-release success criteria.
- Codex CLI runtime readiness remains user-owned: Morrow detects missing CLI, not logged in, timeout, unknown failure, ready, and redaction states.
- Tester-facing docs that say packaged beta testers install the app/DMG, grant macOS permissions, and install/login Codex CLI only if testing provider-backed Sync Now.
- Maintainer/source-build docs that preserve `npm install`, Rust, Xcode/Command Line Tools, and SDK/linker prerequisites only for source builds.
- Agent-executed QA for config assertions, docs assertions, Codex readiness tests, artifact existence, signing/notarization, DMG mount/inspect/unmount, packaged app launch, screenshot capture, and cleanup.

### Must NOT have (guardrails, anti-slop, scope boundaries)
- Must not auto-install Codex CLI from the app or scripts.
- Must not bundle Codex CLI into `Morrow.app` or the DMG.
- Must not copy, print, migrate, delete, or otherwise manage `~/.codex`, `auth.json`, keychain Codex credentials, or access tokens.
- Must not require packaged-beta testers to clone the repo, install Node dependencies, install Rust, install Xcode, or run `npm run tauri:dev`.
- Must not add automatic updater configuration, updater artifacts, update channels, or hosting.
- Must not claim real Messages-to-Calendar production QA from package build, launch, or synthetic tests.
- Must not weaken existing privacy, Codex auth redaction, Full Disk Access, Calendar, or scan-readiness tests.
- Must not depend on currently clean branch history or overwrite unrelated future worktree changes.

## Verification strategy
> Zero human intervention - all verification is agent-executed.
- Test decision: TDD for new QA scripts and readiness/docs assertions; failing-first artifact/config proof for package behavior; tests-after only for pure docs wording that is immediately covered by docs QA.
- Unit/integration commands:
  - `npm test -- App.providerCredential.test.tsx`
  - `npm run build`
  - `cd src-tauri && SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk RUSTFLAGS="-C linker=/Library/Developer/CommandLineTools/usr/bin/cc" cargo test --test native_codex_auth`
  - `node scripts/beta-release-docs-qa.mjs README.md docs/beta-testing.md .omo/evidence/task-4-beta-release-distribution.md`
  - `node scripts/release-config-qa.mjs src-tauri/tauri.conf.json package.json .omo/evidence/task-1-beta-release-distribution.md`
  - `node scripts/release-artifact-qa.mjs src-tauri/target/release/bundle .omo/evidence/task-2-beta-release-distribution.md`
  - `scripts/release-signing-qa.sh src-tauri/target/release/bundle .omo/evidence/task-3-beta-release-distribution.md`
- Real-surface commands:
  - `npm run tauri:build:beta`
  - `scripts/release-dmg-install-qa.sh src-tauri/target/release/bundle .omo/evidence/task-7-beta-release-distribution.md`
  - `scripts/release-packaged-app-launch-qa.sh src-tauri/target/release/bundle .omo/evidence/task-8-beta-release-distribution.md`
- Evidence paths:
  - `.omo/evidence/task-1-beta-release-distribution.md` - release config/script assertions
  - `.omo/evidence/task-2-beta-release-distribution.md` - artifact discovery and Info.plist assertions
  - `.omo/evidence/task-3-beta-release-distribution.md` - signing/notarization gate
  - `.omo/evidence/task-4-beta-release-distribution.md` - tester docs assertions
  - `.omo/evidence/task-5-beta-release-distribution.md` - Codex readiness/redaction test output
  - `.omo/evidence/task-6-beta-release-distribution.md` - build transcript and artifact paths
  - `.omo/evidence/task-7-beta-release-distribution.md` - DMG mount/install/unmount transcript and cleanup receipt
  - `.omo/evidence/task-8-beta-release-distribution.md` plus `.omo/evidence/task-8-beta-release-distribution.png` - packaged app launch transcript and screenshot
  - `.omo/evidence/task-9-beta-release-distribution.md` - final release manifest/checklist QA
- Cleanup requirements:
  - Every DMG mount must be detached with `hdiutil detach`.
  - Every launched app process must be quit with `osascript` or killed by PID if normal quit fails.
  - Every temp app copy and temp mount staging directory must be removed.
  - Final QA must run `hdiutil info`, `pgrep -fl "Morrow|morrow"`, and `find /tmp -maxdepth 1 -name "morrow-beta-qa-*"` and record no leftover QA state.

## Execution strategy
### Parallel execution waves
> Target 5-8 todos per wave. Fewer than 3 (except the final) means you under-split.
- Wave 1: config, QA harnesses, docs, and Codex readiness contracts. Tasks 1-5 can mostly proceed in parallel after checking for file conflicts.
- Wave 2: build, artifact, signing, DMG, packaged app surface, and release manifest. Tasks 6-9 depend on Wave 1.
- Final wave: full plan compliance, code quality, real manual QA, and scope-fidelity review.

### Dependency matrix
| Todo | Depends on | Blocks | Can parallelize with |
| --- | --- | --- | --- |
| 1 | none | 2, 3, 6 | 4, 5 |
| 2 | 1 | 6, 7, 8, 9 | 3, 4, 5 |
| 3 | 1 | 6, 9 | 2, 4, 5 |
| 4 | none | 9 | 1, 2, 3, 5 |
| 5 | none | 6, 8, 9 | 1, 2, 3, 4 |
| 6 | 1, 2, 3, 5 | 7, 8, 9 | none |
| 7 | 6 | 9 | 8 |
| 8 | 6 | 9 | 7 |
| 9 | 4, 6, 7, 8 | final verification | none |

## Todos
> Implementation + Test = ONE todo. Never separate.
<!-- APPEND TASK BATCHES BELOW THIS LINE WITH edit/apply_patch - never rewrite the headers above. -->
- [ ] 1. Configure beta bundle scripts and static config QA
  What to do / Must NOT do: Update `src-tauri/tauri.conf.json` so bundling is enabled for release builds, but do not enable updater artifacts. Add an explicit package script, recommended name `tauri:build:beta`, that runs the existing SDK/linker environment and invokes `tauri build --bundles app,dmg`. Keep source/dev scripts available. Add `scripts/release-config-qa.mjs` to assert: bundle is active, product name is `Morrow`, identifier is `dev.morrow.desktop`, no `bundle.createUpdaterArtifacts`, and `package.json` has a beta build script containing `tauri build --bundles app,dmg`.
  Parallelization: Wave 1 | Blocked by: none | Blocks: 2, 3, 6
  References (executor has NO interview context - be exhaustive): `src-tauri/tauri.conf.json:1-31`; `package.json:6-13`; Tauri v2 docs: `tauri build` generates bundles/installers; Tauri v2 docs: `npm run tauri build -- --bundles dmg`; draft assumptions in `.omo/drafts/beta-release-distribution.md`.
  Acceptance criteria (agent-executable): `node scripts/release-config-qa.mjs src-tauri/tauri.conf.json package.json .omo/evidence/task-1-beta-release-distribution.md` exits 0 and evidence records every assertion as PASS. `rg -n "createUpdaterArtifacts|updater" src-tauri package.json README.md docs scripts` must not find a new updater requirement except an explicit out-of-scope sentence.
  QA scenarios (name the exact tool + invocation): happy: `node scripts/release-config-qa.mjs src-tauri/tauri.conf.json package.json .omo/evidence/task-1-beta-release-distribution.md`; failure: `tmp="$(mktemp -d /tmp/morrow-release-config-fail-XXXXXX)" && printf '{"bundle":{"active":false}}' > "$tmp/tauri.conf.json" && printf '{"scripts":{}}' > "$tmp/package.json" && ! node scripts/release-config-qa.mjs "$tmp/tauri.conf.json" "$tmp/package.json" "$tmp/evidence.md" && rm -rf "$tmp"`; evidence `.omo/evidence/task-1-beta-release-distribution.md`.
  Commit: Y | `build(release): add beta bundle configuration`

- [ ] 2. Add artifact discovery and Info.plist QA
  What to do / Must NOT do: Add `scripts/release-artifact-qa.mjs` that inspects `src-tauri/target/release/bundle` after a build and verifies exactly one beta `Morrow.app` bundle and one DMG are present. It must inspect `Morrow.app/Contents/Info.plist` with `plutil` or a structured plist parser and assert `CFBundleIdentifier=dev.morrow.desktop`, product/display name is Morrow, executable exists under `Contents/MacOS`, and the DMG filename includes version `0.1.0` or the version from `src-tauri/tauri.conf.json`. It must not parse plist with fragile regex.
  Parallelization: Wave 1 | Blocked by: 1 | Blocks: 6, 7, 8, 9
  References: `src-tauri/tauri.conf.json:3-5`; `package.json:1-3`; `README.md:64-68`; Tauri v2 docs: DMG is the common outside-App-Store macOS installer.
  Acceptance criteria: `node scripts/release-artifact-qa.mjs src-tauri/target/release/bundle .omo/evidence/task-2-beta-release-distribution.md` exits 0 after a successful beta build; a missing DMG or missing app fixture exits nonzero and records FAIL evidence.
  QA scenarios: happy after Todo 6: `node scripts/release-artifact-qa.mjs src-tauri/target/release/bundle .omo/evidence/task-2-beta-release-distribution.md`; failure: `tmp="$(mktemp -d /tmp/morrow-artifact-fail-XXXXXX)" && mkdir -p "$tmp/macos/Morrow.app/Contents/MacOS" && ! node scripts/release-artifact-qa.mjs "$tmp" "$tmp/evidence.md" && rm -rf "$tmp"`; evidence `.omo/evidence/task-2-beta-release-distribution.md`.
  Commit: Y | `test(release): add beta artifact qa`

- [ ] 3. Add signed/notarized release gate QA
  What to do / Must NOT do: Add `scripts/release-signing-qa.sh` that takes a bundle directory and evidence path, finds the built `Morrow.app` and DMG, then verifies signing and notarization. It must run `codesign --verify --deep --strict --verbose=2 "$APP"`, `codesign -dv "$APP"`, `spctl --assess --type execute --verbose=4 "$APP"`, `xcrun stapler validate "$APP"`, and `spctl --assess --type open --context context:primary-signature --verbose=4 "$DMG"`. It must return 0 only when release artifacts are signed/notarized. If signing credentials or notarization are absent, it must record BLOCKED and exit nonzero; unsigned artifacts are local diagnostics only, not beta-release success.
  Parallelization: Wave 1 | Blocked by: 1 | Blocks: 6, 9
  References: Tauri v2 docs: `APPLE_SIGNING_IDENTITY` or `bundle.macOS.signingIdentity` for signing; Tauri v2 docs: `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID` for Apple-ID notarization; `scripts/tauri-dev-signed-runner.sh:74-76` shows current ad-hoc dev signing only and must not be treated as release signing.
  Acceptance criteria: On signed/notarized artifacts, `scripts/release-signing-qa.sh src-tauri/target/release/bundle .omo/evidence/task-3-beta-release-distribution.md` exits 0 and records PASS for every command. On an unsigned diagnostic fixture, the script exits nonzero and records BLOCKED or FAIL without leaking secret environment values.
  QA scenarios: happy after Todo 6 with signing credentials: `scripts/release-signing-qa.sh src-tauri/target/release/bundle .omo/evidence/task-3-beta-release-distribution.md`; failure: `tmp="$(mktemp -d /tmp/morrow-signing-fail-XXXXXX)" && mkdir -p "$tmp/macos/Morrow.app" && touch "$tmp/Morrow_0.1.0_aarch64.dmg" && ! scripts/release-signing-qa.sh "$tmp" "$tmp/evidence.md" && rm -rf "$tmp"`; evidence `.omo/evidence/task-3-beta-release-distribution.md`.
  Commit: Y | `test(release): require signed notarized beta artifacts`

- [ ] 4. Write tester and maintainer prerequisite docs with docs QA
  What to do / Must NOT do: Add `docs/beta-testing.md` and update `README.md` so the primary beta flow is: download/open DMG, drag/install `Morrow.app`, open Morrow, grant Full Disk Access and Calendar access, install Codex CLI and run `codex login` only for provider-backed Sync Now, then run the manual QA flow. Keep `npm install`, Rust, Xcode/Command Line Tools, SDKROOT, and Tauri dev commands clearly labeled as maintainer/source-build-only. Add `scripts/beta-release-docs-qa.mjs` to assert required phrases and forbidden claims. Must not state that Morrow grants permissions automatically, bundles Codex CLI, owns Codex login, or that package launch proves real Messages-to-Calendar QA.
  Parallelization: Wave 1 | Blocked by: none | Blocks: 9
  References: `README.md:7-13`, `README.md:20`, `README.md:29-42`, `README.md:46-56`, `README.md:58-80`, `README.md:149-167`; `scripts/readme-full-disk-access-recovery-qa.mjs:7-22`.
  Acceptance criteria: `node scripts/beta-release-docs-qa.mjs README.md docs/beta-testing.md .omo/evidence/task-4-beta-release-distribution.md` exits 0 and records PASS for tester-vs-maintainer prerequisite separation.
  QA scenarios: happy: `node scripts/beta-release-docs-qa.mjs README.md docs/beta-testing.md .omo/evidence/task-4-beta-release-distribution.md`; failure: `tmp="$(mktemp -d /tmp/morrow-docs-fail-XXXXXX)" && printf 'Testers must install Rust and Xcode. Morrow bundles Codex CLI.\n' > "$tmp/README.md" && printf '' > "$tmp/beta-testing.md" && ! node scripts/beta-release-docs-qa.mjs "$tmp/README.md" "$tmp/beta-testing.md" "$tmp/evidence.md" && rm -rf "$tmp"`; also run existing `node scripts/readme-full-disk-access-recovery-qa.mjs README.md .omo/evidence/task-4-full-disk-access-readme.md`; evidence `.omo/evidence/task-4-beta-release-distribution.md`.
  Commit: Y | `docs(release): document beta tester prerequisites`

- [ ] 5. Preserve and extend Codex CLI readiness/redaction contracts
  What to do / Must NOT do: Keep Codex CLI as a runtime prerequisite. Do not add installer/downloader code. Extend tests only where needed so missing CLI, not logged in, ready, timeout, unknown failure, misleading success output, noisy output, and redaction remain covered. Ensure Settings displays static `codex login` guidance and never raw `codex login status` output or token-looking data. If implementation changes are unnecessary, still run and record the existing tests.
  Parallelization: Wave 1 | Blocked by: none | Blocks: 6, 8, 9
  References: `src-tauri/src/native_bridge/codex_auth.rs:49-67`; `src-tauri/src/native_bridge/codex_auth/readiness.rs:3-49`; `src-tauri/tests/native_codex_auth.rs:40-258`; `src/App.providerCredential.test.tsx:103-174`; `src/SettingsProviderCredentialSection.tsx:46-133`; OpenAI Codex manual auth section: CLI login caches credentials locally and they are sensitive user credentials.
  Acceptance criteria: `cd src-tauri && SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk RUSTFLAGS="-C linker=/Library/Developer/CommandLineTools/usr/bin/cc" cargo test --test native_codex_auth` passes. `npm test -- App.providerCredential.test.tsx` passes. Evidence contains no `codex_access_token`, `sk-`, `auth.json`, or raw `codex login status --raw` text.
  QA scenarios: happy: `mkdir -p .omo/evidence && { printf '## Native Codex auth QA\n\n'; { cd src-tauri && SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk RUSTFLAGS="-C linker=/Library/Developer/CommandLineTools/usr/bin/cc" cargo test --test native_codex_auth; } 2>&1; printf '\n## Provider UI QA\n\n'; npm test -- App.providerCredential.test.tsx 2>&1; } | tee .omo/evidence/task-5-beta-release-distribution.md` and then `! rg -n "codex_access_token|sk-[A-Za-z0-9]|auth\\.json|codex login status --raw" .omo/evidence/task-5-beta-release-distribution.md`; failure: `tmp="$(mktemp -d /tmp/morrow-codex-redaction-fail-XXXXXX)" && printf 'raw codex_access_token=leaked\ncodex login status --raw\n' > "$tmp/evidence.md" && rg -n "codex_access_token|codex login status --raw" "$tmp/evidence.md" && rm -rf "$tmp"` proves the forbidden-token check detects leaked output; evidence `.omo/evidence/task-5-beta-release-distribution.md`.
  Commit: Y | `test(auth): lock codex readiness release contract`

- [ ] 6. Produce signed beta app and DMG artifacts
  What to do / Must NOT do: Run the beta build script and produce release artifacts under `src-tauri/target/release/bundle`. The build must use the same SDK/linker environment as existing scripts. Release success requires signing/notarization credentials. If `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD`, or `APPLE_TEAM_ID` are missing and no equivalent Tauri-supported signing setup is present, record BLOCKED and stop release execution; do not downgrade to unsigned tester shipping.
  Parallelization: Wave 2 | Blocked by: 1, 2, 3, 5 | Blocks: 7, 8, 9
  References: `package.json:11-12`; `src-tauri/tauri.conf.json:1-31`; Tauri v2 docs: build command and macOS signing/notarization environment variables.
  Acceptance criteria: `npm run tauri:build:beta` exits 0; `node scripts/release-artifact-qa.mjs src-tauri/target/release/bundle .omo/evidence/task-2-beta-release-distribution.md` exits 0; `scripts/release-signing-qa.sh src-tauri/target/release/bundle .omo/evidence/task-3-beta-release-distribution.md` exits 0.
  QA scenarios: happy: `npm run tauri:build:beta 2>&1 | tee .omo/evidence/task-6-beta-release-distribution.md`; then run artifact and signing QA commands from Todos 2 and 3; failure: unset signing variables in a clean shell and confirm the release script records BLOCKED instead of PASS, `env -u APPLE_SIGNING_IDENTITY -u APPLE_ID -u APPLE_PASSWORD -u APPLE_TEAM_ID npm run tauri:build:beta 2>&1 | tee .omo/evidence/task-6-unsigned-blocked.txt` or equivalent project script behavior; evidence `.omo/evidence/task-6-beta-release-distribution.md`.
  Commit: N | covered by earlier build/test commits unless script changes are discovered here

- [ ] 7. Verify DMG mount, app presence, and cleanup
  What to do / Must NOT do: Add and run `scripts/release-dmg-install-qa.sh`. It must locate the DMG, mount it with `hdiutil attach -nobrowse`, assert mounted volume contains `Morrow.app`, assert the app bundle has the expected `Contents/MacOS` executable, optionally assert `/Applications` symlink if the DMG layout provides it, then detach the volume. It must record cleanup receipts and fail if a mount remains. It must not copy into real `/Applications`.
  Parallelization: Wave 2 | Blocked by: 6 | Blocks: 9 | Can parallelize with: 8
  References: Tauri v2 docs: DMG direct distribution; `README.md:64-68`.
  Acceptance criteria: `scripts/release-dmg-install-qa.sh src-tauri/target/release/bundle .omo/evidence/task-7-beta-release-distribution.md` exits 0, evidence includes mount path, assertions, `hdiutil detach` result, and post-cleanup `hdiutil info` check.
  QA scenarios: happy: `scripts/release-dmg-install-qa.sh src-tauri/target/release/bundle .omo/evidence/task-7-beta-release-distribution.md`; failure: `tmp="$(mktemp -d /tmp/morrow-dmg-fail-XXXXXX)" && touch "$tmp/not-a-real.dmg" && ! scripts/release-dmg-install-qa.sh "$tmp" "$tmp/evidence.md" && rm -rf "$tmp"`; cleanup: `hdiutil info | tee -a .omo/evidence/task-7-beta-release-distribution.md`; evidence `.omo/evidence/task-7-beta-release-distribution.md`.
  Commit: Y | `test(release): add dmg install qa`

- [ ] 8. Verify packaged app launch through desktop automation
  What to do / Must NOT do: Add and run `scripts/release-packaged-app-launch-qa.sh`. It must find the packaged `Morrow.app`, launch it with `open -n`, wait for the app process/window, capture a screenshot with `screencapture`, record the running process identity, then quit Morrow via `osascript` and verify no leftover process remains. It must use a copied temp app or mounted app path, not the source dev binary. It must not require real Messages/Calendar data to pass; launch and visible app shell are the proof here.
  Parallelization: Wave 2 | Blocked by: 6 | Blocks: 9 | Can parallelize with: 7
  References: `README.md:64-68`; `src-tauri/tauri.conf.json:12-23`; `src-tauri/tests/macos_metadata.rs:59-89`.
  Acceptance criteria: `scripts/release-packaged-app-launch-qa.sh src-tauri/target/release/bundle .omo/evidence/task-8-beta-release-distribution.md` exits 0, creates `.omo/evidence/task-8-beta-release-distribution.png`, evidence records launch, screenshot path, quit, and no leftover Morrow process.
  QA scenarios: happy: `scripts/release-packaged-app-launch-qa.sh src-tauri/target/release/bundle .omo/evidence/task-8-beta-release-distribution.md`; failure: `tmp="$(mktemp -d /tmp/morrow-launch-fail-XXXXXX)" && mkdir -p "$tmp/macos" && ! scripts/release-packaged-app-launch-qa.sh "$tmp" "$tmp/evidence.md" && rm -rf "$tmp"`; cleanup: `pgrep -fl "Morrow|morrow" | tee -a .omo/evidence/task-8-beta-release-distribution.md || true`; evidence `.omo/evidence/task-8-beta-release-distribution.md` and `.omo/evidence/task-8-beta-release-distribution.png`.
  Commit: Y | `test(release): add packaged app launch qa`

- [ ] 9. Generate release manifest and final tester checklist
  What to do / Must NOT do: Add `docs/beta-release-checklist.md` or a release manifest file that records version, branch, commit SHA, artifact paths, artifact hashes, signing/notarization status, QA evidence paths, and explicit limitations. The limitation section must say package QA does not prove real Messages-to-Calendar event creation. Add a manifest QA script if needed to assert hashes, paths, and required limitations.
  Parallelization: Wave 2 | Blocked by: 4, 6, 7, 8 | Blocks: final verification
  References: `README.md:7-13`, `README.md:147-155`, `README.md:157-167`; all task evidence paths above.
  Acceptance criteria: `node scripts/release-manifest-qa.mjs docs/beta-release-checklist.md src-tauri/target/release/bundle .omo/evidence/task-9-beta-release-distribution.md` exits 0; manifest includes SHA-256 hashes for app/DMG artifacts, signing result, notarization/stapling result, and every QA evidence path.
  QA scenarios: happy: `node scripts/release-manifest-qa.mjs docs/beta-release-checklist.md src-tauri/target/release/bundle .omo/evidence/task-9-beta-release-distribution.md`; failure: `tmp="$(mktemp -d /tmp/morrow-manifest-fail-XXXXXX)" && printf '# Beta release checklist\n\nArtifact: missing-limitations-only\n' > "$tmp/beta-release-checklist.md" && mkdir -p "$tmp/bundle" && ! node scripts/release-manifest-qa.mjs "$tmp/beta-release-checklist.md" "$tmp/bundle" "$tmp/evidence.md" && rg -n "does not prove real Messages-to-Calendar|limitation|missing" "$tmp/evidence.md" && rm -rf "$tmp"`; evidence `.omo/evidence/task-9-beta-release-distribution.md`.
  Commit: Y | `docs(release): add beta release checklist`

## Final verification wave
> Runs in parallel after ALL todos. ALL must APPROVE. Surface results and wait for the user's explicit okay before declaring complete.
- [ ] F1. Plan compliance audit
  Command/evidence: `node scripts/release-config-qa.mjs src-tauri/tauri.conf.json package.json .omo/evidence/f1-release-plan-compliance.md`; `node scripts/beta-release-docs-qa.mjs README.md docs/beta-testing.md .omo/evidence/f1-release-plan-compliance.md`; `node scripts/release-manifest-qa.mjs docs/beta-release-checklist.md src-tauri/target/release/bundle .omo/evidence/f1-release-plan-compliance.md`. APPROVE only if every todo acceptance criterion has an evidence path and no updater/Codex-auto-install scope creep exists.
- [ ] F2. Code quality review
  Command/evidence: `npm test 2>&1 | tee .omo/evidence/f2-npm-test.txt`; `npm run build 2>&1 | tee .omo/evidence/f2-npm-build.txt`; `{ cd src-tauri && SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk RUSTFLAGS="-C linker=/Library/Developer/CommandLineTools/usr/bin/cc" cargo test --test native_codex_auth; } 2>&1 | tee .omo/evidence/f2-native-codex-auth.txt`; run any new script unit/fixture tests. APPROVE only if all pass without skipped/weakened tests.
- [ ] F3. Real manual QA
  Command/evidence: `scripts/release-dmg-install-qa.sh src-tauri/target/release/bundle .omo/evidence/f3-dmg-install.md`; `scripts/release-packaged-app-launch-qa.sh src-tauri/target/release/bundle .omo/evidence/f3-packaged-app-launch.md`; screenshot `.omo/evidence/f3-packaged-app-launch.png`. APPROVE only if app launches from packaged artifact, screenshot is nonblank, and cleanup receipts show no mounted DMG or app process.
- [ ] F4. Scope fidelity
  Command/evidence: `mkdir -p .omo/evidence && { printf '## Scope fidelity scan\n\n'; rg -n "createUpdaterArtifacts|updater|auto.?update|curl .*codex|brew install .*codex|npm install -g .*codex|auth\\.json|~/.codex|Rust|Xcode|tauri dev" README.md docs src src-tauri scripts package.json || true; } | tee .omo/evidence/f4-scope-fidelity.md`. APPROVE only if every recorded match is expected: updater appears only as out-of-scope/deferred language, Codex CLI is user-installed/login-only, credentials are never handled, Rust/Xcode/Tauri dev appear only in maintainer/source-build sections, and packaged tester docs do not require source-build tools.

## Commit strategy
- Commit 1: `build(release): add beta bundle configuration`
  - Include `src-tauri/tauri.conf.json`, `package.json`, and release config QA script.
- Commit 2: `test(release): add artifact signing qa`
  - Include artifact discovery, signing/notarization, DMG install, and packaged app launch QA scripts.
- Commit 3: `test(auth): lock codex readiness release contract`
  - Include Codex readiness/redaction test additions if needed.
- Commit 4: `docs(release): document beta tester flow`
  - Include README/docs updates, docs QA script, checklist/manifest, and release limitation language.
- Every commit must pass the relevant local tests for touched files before the next commit.
- Final commit footer: `Plan: .omo/plans/beta-release-distribution.md`

## Success criteria
- A maintainer can run one documented beta build command and get both `Morrow.app` and a DMG under `src-tauri/target/release/bundle`.
- Signed/notarized artifacts are required for beta tester distribution; missing signing/notarization records BLOCKED, not PASS.
- Packaged-beta tester docs do not require cloning the repo, `npm install`, Rust, Xcode, Tauri CLI, or `npm run tauri:dev`.
- Codex CLI is handled as a runtime prerequisite with app/docs guidance and tests for missing CLI, not logged in, ready, timeout, unknown failure, and redaction.
- No code or docs path bundles Codex CLI, auto-installs Codex CLI, touches `~/.codex`, or exposes raw Codex auth output.
- DMG mount/inspect/unmount QA passes and records cleanup.
- Packaged app launch QA passes through OS-level automation and records a screenshot plus cleanup.
- `npm test`, `npm run build`, and `cargo test --test native_codex_auth` pass.
- Release manifest/checklist includes artifact paths, hashes, signing/notarization status, QA evidence paths, and the limitation that packaging QA does not prove real Messages-to-Calendar event creation.
