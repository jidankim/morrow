# Wider Product Rollout Release Checklist

## Current Handoff

- Version: 0.1.0
- Branch: codex/wider-product-rollout
- Commit: 0cdf56863f2f7f0cb6f6ab8d7a136dafd621c981 with uncommitted rollout handoff edits pending review
- Manifest State: beta
- Default Availability Allowed: false
- Beta Gate: BLOCKED
- Phase 6 Fixture Gate: PASS, fixture-monitoring input only
- Rollout Gate: BLOCKED
- deployment_action=none
- Deployment Action: none
- Manifest Source: `docs/wider-product-rollout-manifest.json`
- Final Smoke: `.omo/evidence/phase-7-wider-product-rollout/final-smoke/summary.txt`
- Privacy Evidence: `.omo/evidence/phase-7-wider-product-rollout/final-smoke/privacy-inspect.txt`
- Docs Evidence: `.omo/evidence/phase-7-wider-product-rollout/final-smoke/docs-qa.md`
- Runbook Evidence: `.omo/evidence/phase-7-wider-product-rollout/final-smoke/runbook-qa.md`

## Artifact Receipts

| Artifact | Path | SHA-256 |
| --- | --- | --- |
| Final smoke summary | `.omo/evidence/phase-7-wider-product-rollout/final-smoke/summary.txt` | `1505357c52614511d1f7d2ddedb61d0fdaf13f2e20ca691293480dae5b83e185` |
| Beta readiness | `.omo/evidence/phase-7-wider-product-rollout/final-smoke/beta-readiness.json` | `72d7fddfb8cc9f1a8c9961488ab4b342f8eb1e8072b7dd7550c59b88dbc2dafa` |
| Rollout regression gate | `.omo/evidence/phase-7-wider-product-rollout/final-smoke/regression-gate.json` | `bfaa245e16572b3496f129a00ba4fd87094251fc25bb324a4b4aa3bcc0b50feb` |
| Negative matrix | `.omo/evidence/phase-7-wider-product-rollout/final-smoke/negative-matrix.json` | `210e62d494b6a53443621f54245edf141209fe638886abb2a4e31b7a84409db7` |
| Deployment action | `.omo/evidence/phase-7-wider-product-rollout/final-smoke/deployment-action.txt` | `7084378e938466d9b4da1558c745ff799bd0245b100ee36b19fc6dae2c8776d2` |
| Privacy inspection | `.omo/evidence/phase-7-wider-product-rollout/final-smoke/privacy-inspect.txt` | `29c2dcacaafd1dba85ca2005c68ce53c6097c56d48d56265429adbca6fcd7c12` |

## Gate Status

- Manifest validation: PASS for static local manifest, telemetry disabled, remote config disabled, fail-closed kill switches, and default availability false.
- Beta gate: BLOCKED because signed Morrow.app, signed/notarized DMG, notarized/stapled beta artifact, beta bundle directory, beta release manifest/checklist, and Apple signing/notarization prerequisites are missing.
- Phase 6 fixture gate: PASS as fixture-monitoring input only; it is not deployed rollout evidence.
- Rollout gate: BLOCKED because beta readiness is BLOCKED.
- Privacy/docs/runbook evidence: PASS for privacy inspection, docs QA, and runbook QA.
- Deployment action: none; no artifact publish, remote config launch, telemetry upload, provider network action, live service action, Messages read, Calendar mutation, or credential management was performed.

## Limitations

- Default availability is false and must remain unavailable until signed/notarized beta artifacts exist and all beta and rollout gates pass.
- The current state is beta/blocked, not staged and not default available.
- The correction UI remains future work.
- The full live Messages-to-Calendar approval trajectory remains future work.
- Automatic promotion remains future work and is disabled.
- Live cloud rollout and live remote config remain future work.
- Phase 6 remains fixture-backed monitoring evidence only.
