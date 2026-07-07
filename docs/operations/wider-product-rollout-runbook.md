# Wider Product Rollout Runbook

This runbook is the operator handoff for Phase 7 wider product rollout. The rollout source of truth is `docs/wider-product-rollout-manifest.json`, a static manifest changed only through reviewed release changes. Live remote config is out of scope. Automatic promotion is out of scope. Operators must never silently publish, expand cohorts, or make the default download available without a reviewed release change and recorded evidence.

Every gate-only pass records `deployment_action=none`, which means gate-only/no publish. An owner-approved static manifest change records `deployment_action=release_manifest_update`, which means owner-approved manifest update through release review. No other deployment action is valid for this phase.

## Operating Rules

- Beta and default availability require signed and notarized artifacts, DMG install QA, packaged launch QA, release manifest QA, public docs QA, retention QA, runbook QA, and rollout gates.
- `paused`, `rolled_back`, and `blocked` are fail-closed states. They stop promotion, default availability, provider/router changes, and any release manifest expansion until a reviewed release change records the recovery evidence.
- Telemetry and upload paths remain disabled. Kill switches can only disable rollout or provider paths; they must not enable telemetry or upload.
- Operators change only the static manifest and public handoff docs through release review. The static manifest changed only through reviewed release changes; there is no live remote config edit, hosted cohort control, App Store/TestFlight path, auto-updater, or automatic promotion.
- Evidence must avoid raw message bodies, raw calendar content, raw provider JSON, raw prompts, credentials, native IDs, private paths, and unredacted titles.

## Beta Release Handoff

Action owner: release owner.

Trigger: signed and notarized beta artifacts are ready for tester handoff.

Steps:

1. Run the beta readiness gate and confirm signed/notarized app and DMG evidence, DMG install QA, packaged launch QA, and release manifest QA.
2. Confirm `docs/beta-testing.md` remains the tester handoff URL and does not require source-build tools for packaged beta testers.
3. Confirm the manifest remains in `beta` unless the owner-approved change explicitly updates it.
4. Record `deployment_action=none` for gate-only verification, or `deployment_action=release_manifest_update` only when the release owner approves a reviewed manifest change.

Fail-closed state: `blocked` if any signed/notarized artifact, install, launch, manifest, privacy, or docs evidence is missing.

Cleanup instruction: archive only sanitized QA receipts under the rollout evidence directory, remove temporary beta QA staging directories, and confirm no mounted DMG or launched Morrow process remains.

## Staged Cohort Expansion

Action owner: rollout owner.

Trigger: beta evidence passes and the owner requests a staged expansion.

Steps:

1. Confirm the current manifest state is `beta` and the only allowed forward transition is `beta` to `staged`.
2. Review privacy, retention, runbook, beta readiness, support readiness, and Phase 6 fixture-monitoring evidence.
3. Prepare a reviewed static manifest change that describes the cohort size and evidence path.
4. Record `deployment_action=release_manifest_update` only after owner approval. Otherwise record `deployment_action=none`.

Fail-closed state: `paused` if support load, privacy risk, provider/router risk, or stale evidence appears before or after expansion.

Cleanup instruction: remove draft cohort notes that contain private support details, keep only sanitized aggregate evidence, and leave the manifest unchanged when approval is absent.

## Pause And Resume

Action owner: incident commander.

Trigger: support, privacy, provider, model, router, package, or evidence quality signals require stopping rollout.

Steps:

1. Move the manifest to `paused` through a reviewed release change, or leave it unchanged and record a gate failure if no owner approval exists.
2. Disable promotion and default availability using the manifest kill switches.
3. Resume only after the incident owner records root cause, evidence, cleanup, support communication, and a reviewed static manifest update.
4. Record `deployment_action=none` for investigation-only runs; record `deployment_action=release_manifest_update` for the owner-approved pause or resume change.

Fail-closed state: `paused` until the incident commander and release owner both accept the recovery evidence.

Cleanup instruction: close temporary incident workspaces, redact private support details from evidence, and retain the sanitized incident receipt.

## Rollback

Action owner: release owner.

Trigger: a staged or beta release must be withdrawn because artifact, privacy, provider, support, or default-availability evidence no longer passes.

Steps:

1. Set the manifest state to `rolled_back` through a reviewed release change.
2. Remove public handoff pointers to the withdrawn artifact in the static docs or manifest update.
3. Keep the rollback receipt, affected artifact identifiers, signed/notarized status, and tester communication record.
4. Record `deployment_action=release_manifest_update` only for the approved rollback change; otherwise record `deployment_action=none`.

Fail-closed state: `rolled_back` blocks cohort expansion, default availability, and provider/model/router rollout changes.

Cleanup instruction: remove local rollback staging directories, verify no withdrawn artifact is presented as the default download, and keep only sanitized evidence.

## Default Availability Gate

Action owner: release owner.

Trigger: staged rollout is complete and the owner asks to mark the signed/notarized DMG as default-available.

Steps:

1. Confirm the manifest state is `staged`; `beta` must not skip directly to `default_available`.
2. Confirm signed/notarized beta artifacts, DMG install QA, packaged launch QA, release manifest QA, privacy docs QA, retention QA, runbook QA, regression gate, and kill-switch checks all pass.
3. Confirm default availability means only the static manifest and public docs may identify the signed/notarized DMG as the default download.
4. Record `deployment_action=release_manifest_update` only for the owner-approved reviewed manifest change.

