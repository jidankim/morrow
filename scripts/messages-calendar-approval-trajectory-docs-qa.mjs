#!/usr/bin/env node

import { readFile, stat } from "node:fs/promises"
import { join } from "node:path"

const docs = [
  "README.md",
  "docs/diagnostics-trace-eval.md",
  "docs/axi-phase-0-feedback-eval-baseline.md",
  "docs/architecture/sans-io-boundaries.md"
]

const phaseRoot = ".omo/evidence/phase-5-messages-calendar-approval-trajectory-eval"
const phase5SmokeCommand =
  "scripts/run-messages-calendar-approval-trajectory-eval-smoke.sh --out-dir .omo/evidence/phase-5-messages-calendar-approval-trajectory-eval/final-smoke --assert-canary-rejection"
const phase5LiveCommand =
  "scripts/run-messages-calendar-approval-live-receipt.sh --out-dir .omo/evidence/phase-5-messages-calendar-approval-trajectory-eval/live-receipt"

const requiredSmokeArtifacts = [
  "summary.txt",
  "trajectory-report.json",
  "trace.jsonl",
  "storage-readback.json",
  "decision-evidence.json",
  "privacy-inspect.txt",
  "canary-rejection.txt",
  "cleanup-receipt.txt",
  "command-log-pass-counts.txt"
]

const requiredCaseFamilies = [
  "scheduled_meeting_accepted",
  "scheduled_meeting_rejected",
  "scheduled_meeting_edited_before_approval",
  "task_reminder_accepted",
  "task_reminder_rejected",
  "provider_quiet_low_confidence",
  "collateral_damage_non_target_preserved",
  "replay_idempotent_retry",
  "privacy_canary_rejection"
]

const requiredDocsPhrases = [
  phase5SmokeCommand,
  phase5LiveCommand,
  `${phaseRoot}/final-smoke`,
  `${phaseRoot}/live-receipt`,
  `${phaseRoot}/final-smoke/summary.txt`,
  `${phaseRoot}/final-smoke/trajectory-report.json`,
  `${phaseRoot}/final-smoke/trace.jsonl`,
  `${phaseRoot}/final-smoke/storage-readback.json`,
  `${phaseRoot}/final-smoke/decision-evidence.json`,
  `${phaseRoot}/final-smoke/privacy-inspect.txt`,
  `${phaseRoot}/final-smoke/canary-rejection.txt`,
  `${phaseRoot}/final-smoke/cleanup-receipt.txt`,
  "local Phase 5 trajectory eval hardening",
  "full live Messages-to-Calendar approval trajectory eval remains future work",
  "correction UI remains future work",
  "cloud telemetry remains future work",
  "deployed rollout remains future work",
  "no live backend, provider network, EventKit, Messages, Calendar, Reminders, Phoenix, Langfuse, or vendor backend"
]

const baseOverclaimPatterns = [
  ["correction UI completion claim", /\b(?:correction UI|human correction UI)\b(?![^.\n]*(?:remains future work|not claimed|deferred|gap|not complete))[^.\n]*(?:complete|completed|done|implemented|shipped|covered|proved|available|enabled)/i],
  ["cloud telemetry completion claim", /\bcloud telemetry\b(?![^.\n]*(?:remains future work|not claimed|deferred|gap|not complete|no cloud telemetry))[^.\n]*(?:complete|completed|done|implemented|shipped|covered|proved|available|enabled|rollout complete)/i],
  ["deployed rollout completion claim", /\b(?:deployed rollout|deployed product rollout|production rollout|cloud rollout)\b(?![^.\n]*(?:remains future work|not claimed|deferred|gap|not complete))[^.\n]*(?:complete|completed|done|implemented|shipped|covered|proved|available|enabled)/i],
  ["approval queue completion claim", /\bapproval queue\b(?![^.\n]*(?:remains future work|not claimed|deferred|gap|not complete))[^.\n]*(?:complete|completed|done|implemented|shipped|covered|proved|available|enabled)/i],
  ["auto-approval completion claim", /\bauto-approval\b(?![^.\n]*(?:not claimed|not introduced|not enabled|deferred|gap|future work))[^.\n]*(?:complete|completed|done|implemented|shipped|covered|proved|available|enabled)/i],
  ["raw privacy canary literal", new RegExp(["MORROW_PRIVACY", "CANARY", "RAW_TEXT"].join("_"))]
]

const liveOverclaimPatterns = [
  ["full live approval trajectory completion claim", /\b(?:full live Messages-to-Calendar approval trajectory eval|full live Messages-to-Calendar approval trajectory|full live approval trajectory|full Messages\s*->\s*Calendar\s*->\s*approval trajectory eval|full Messages-to-Calendar approval trajectory eval)\b(?![^.\n]*(?:remains future work|not claimed|deferred|gap|not complete|not allowed))[^.\n]*(?:complete|completed|done|implemented|shipped|covered|proved|available|enabled)/i]
]

const allowedArgs = new Set(["--smoke-dir", "--live-dir", "--fixture-overclaim"])

function parseArgs(argv) {
  const args = new Map()
  for (let index = 0; index < argv.length; index += 1) {
    const key = argv[index]
    if (!allowedArgs.has(key)) throw new Error(`unknown argument: ${key}`)
    const value = argv[index + 1]
    if (!value || value.startsWith("--")) throw new Error(`missing value for ${key}`)
    args.set(key, value)
    index += 1
  }
  const smokeDir = args.get("--smoke-dir")
  const liveDir = args.get("--live-dir")
  if (!smokeDir || !liveDir) throw new Error("required arguments missing: --smoke-dir and --live-dir")
  return { smokeDir, liveDir, fixtureOverclaimPath: args.get("--fixture-overclaim") ?? null }
}

