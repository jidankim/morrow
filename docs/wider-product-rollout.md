# Wider Product Rollout

This rollout is documentation- and evidence-gated. The public claim is intentionally conservative: Morrow has local and fixture-backed QA coverage, while live Messages-to-Calendar completion, beta distribution, and default availability remain gated by the checks below.

## Claim Boundaries

- Local diagnostics are private local files on the user's Mac.
- Telemetry and upload paths are disabled by default.
- Delete All removes Morrow-owned diagnostics and Morrow-owned legacy provider markers only.
- Codex owns the user login for the Codex CLI provider path.
- Morrow does not store provider tokens and does not read, manage, log out, or delete Codex credentials.
- Messages Full Disk Access and Calendar access are separate macOS permissions.
- Phase 6 is fixture-backed monitoring only.
- The full live Messages-to-Calendar approval trajectory remains separately gated.
- Beta and default availability require signed and notarized artifacts.

## What Phase 6 Proves

Phase 6 proves deterministic fixture-backed monitoring behavior: fixture inputs, version-sliced metrics, dashboard artifacts, release gates, alert receipts, privacy/canary rejection, negative matrix coverage, freshness checks, and cleanup. It is a local evidence claim.

Phase 6 does not prove deployed rollout, live telemetry upload, provider network behavior, Messages access, EventKit mutation, Calendar mutation, Reminders mutation, automatic release promotion, or full live Messages-to-Calendar completion.

## Public Docs Rules

Do not publish private message text, prompt transcripts, provider payloads, native database identifiers, or local storage paths. Use sanitized summaries and bounded evidence claims instead.

Do not treat diagnostic artifacts as beta artifacts. Diagnostic artifacts are for trusted diagnostic testing and are not a substitute for signed, notarized beta release artifacts.

Do not claim App Store, TestFlight, beta, or default availability until the required signed and notarized artifacts exist and the relevant release gates pass.

## Manual QA Gate

Package launch by itself only proves that the app opens. Real Messages-to-Calendar QA still requires Messages access, provider readiness for candidate extraction, Calendar access, and EventKit readback and cleanup.

The full live Messages-to-Calendar approval trajectory remains separately gated until a live environment verifies the complete path from a selected real message through candidate extraction, proposed Calendar creation, readback, approval or rejection, and cleanup.
