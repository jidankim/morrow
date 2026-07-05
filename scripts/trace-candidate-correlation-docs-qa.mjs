#!/usr/bin/env node

import { readFile, stat } from "node:fs/promises"

const diagnosticsDocPath = "docs/diagnostics-trace-eval.md"
const axiDocPath = "docs/axi-phase-0-feedback-eval-baseline.md"
const finalSmokeSummaryPath =
  ".omo/evidence/phase-2-trace-candidate-correlation/final-smoke/summary.txt"
const finalSmokeDir = ".omo/evidence/phase-2-trace-candidate-correlation/final-smoke"
const phase2SmokeCommand =
  "scripts/run-trace-candidate-correlation-smoke.sh --out-dir .omo/evidence/phase-2-trace-candidate-correlation/final-smoke --assert-canary-rejection"

const requiredArtifacts = [
  "summary.txt",
  "decision-evidence.json",
  "privacy-inspect.txt",
  "canary-rejection.txt",
  "cargo-decision-evidence.txt",
  "npm-decision-evidence.txt",
  "cleanup-receipt.txt"
]

const requiredCombinedPhrases = [
  phase2SmokeCommand,
  finalSmokeSummaryPath,
  "Phase 2 product correlation is implemented for local candidate and quiet summaries",
  "retained and deleted diagnostics states",
  "Raw private content is not exported by the local diagnostics evidence flow.",
  "No raw prompt text, raw source content, provider JSON, native identifiers, or app-data paths are exposed",
  "Phase 3 lifecycle coverage is now backed by local smoke artifacts and real-surface PASS/BLOCKED receipts.",
  "Phase 4 local approval/correction evidence is documented below and is outside the Phase 2 product-correlation claim.",
  "Phase 5 trajectory-level eval hardening remains future work",
  "Cloud telemetry rollout remains future work",
  "not full AXI compliance"
]

const forbiddenPatterns = [
  {
    name: "stale Phase 2 non-goal",
    pattern: /No Phase 2 product correlation from durable traces back to candidate and decision surfaces/i
  },
  {
    name: "stale Phase 2 still-needed wording",
    pattern: /Phase 2 product correlation is still needed|Phase 2 must preserve these boundaries when product correlation is introduced/i
  },
  {
    name: "full AXI overclaim",
    pattern: /\b(?:is|now|achieves|meets|claims)\s+full AXI compliance\b/i
  },
  {
    name: "Phase 3 overclaim",
    pattern: /\bPhase 3\b[^.\n]*(?:implemented|complete|done|shipped)/i
  },
  {
    name: "Phase 4 overclaim",
    pattern: /\bPhase 4\b[^.\n]*(?:implemented|complete|done|shipped)/i
  },
  {
    name: "Phase 5 overclaim",
    pattern: /\bPhase 5\b[^.\n]*(?:implemented|complete|done|shipped)/i
  },
  {
    name: "trajectory eval complete overclaim",
    pattern: /trajectory-level Messages -> Calendar -> approval eval is complete|trajectory eval (?:is )?(?:implemented|complete|done)/i
  },
  {
    name: "cloud telemetry overclaim",
    pattern: /cloud telemetry[^.\n]*(?:implemented|enabled|complete|rollout complete|shipping)/i
  },
  {
    name: "raw content upload claim",
    pattern:
      /raw (?:content|prompt|message|source)[^.\n]*(?:uploads|is uploaded|sent to cloud|sent to vendor)|(?:raw prompt|raw message|raw source)[^.\n]*leaves the device/i
  },
  {
    name: "copied raw smoke summary field",
    pattern:
      /provider_candidate_coverage:|quiet_provider_rejection_coverage:|retained_trace_coverage:|not_retained_deleted_trace_coverage:/i
  },
  {
    name: "raw privacy canary literal",
    pattern: new RegExp(["MORROW_PRIVACY", "CANARY", "RAW_TEXT"].join("_"))
  }
]

function missingPhrases(text, phrases) {
  return phrases.filter((phrase) => !text.includes(phrase))
}

function forbiddenMatches(text) {
  return forbiddenPatterns
    .filter(({ pattern }) => pattern.test(text))
    .map(({ name }) => `forbidden wording present: ${name}`)
}

function requiredArtifactFailures(text) {
  return requiredArtifacts
    .filter((artifact) => !text.includes(artifact))
    .map((artifact) => `missing Phase 2 expected artifact: ${artifact}`)
}

async function nonEmptyEvidenceFailures() {
  const failures = []
  for (const artifact of requiredArtifacts) {
    const path = `${finalSmokeDir}/${artifact}`
    try {
      const info = await stat(path)
      if (!info.isFile() || info.size === 0) {
        failures.push(`empty evidence artifact: ${path}`)
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      failures.push(message.includes("ENOENT") ? `missing evidence artifact: ${path}` : message)
    }
  }
  return failures
}

function assertFinalSmokePassed(summaryText) {
  const failures = []
  if (!summaryText.includes("scenario: Phase 2 trace candidate correlation smoke")) {
    failures.push("final-smoke summary does not identify the Phase 2 smoke scenario")
  }
  if (!summaryText.includes("result: PASS")) {
    failures.push("final-smoke summary does not report result: PASS")
  }
  return failures
}

function formatFailures(failures) {
  return failures.map((failure) => `- ${failure}`).join("\n")
}

function assertNoArgs(argv) {
  if (argv.length > 0) {
    throw new Error(`unknown argument: ${argv[0]}`)
  }
}

async function main() {
  assertNoArgs(process.argv.slice(2))

  const [diagnosticsText, axiText, summaryText] = await Promise.all([
    readFile(diagnosticsDocPath, "utf8"),
    readFile(axiDocPath, "utf8"),
    readFile(finalSmokeSummaryPath, "utf8")
  ])
  const combinedText = `${diagnosticsText}\n${axiText}`
  const failures = [
    ...(await nonEmptyEvidenceFailures()),
    ...assertFinalSmokePassed(summaryText),
    ...missingPhrases(combinedText, requiredCombinedPhrases).map(
      (phrase) => `missing required wording: ${phrase}`
    ),
    ...requiredArtifactFailures(combinedText),
    ...forbiddenMatches(combinedText)
  ]

  if (failures.length > 0) {
    console.error("FAIL trace candidate correlation docs QA")
    console.error(formatFailures(failures))
    process.exitCode = 1
    return
  }

  console.log("PASS trace candidate correlation docs QA")
  console.log(`docs: ${diagnosticsDocPath}, ${axiDocPath}`)
  console.log(`smoke: ${finalSmokeSummaryPath}`)
  console.log(`command: ${phase2SmokeCommand}`)
}

main().catch((error) => {
  const message = error instanceof Error ? error.message : "unknown docs QA failure"
  console.error(`FAIL trace candidate correlation docs QA: ${message}`)
  process.exitCode = 1
})
