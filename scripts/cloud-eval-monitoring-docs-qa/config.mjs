export const docs = [
  "README.md",
  "docs/diagnostics-trace-eval.md",
  "docs/axi-phase-0-feedback-eval-baseline.md",
  "docs/architecture/sans-io-boundaries.md"
]

export const phase6Root = ".omo/evidence/phase-6-cloud-eval-monitoring"
export function phase6SmokeCommandForOutDir(outDir) {
  return `scripts/run-cloud-eval-monitoring-smoke.sh --out-dir ${outDir} --assert-canary-rejection`
}

export const phase6SmokeCommand = phase6SmokeCommandForOutDir(`${phase6Root}/final-smoke`)

export const requiredSmokeArtifacts = [
  "summary.txt",
  "preflight-report.json",
  "eval-job-input.json",
  "eval-slices.json",
  "metrics.json",
  "cloud-eval-monitoring-report.json",
  "cloud-eval-monitoring-dashboard.md",
  "dashboard-summary.txt",
  "release-gate.json",
  "release-gate.md",
  "alerts.json",
  "privacy-inspect.txt",
  "canary-rejection.txt",
  "negative-matrix.json",
  "cleanup-receipt.txt"
]

export const requiredDocsPhrases = [
  phase6SmokeCommand,
  `${phase6Root}/final-smoke`,
  `${phase6Root}/final-smoke/summary.txt`,
  `${phase6Root}/final-smoke/preflight-report.json`,
  `${phase6Root}/final-smoke/eval-job-input.json`,
  `${phase6Root}/final-smoke/eval-slices.json`,
  `${phase6Root}/final-smoke/metrics.json`,
  `${phase6Root}/final-smoke/cloud-eval-monitoring-report.json`,
  `${phase6Root}/final-smoke/cloud-eval-monitoring-dashboard.md`,
  `${phase6Root}/final-smoke/dashboard-summary.txt`,
  `${phase6Root}/final-smoke/release-gate.json`,
  `${phase6Root}/final-smoke/release-gate.md`,
  `${phase6Root}/final-smoke/alerts.json`,
  `${phase6Root}/final-smoke/privacy-inspect.txt`,
  `${phase6Root}/final-smoke/canary-rejection.txt`,
  `${phase6Root}/final-smoke/negative-matrix.json`,
  `${phase6Root}/final-smoke/cleanup-receipt.txt`,
  `${phase6Root}/final-smoke/command-logs/`,
  "Phase 6 cloud eval and monitoring is covered by smoke artifacts",
  "cloud eval and monitoring is separate from deployed rollout",
  "correction UI remains future work",
  "deployed rollout remains future work",
  "automatic release promotion remains future work",
  "full live Messages-to-Calendar approval trajectory eval remains future work",
  "no live backend, provider network, EventKit, Messages, Calendar, Reminders, Phoenix, Langfuse, vendor backend, deployment, or cloud upload surface"
]

export const staleFutureWorkPatterns = [
  {
    name: "Phase 6 entirely future-work claim",
    pattern: /\bPhase 6 cloud eval(?: and)? monitoring remains future work\b/i
  },
  {
    name: "cloud eval monitoring must still be added",
    pattern: /\bcloud eval(?: and)? monitoring (?:must still be added|has not been added|is not covered)\b/i
  }
]

export const overclaimPatterns = [
  {
    name: "correction UI completion claim",
    pattern:
      /\b(?:correction UI|human correction UI)\b(?![^.\n]*(?:remains future work|not claimed|deferred|gap|not complete))[^.\n]*(?:complete|completed|done|implemented|shipped|covered|proved|available|enabled)/i
  },
  {
    name: "deployed rollout completion claim",
    pattern:
      /\b(?:deployed rollout|deployed product rollout|production rollout|cloud rollout)\b(?![^.\n]*(?:remains future work|not claimed|deferred|gap|not complete|separate from))[^.\n]*(?:complete|completed|done|implemented|shipped|covered|proved|available|enabled)/i
  },
  {
    name: "automatic release promotion claim",
    pattern:
      /\b(?:automatic release promotion|auto(?:matic)? promotion|release auto-promotion)\b(?![^.\n]*(?:remains future work|not claimed|not enabled|deferred|gap|not complete))[^.\n]*(?:complete|completed|done|implemented|shipped|covered|proved|available|enabled|promotes?)/i
  },
  {
    name: "full live approval trajectory completion claim",
    pattern:
      /\b(?:full live Messages-to-Calendar approval trajectory eval|full live Messages-to-Calendar approval trajectory|full live approval trajectory|full Messages\s*->\s*Calendar\s*->\s*approval trajectory eval|full Messages-to-Calendar approval trajectory eval)\b(?![^.\n]*(?:remains future work|not claimed|deferred|gap|not complete|unless the live receipt summary is PASS))[^.\n]*(?:complete|completed|done|implemented|shipped|covered|proved|available|enabled)/i
  },
  {
    name: "cloud eval monitoring completion claim without smoke receipt",
    pattern:
      /\b(?:Phase 6 cloud eval(?: and)? monitoring|cloud eval(?: and)? monitoring)\b(?![^.\n]*(?:covered by smoke artifacts|final-smoke|smoke receipt|smoke artifacts|local deterministic smoke))[^.\n]*(?:complete|completed|done|implemented|shipped|proved|available|enabled)/i
  }
]

export const forbiddenPrivacyPatterns = [
  {
    name: "raw privacy canary literal",
    pattern: /MORROW_PRIVACY_CANARY_RAW/
  },
  {
    name: "raw privacy text canary literal",
    pattern: new RegExp(["MORROW_PRIVACY", "CANARY", "RAW_TEXT"].join("_"))
  },
  {
    name: "absolute private path",
    pattern: /(?:\/Users\/|\/private\/)/
  }
]
