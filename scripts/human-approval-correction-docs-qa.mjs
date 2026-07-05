#!/usr/bin/env node

import { readFile, stat } from "node:fs/promises"
import { join } from "node:path"

const docs = [
  "README.md",
  "docs/diagnostics-trace-eval.md",
  "docs/axi-phase-0-feedback-eval-baseline.md",
  "docs/architecture/sans-io-boundaries.md"
]

const phase4SmokeCommand =
  "scripts/run-human-approval-correction-smoke.sh --out-dir .omo/evidence/phase-4-human-approval-correction/final-smoke --assert-canary-rejection"
const phase4RealSurfaceCommand =
  "scripts/run-lifecycle-real-surface-qa.sh --out-dir .omo/evidence/phase-4-human-approval-correction/real-surface"

const requiredArtifactNames = [
  "summary.txt",
  "human-approval-correction-report.json",
  "trace.jsonl",
  "storage-readback.json",
  "decision-evidence.json",
  "privacy-inspect.txt",
  "canary-rejection.txt",
  "cleanup-receipt.txt",
  "command-log-pass-counts.txt"
]

const requiredDocPhrases = [
  phase4SmokeCommand,
  phase4RealSurfaceCommand,
  ".omo/evidence/phase-4-human-approval-correction/final-smoke/summary.txt",
  ".omo/evidence/phase-4-human-approval-correction/final-smoke/human-approval-correction-report.json",
  ".omo/evidence/phase-4-human-approval-correction/final-smoke/trace.jsonl",
  ".omo/evidence/phase-4-human-approval-correction/final-smoke/storage-readback.json",
  ".omo/evidence/phase-4-human-approval-correction/final-smoke/decision-evidence.json",
  ".omo/evidence/phase-4-human-approval-correction/final-smoke/privacy-inspect.txt",
  ".omo/evidence/phase-4-human-approval-correction/final-smoke/canary-rejection.txt",
  ".omo/evidence/phase-4-human-approval-correction/final-smoke/cleanup-receipt.txt",
  ".omo/evidence/phase-4-human-approval-correction/real-surface/summary.txt",
  ".omo/evidence/phase-4-human-approval-correction/real-surface/cleanup-receipt.txt",
  "local Phase 4 approval/correction evidence loop",
  "covered by local smoke artifacts",
  "no live network, vendor, Messages, Calendar, or Reminders access",
  "correction UI remains future work",
  "cloud telemetry remains future work",
  "full live Messages-to-Calendar approval trajectory eval remains future work",
  "real-surface status remains PASS or sanitized BLOCKED",
  "not full AXI compliance"
]

const staleFutureWorkPatterns = [
  {
    name: "Phase 4 entirely future-work claim",
    pattern: /\bPhase 4 human approval\/correction(?: loop)? remains future work\b/i
  },
  {
    name: "Phase 4 must still add local loop",
    pattern: /\bPhase 4 must add (?:the )?human approval\/correction loop\b/i
  },
  {
    name: "approval/correction invariants deferred to Phase 4",
    pattern: /\bApproval\/correction invariants remain deferred to Phase 4\b/i
  },
  {
    name: "no Phase 4 local loop claim",
    pattern: /\bNo Phase 4 human approval\/correction loop\b/i
  }
]

const overclaimPatterns = [
  {
    name: "correction UI completion claim",
    pattern:
      /\b(?:correction UI|human correction UI)\b(?![^.\n]*(?:remains future work|not claimed|deferred|gap|not complete))[^.\n]*(?:complete|completed|done|implemented|shipped|covered|proved|available|enabled)/i
  },
  {
    name: "cloud telemetry completion claim",
    pattern:
      /\bcloud telemetry\b(?![^.\n]*(?:remains future work|not claimed|deferred|gap|not complete|no cloud telemetry))[^.\n]*(?:complete|completed|done|implemented|shipped|covered|proved|available|enabled|rollout complete)/i
  },
  {
    name: "full live trajectory eval completion claim",
    pattern:
      /\b(?:full live Messages-to-Calendar approval trajectory eval|full Messages\s*->\s*Calendar\s*->\s*approval trajectory eval|full Messages-to-Calendar approval trajectory eval)\b(?![^.\n]*(?:remains future work|not claimed|deferred|gap|not complete))[^.\n]*(?:complete|completed|done|implemented|shipped|covered|proved|available|enabled)/i
  }
]

const forbiddenPrivacyPatterns = [
  {
    name: "raw privacy canary literal",
    pattern: new RegExp(["MORROW_PRIVACY", "CANARY", "RAW_TEXT"].join("_"))
  }
]

const allowedArgs = new Set(["--smoke-dir", "--fixture-overclaim"])

function parseArgs(argv) {
  const args = new Map()
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index]
    if (!arg.startsWith("--")) {
      throw new Error(`unexpected positional argument: ${arg}`)
    }
    if (!allowedArgs.has(arg)) {
      throw new Error(`unknown argument: ${arg}`)
    }
    const value = argv[index + 1]
    if (!value || value.startsWith("--")) {
      throw new Error(`missing value for ${arg}`)
    }
    args.set(arg, value)
    index += 1
  }
  const smokeDir = args.get("--smoke-dir")
  if (!smokeDir) {
    throw new Error("required argument missing: --smoke-dir")
  }
  return {
    smokeDir,
    fixtureOverclaimPath: args.get("--fixture-overclaim") ?? null
  }
}

