# AXI Phase 0 Feedback/Eval Baseline

Phase 0 is not full AXI compliance. It is the evidence baseline that now has Phase 1 production trace sink smoke coverage, while Phase 2 product correlation, Phase 3 lifecycle coverage, Phase 4 human approval/correction loop, and Phase 5 trajectory eval hardening remain future work.

This ledger maps the six AXI product-rule families to current local evidence and remaining gaps. The diagnostics runbook remains the detailed command and artifact source.

| AXI product-rule family | Current Phase 0 evidence | Still missing |
| --- | --- | --- |
| durable evidence-anchored candidates | Local feedback/eval smoke records candidate feedback events, labels, feature snapshots, metrics, and eval reports from fixture-backed runs. Diagnostics traces remain local artifacts with eval summaries derived from replayable data. Phase 1 production trace sink smoke proves the fake Codex production provider path can write sanitized local JSONL. | Phase 2 product correlation is still needed before production candidates have end-to-end durable evidence linkage. |
| risk-based human approval | Current production flow proposes calendar work through EventKit replay and keeps approval-bypassing mutation paths out of the diagnostics/eval layer. Reserved trace vocabulary names future correction and replay events without implementing them here. | Phase 4 must add the human approval/correction loop, including risk gates, correction capture, and auditable approval outcomes. |
| strict privacy boundaries | The local smokes avoid live vendor backends and real Messages data. Diagnostics exports are sanitized, privacy inspection rejects canary leakage, and feedback snapshots use bounded excerpts plus metadata rather than full private content. Phase 1 production trace sink smoke uses the fake Codex production fixture and rejects an injected raw canary before sanitization. | Phase 2 must preserve these boundaries when product correlation is introduced. |
| schema plus invariant validation | Feedback/eval smoke checks required report schema keys and minimum case coverage. Storage validation and delete-all preservation tests cover feedback/eval table invariants and private-data cleanup behavior. | Phase 3 must broaden lifecycle invariants for supersede, reschedule, cancel, dry-run, commit, and replay paths. |
| replayable trace coverage | Diagnostics smoke emits local trace JSONL, eval summaries, and downstream sanitized payload captures from Morrow-owned fixtures. The Phase 1 production trace sink smoke emits a sanitized provider-path `trace.jsonl` from the native fake Codex production fixture. | Phase 3 must extend replay coverage across lifecycle operations. |
| trajectory-level eval coverage | Current evals cover fixture-level detection, route/reason, privacy, and unsafe-action metrics. Benchmark families are references for later fixture design, not required hosted dependencies. | Phase 5 must harden trajectory-level eval coverage for Messages-to-Calendar approval paths, collateral-damage checks, and multi-step replay scoring. |

## Current Phase 0 Evidence

- `scripts/run-feedback-eval-smoke.sh` proves the local feedback/eval DB smoke with labels, snapshots, metrics, report schema checks, and delete-all preservation.
- `scripts/run-diagnostics-trace-eval-smoke.sh --assert-canary-rejection` proves local diagnostics trace/eval generation and privacy canary rejection without live vendor backends.
- `scripts/run-production-trace-sink-smoke.sh --assert-canary-rejection` proves Phase 1 production provider trace writing through the native fake Codex fixture, sanitized JSONL output, privacy inspection, canary rejection, and diagnostics cleanup receipt without live vendor backends.
- `docs/diagnostics-trace-eval.md` remains the detailed runbook for local layout, smoke commands, privacy inspection, reserved operations, and non-goals.
- `.omo/evidence/task-1-stabilize-main-feedback-eval-baseline.md` records the current worktree boundary and preserves beta release-distribution files outside this Phase 0 scope.

## Still Missing

- Phase 2: product correlation from durable traces back to candidate and decision surfaces.
- Phase 3: lifecycle coverage for supersede, reschedule, cancel, dry-run, commit, and replay paths.
- Phase 4: human approval/correction loop with risk-based human approval evidence.
- Phase 5: trajectory-level eval hardening for multi-step Messages-to-Calendar outcomes.
