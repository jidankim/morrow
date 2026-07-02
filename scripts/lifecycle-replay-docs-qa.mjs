#!/usr/bin/env node

import { readFile, stat } from "node:fs/promises"

const docs = [
  "docs/diagnostics-trace-eval.md",
  "docs/axi-phase-0-feedback-eval-baseline.md"
]

const requiredPhrases = [
  "scripts/run-lifecycle-replay-coverage-smoke.sh --out-dir .omo/evidence/phase-3-lifecycle-replay-coverage/final-smoke --assert-canary-rejection",
  "scripts/run-lifecycle-real-surface-qa.sh --out-dir .omo/evidence/phase-3-lifecycle-replay-coverage/real-surface",
  ".omo/evidence/phase-3-lifecycle-replay-coverage/final-smoke/summary.txt",
  ".omo/evidence/phase-3-lifecycle-replay-coverage/final-smoke/lifecycle-report.json",
  ".omo/evidence/phase-3-lifecycle-replay-coverage/final-smoke/trace.jsonl",
  ".omo/evidence/phase-3-lifecycle-replay-coverage/final-smoke/storage-readback.json",
  ".omo/evidence/phase-3-lifecycle-replay-coverage/final-smoke/privacy-inspect.txt",
  ".omo/evidence/phase-3-lifecycle-replay-coverage/final-smoke/canary-rejection.txt",
  ".omo/evidence/phase-3-lifecycle-replay-coverage/final-smoke/cleanup-receipt.txt",
  ".omo/evidence/phase-3-lifecycle-replay-coverage/real-surface/calendar-status.txt",
  ".omo/evidence/phase-3-lifecycle-replay-coverage/real-surface/reminders-status.txt",
  ".omo/evidence/phase-3-lifecycle-replay-coverage/real-surface/cleanup-receipt.txt",
  "PASS or BLOCKED per surface exits 0; any FAIL exits nonzero",
  "Phase 4 human approval/correction loop remains future work",
  "Phase 5 trajectory eval hardening remains future work",
  "full Messages -> Calendar -> approval trajectory eval remains future work",
  "No full real-surface pass is claimed because at least one surface is BLOCKED",
  "No raw prompt text, raw message bodies, provider JSON, native identifiers, app-data paths, or unredacted candidate titles are exposed"
]

const evidenceFiles = [
  ".omo/evidence/phase-3-lifecycle-replay-coverage/final-smoke/summary.txt",
  ".omo/evidence/phase-3-lifecycle-replay-coverage/final-smoke/lifecycle-report.json",
  ".omo/evidence/phase-3-lifecycle-replay-coverage/final-smoke/trace.jsonl",
  ".omo/evidence/phase-3-lifecycle-replay-coverage/final-smoke/storage-readback.json",
  ".omo/evidence/phase-3-lifecycle-replay-coverage/final-smoke/privacy-inspect.txt",
  ".omo/evidence/phase-3-lifecycle-replay-coverage/final-smoke/canary-rejection.txt",
  ".omo/evidence/phase-3-lifecycle-replay-coverage/final-smoke/cleanup-receipt.txt",
  ".omo/evidence/phase-3-lifecycle-replay-coverage/real-surface/summary.txt",
  ".omo/evidence/phase-3-lifecycle-replay-coverage/real-surface/calendar-status.txt",
  ".omo/evidence/phase-3-lifecycle-replay-coverage/real-surface/reminders-status.txt",
  ".omo/evidence/phase-3-lifecycle-replay-coverage/real-surface/cleanup-receipt.txt"
]

const forbiddenPatterns = [
  {
    name: "raw private-content export claim",
    pattern: /\braw content leaves (?:the )?device\b/i
  },
  {
    name: "full Calendar write workflow claim",
    pattern: /\bfull Calendar write workflow\b(?!\s+is\s+(?:not\s+claimed|deferred|future work))/i
  },
  {
    name: "approval workflow completion claim",
    pattern: /\b(?:approval workflow|human approval\/correction loop)\b[^.\n]*(?:complete|completed|done|implemented|proved|covered)/i
  },
  {
    name: "trajectory eval completion claim",
    pattern: /\btrajectory(?:-level)? eval\b[^.\n]*(?:complete|completed|done|implemented|proved|covered)/i
  },
  {
    name: "cloud telemetry completion claim",
    pattern: /\bcloud telemetry\b[^.\n]*(?:complete|completed|done|implemented|proved|covered)/i
  },
  {
    name: "Messages to Calendar approval trajectory completion claim",
    pattern:
      /\b(?:full\s+)?Messages\s*->\s*Calendar\s*->\s*approval\b[^.\n]*(?:complete|completed|done|implemented|proved|covered)/i
  }
]

function missingPhrases(text) {
  return requiredPhrases.filter((phrase) => !text.includes(phrase))
}

function presentForbidden(text) {
  return forbiddenPatterns.filter(({ pattern }) => pattern.test(text)).map(({ name }) => name)
}

async function nonEmptyEvidenceFailures() {
  const failures = []
  for (const file of evidenceFiles) {
    try {
      const info = await stat(file)
      if (!info.isFile() || info.size === 0) {
        failures.push(`empty evidence artifact: ${file}`)
      }
    } catch {
      failures.push(`missing evidence artifact: ${file}`)
    }
  }
  return failures
}

function printList(title, items) {
  console.log(title)
  if (items.length === 0) {
    console.log("- none")
    return
  }
  for (const item of items) {
    console.log(`- ${item}`)
  }
}

async function main() {
  const docTexts = await Promise.all(docs.map((doc) => readFile(doc, "utf8")))
  const text = docTexts.join("\n")
  const failures = [
    ...missingPhrases(text).map((phrase) => `missing required wording: ${phrase}`),
    ...presentForbidden(text).map((name) => `forbidden overclaim present: ${name}`),
    ...(await nonEmptyEvidenceFailures())
  ]

  printList("Lifecycle replay docs QA failures:", failures)

  if (failures.length > 0) {
    process.exitCode = 1
    return
  }

  console.log("PASS lifecycle replay docs QA")
}

await main()