async function readText(path) {
  return readFile(path, "utf8")
}

async function assertNonEmptyFile(path) {
  const info = await stat(path)
  if (!info.isFile() || info.size === 0) {
    throw new Error(`empty evidence artifact: ${path}`)
  }
}

async function evidenceFailures(smokeDir) {
  const failures = []
  for (const artifactName of requiredArtifactNames) {
    const artifactPath = join(smokeDir, artifactName)
    try {
      await assertNonEmptyFile(artifactPath)
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      failures.push(message.includes("ENOENT") ? `missing evidence artifact: ${artifactPath}` : message)
    }
  }
  return failures
}

function missingPhrases(text) {
  return requiredDocPhrases.filter((phrase) => !text.includes(phrase))
}

function patternFailures(text, patterns, prefix) {
  return patterns
    .filter(({ pattern }) => pattern.test(text))
    .map(({ name }) => `${prefix}: ${name}`)
}

function parseReport(reportText) {
  try {
    return JSON.parse(reportText)
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error)
    throw new Error(`human-approval-correction-report.json is not valid JSON: ${message}`)
  }
}

function smokeSummaryFailures(summaryText) {
  const required = [
    "scenario: Phase 4 human approval correction smoke",
    `invocation: ${phase4SmokeCommand}`,
    "backend_observable: no live network, vendor, Messages, Calendar, or Reminders access was required",
    "coverage: user_correction=user_corrected",
    "coverage: proposal_outcome=accepted",
    "coverage: proposal_outcome=rejected_observed",
    "coverage: proposal_outcome=pending_edited",
    "coverage: proposal_outcome=unknown",
    "result: PASS"
  ]
  return required
    .filter((phrase) => !summaryText.includes(phrase))
    .map((phrase) => `smoke summary missing required observable: ${phrase}`)
}

function smokeReportFailures(report) {
  const failures = []
  const requiredBooleans = [
    ["current_run", report.current_run],
    [
      "no_live_network_vendor_messages_calendar_reminders",
      report.no_live_network_vendor_messages_calendar_reminders
    ],
    ["coverage.user_correction", report.coverage?.user_correction],
    ["coverage.outcomes.user_corrected", report.coverage?.outcomes?.user_corrected],
    ["coverage.outcomes.accepted", report.coverage?.outcomes?.accepted],
    ["coverage.outcomes.rejected_observed", report.coverage?.outcomes?.rejected_observed],
    ["coverage.outcomes.pending_edited", report.coverage?.outcomes?.pending_edited],
    ["coverage.outcomes.unknown", report.coverage?.outcomes?.unknown],
    ["coverage.cleanup.synthetic_privacy_surfaces_removed", report.coverage?.cleanup?.synthetic_privacy_surfaces_removed]
  ]
  for (const [name, value] of requiredBooleans) {
    if (value !== true) {
      failures.push(`smoke report missing true flag: ${name}`)
    }
  }

  for (const outcome of ["accepted", "rejected_observed", "pending_edited", "unknown"]) {
    const evidenceArtifact = report.coverage?.required_outcomes?.[outcome]?.evidence_artifact
    if (typeof evidenceArtifact !== "string" || evidenceArtifact.length === 0) {
      failures.push(`smoke report missing named local evidence artifact for outcome: ${outcome}`)
    }
  }

  if (report.schema !== "phase4_human_approval_correction_smoke_report_v1") {
    failures.push("smoke report schema mismatch")
  }
  if (report.invocation !== phase4SmokeCommand) {
    failures.push("smoke report invocation mismatch")
  }
  return failures
}

function formatFailures(failures) {
  return failures.map((failure) => `- ${failure}`).join("\n")
}

async function main() {
  const { smokeDir, fixtureOverclaimPath } = parseArgs(process.argv.slice(2))
  const [summaryText, reportText, ...docTexts] = await Promise.all([
    readText(join(smokeDir, "summary.txt")),
    readText(join(smokeDir, "human-approval-correction-report.json")),
    ...docs.map((doc) => readText(doc))
  ])
  const fixtureText = fixtureOverclaimPath ? await readText(fixtureOverclaimPath) : ""
  const docsText = docTexts.join("\n")
  const scanText = `${docsText}\n${fixtureText}`
  const report = parseReport(reportText)
  const failures = [
    ...(await evidenceFailures(smokeDir)),
    ...smokeSummaryFailures(summaryText),
    ...smokeReportFailures(report),
    ...missingPhrases(docsText).map((phrase) => `missing required docs wording: ${phrase}`),
    ...patternFailures(scanText, staleFutureWorkPatterns, "stale future-work wording present"),
    ...patternFailures(scanText, overclaimPatterns, "forbidden overclaim present"),
    ...patternFailures(scanText, forbiddenPrivacyPatterns, "forbidden privacy wording present")
  ]

  if (failures.length > 0) {
    console.error("FAIL human approval correction docs QA")
    console.error(formatFailures(failures))
    process.exitCode = 1
    return
  }

  console.log("PASS human approval correction docs QA")
  console.log(`docs: ${docs.join(", ")}`)
  console.log(`smoke: ${smokeDir}/summary.txt`)
  console.log(`report: ${smokeDir}/human-approval-correction-report.json`)
  console.log(`command: ${phase4SmokeCommand}`)
}

main().catch((error) => {
  const message = error instanceof Error ? error.message : "unknown docs QA failure"
  console.error(`FAIL human approval correction docs QA: ${message}`)
  process.exitCode = 1
})