Fail-closed state: `blocked` if any prerequisite is missing, stale, or contradicted.

Cleanup instruction: remove draft default-download links from unapproved docs, preserve sanitized gate evidence, and keep `default_availability.allowed=false` until every gate passes.

## Privacy Incident Triage

Action owner: privacy owner.

Trigger: evidence, logs, screenshots, support tickets, or docs may contain raw private content or credential material.

Steps:

1. Pause rollout gates and keep telemetry/upload disabled.
2. Treat the report as sensitive input. Do not copy raw message bodies, raw calendar content, raw provider JSON, raw prompts, credentials, native IDs, private paths, or unredacted titles into evidence.
3. Redact or quarantine affected local evidence, record only sanitized class, path category, and remediation status, and rerun docs/privacy/runbook QA.
4. Escalate to the release owner before any manifest change.

Fail-closed state: `paused` until the privacy owner signs off on sanitized evidence and cleanup.

Cleanup instruction: delete temporary raw triage files, retain sanitized receipts only, and document any redaction in the evidence cleanup receipt.

## Provider Model Router Incident Triage

Action owner: provider owner.

Trigger: provider path, model selection, router behavior, Codex readiness, malformed output, stale state, latency, or regression signals threaten rollout quality.

Steps:

1. Use kill switches to block provider changes and model/router changes.
2. Confirm Codex credentials remain user-owned and are not read, managed, logged out, copied, or deleted by Morrow.
3. Use deterministic fixtures and sanitized receipts before considering any release manifest update.
4. Record `deployment_action=none` unless the release owner approves a manifest state or kill-switch update.

Fail-closed state: `blocked` for provider/model/router changes until regression and privacy gates pass.

Cleanup instruction: remove temporary provider fixtures that contain sensitive data, keep only sanitized aggregate receipts, and verify telemetry/upload remain disabled.

## Support Escalation

Action owner: support lead.

Trigger: tester reports indicate install, launch, permissions, Codex readiness, provider-backed Sync Now, Calendar proposal, privacy, or documentation failures.

Steps:

1. Classify the issue as install/artifact, permission, Codex readiness, provider/model/router, privacy, docs, or product behavior.
2. Escalate privacy issues to the privacy owner, provider/model/router issues to the provider owner, and artifact/default-availability issues to the release owner.
3. Keep support evidence sanitized and avoid private content examples.
4. Request pause or rollback when support cannot safely route testers.

Fail-closed state: `paused` when support cannot provide safe tester guidance or privacy-safe reproduction steps.

Cleanup instruction: redact support transcripts before evidence retention, delete temporary raw support exports, and keep the sanitized escalation receipt.

## Tester Communication

Action owner: tester communications owner.

Trigger: beta handoff, staged expansion, pause, resume, rollback, privacy incident, support issue, or default-availability request changes tester instructions.

Steps:

1. Use `docs/beta-testing.md` as the packaged beta tester source.
2. Tell testers that package launch only proves the app opens; real Messages-to-Calendar QA still requires Messages access, provider readiness, Calendar access, EventKit readback, and cleanup.
3. Communicate pauses, rollbacks, and default-availability status only after the release owner approves the static manifest or docs change.
4. Record `deployment_action=none` unless the communication accompanies an owner-approved manifest update.

Fail-closed state: `paused` if tester instructions would overclaim readiness or expose unsafe artifacts.

Cleanup instruction: retire outdated drafts, remove unapproved artifact links, and retain the sanitized tester communication receipt.

## Evidence Retention

Action owner: evidence owner.

Trigger: every gate, failure, incident, pause, resume, rollback, or release manifest update.

Steps:

1. Store receipts under `.omo/evidence/phase-7-wider-product-rollout/` with scenario, invocation, binary observable, result, and artifact path.
2. Treat `.omo/evidence` as planning evidence, not product runtime diagnostics.
3. Keep product runtime diagnostics separate from rollout planning evidence and ensure Delete All scope remains bounded to Morrow-owned runtime data.
4. Record evidence freshness and rerun stale-state checks for repeated gate runs.

Fail-closed state: `blocked` when evidence is missing, stale, empty, malformed, or contains private content.

Cleanup instruction: remove temp directories, record cleanup receipts, and redact or quarantine any evidence that includes raw private content or secrets.

## Kill-Switch Use

Action owner: incident commander.

Trigger: any gate, incident, support signal, provider/model/router concern, privacy report, or artifact issue requires rollout stop.

Steps:

1. Use manifest kill switches for telemetry, upload, remote config, staged promotion, default availability, model/router changes, and provider changes.
2. Confirm kill switches fail closed and cannot make telemetry or upload available.
3. Stop promotion before triage, then require a reviewed static manifest change to resume.
4. Record `deployment_action=none` for gate-only kill-switch verification or `deployment_action=release_manifest_update` for owner-approved static manifest changes.

Fail-closed state: `paused` or `blocked` until the incident commander confirms mitigation and cleanup evidence.

Cleanup instruction: remove temporary switch-test fixtures, keep the sanitized kill-switch receipt, and verify the manifest still has live remote config disabled.