async function assertNonEmptyFile(path) {
  const info = await stat(path)
  if (!info.isFile() || info.size === 0) throw new Error(`empty evidence artifact: ${path}`)
}

async function evidenceFailures(smokeDir, liveDir) {
  const paths = [
    ...requiredSmokeArtifacts.map((artifact) => join(smokeDir, artifact)),
    join(liveDir, "summary.txt"),
    join(liveDir, "cleanup-receipt.txt"),
    join(liveDir, "privacy-inspect.txt")
  ]
  const failures = []
  for (const path of paths) {
    try {
      await assertNonEmptyFile(path)
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      failures.push(message.includes("ENOENT") ? `missing evidence artifact: ${path}` : message)
    }
  }
  return failures
}

function parseKeyValues(text) {
  return Object.fromEntries(
    text
      .split("\n")
      .filter((line) => line.includes("="))
      .map((line) => {
        const [key, ...rest] = line.split("=")
        return [key, rest.join("=")]
      })
  )
}

function smokeFailures(summaryText, report) {
  const requiredSummary = [
    "scenario: Phase 5 messages calendar approval trajectory eval smoke",
    `invocation: ${phase5SmokeCommand}`,
    "result: PASS"
  ]
  const failures = requiredSummary
    .filter((phrase) => !summaryText.includes(phrase))
    .map((phrase) => `smoke summary missing required observable: ${phrase}`)
  if (report.schema !== "phase5_messages_calendar_approval_trajectory_eval_report_v1") {
    failures.push("trajectory report schema mismatch")
  }
  if (report.invocation !== phase5SmokeCommand) failures.push("trajectory report invocation mismatch")
  if (report.current_run !== true) failures.push("trajectory report is not marked current_run=true")
  for (const family of requiredCaseFamilies) {
    if (report.coverage?.required_case_families?.[family]?.covered !== true) {
      failures.push(`trajectory report missing required case family: ${family}`)
    }
  }
  return failures
}

function liveReceiptFailures(liveSummary) {
  const fields = parseKeyValues(liveSummary)
  const failures = []
  if (fields.schema !== "phase5_messages_calendar_approval_live_receipt_v1") {
    failures.push("live receipt schema mismatch")
  }
  if (fields.overall_status === "FAIL") failures.push("live receipt reports FAIL")
  if (!["PASS", "BLOCKED"].includes(fields.overall_status)) failures.push("live receipt status is not PASS or BLOCKED")
  if (fields.overall_status === "PASS" && fields.full_live_claim_allowed !== "true") {
    failures.push("PASS live receipt does not allow full live claim")
  }
  if (fields.overall_status === "BLOCKED" && fields.full_live_claim_allowed !== "false") {
    failures.push("BLOCKED live receipt must disallow full live claim")
  }
  return { failures, fullLiveClaimAllowed: fields.overall_status === "PASS" && fields.full_live_claim_allowed === "true" }
}

function patternFailures(text, patterns, prefix) {
  return patterns.filter(([, pattern]) => pattern.test(text)).map(([name]) => `${prefix}: ${name}`)
}

async function main() {
  const { smokeDir, liveDir, fixtureOverclaimPath } = parseArgs(process.argv.slice(2))
  const [summaryText, reportText, liveSummary, fixtureText, ...docTexts] = await Promise.all([
    readFile(join(smokeDir, "summary.txt"), "utf8"),
    readFile(join(smokeDir, "trajectory-report.json"), "utf8"),
    readFile(join(liveDir, "summary.txt"), "utf8"),
    fixtureOverclaimPath ? readFile(fixtureOverclaimPath, "utf8") : "",
    ...docs.map((doc) => readFile(doc, "utf8"))
  ])
  const report = JSON.parse(reportText)
  const docsText = docTexts.join("\n")
  const liveResult = liveReceiptFailures(liveSummary)
  const overclaimPatterns = liveResult.fullLiveClaimAllowed
    ? baseOverclaimPatterns
    : [...baseOverclaimPatterns, ...liveOverclaimPatterns]
  const failures = [
    ...(await evidenceFailures(smokeDir, liveDir)),
    ...smokeFailures(summaryText, report),
    ...liveResult.failures,
    ...requiredDocsPhrases.filter((phrase) => !docsText.includes(phrase)).map((phrase) => `missing required docs wording: ${phrase}`),
    ...patternFailures(`${docsText}\n${fixtureText}`, overclaimPatterns, "forbidden overclaim present")
  ]

  if (failures.length > 0) {
    console.error("FAIL messages calendar approval trajectory docs QA")
    console.error(failures.map((failure) => `- ${failure}`).join("\n"))
    process.exitCode = 1
    return
  }

  console.log("PASS messages calendar approval trajectory docs QA")
  console.log(`docs: ${docs.join(", ")}`)
  console.log(`smoke: ${smokeDir}/summary.txt`)
  console.log(`live_receipt: ${liveDir}/summary.txt`)
  console.log(`command: ${phase5SmokeCommand}`)
  console.log(`live_command: ${phase5LiveCommand}`)
}

main().catch((error) => {
  const message = error instanceof Error ? error.message : "unknown docs QA failure"
  console.error(`FAIL messages calendar approval trajectory docs QA: ${message}`)
  process.exitCode = 1
})
