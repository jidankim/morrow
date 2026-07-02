#!/usr/bin/env node
import fs from "node:fs"

const forbiddenLiterals = [
  "MORROW_PRIVACY_CANARY_RAW_TEXT",
  "Maybe meet tomorrow?",
  "Provider meeting",
  "Low confidence provider title",
  "iMessage;-;+15555550103",
  "\"rawText\"",
  "\"raw_text\"",
  "\"prompt\"",
  "\"response\"",
  "\"rawJson\"",
  "\"raw_json\"",
  "\"providerJson\"",
  "\"provider_json\"",
  "\"embedding\"",
  "\"fullMessage\"",
  "\"full_message\"",
  "\"rawTitle\"",
  "\"raw_title\"",
  "\"titleText\"",
  "\"title_text\"",
  "\"unredactedTitle\"",
  "\"unredacted_title\"",
  "\"appDataDir\"",
  "\"diagnosticsPath\""
]

const forbiddenPathPatterns = [
  /\/Users\//,
  /\/private\//,
  /\/var\/folders\//
]

function usage() {
  console.error(
    "usage: trace-candidate-correlation-report.mjs <build|assert-clean> <args...>"
  )
}

function readJson(path) {
  return JSON.parse(fs.readFileSync(path, "utf8"))
}

function fail(message) {
  console.error(message)
  process.exit(1)
}

function rejectForbiddenContent(path) {
  const content = fs.readFileSync(path, "utf8")
  for (const literal of forbiddenLiterals) {
    if (content.includes(literal)) {
      fail(`forbidden literal leaked into ${path}`)
    }
  }
  for (const pattern of forbiddenPathPatterns) {
    if (pattern.test(content)) {
      fail(`forbidden local path leaked into ${path}`)
    }
  }
}

function firstItem(report, name) {
  const reportValue = report.nativeReports[name]
  if (!reportValue || !Array.isArray(reportValue.items) || reportValue.items.length < 1) {
    fail(`missing native report item: ${name}`)
  }
  return reportValue.items[0]
}

function assertCoverage(report) {
  const provider = firstItem(report, "providerCandidateRetained")
  const quiet = firstItem(report, "quietProviderRejection")
  const deleted = firstItem(report, "deletedDiagnosticsRoot")
  if (provider.subjectType !== "candidate" || provider.route !== "provider_candidate") {
    fail("provider candidate coverage missing")
  }
  if (provider.traceRetention !== "retained" || provider.traceSequence.length < 1) {
    fail("retained trace coverage missing")
  }
  if (quiet.subjectType !== "quietLog" || quiet.route !== "provider_rejection") {
    fail("quiet/provider rejection coverage missing")
  }
  if (quiet.traceRetention !== "retained") {
    fail("quiet retained trace coverage missing")
  }
  if (deleted.traceRetention !== "diagnosticsMissing") {
    fail("not-retained/deleted trace coverage missing")
  }
  if (report.boundary.noLiveBackend !== true || report.boundary.localFakeFixturesOnly !== true) {
    fail("fixture/no-live boundary missing")
  }
}

function buildReport(args) {
  const [providerPath, quietPath, deletedPath, sinkPath, outPath] = args
  if (!outPath) {
    usage()
    process.exit(64)
  }
  const report = {
    scenario: "phase-2-trace-candidate-correlation",
    generatedAtUtc: new Date().toISOString(),
    boundary: {
      localFakeFixturesOnly: true,
      noLiveBackend: true,
      networkRequired: false,
      realUserDataRequired: false
    },
    coverage: {
      providerCandidate: "native fake Codex provider candidate fixture",
      quietProviderRejection: "native fake Codex low-confidence provider rejection fixture",
      retainedTrace: "provider candidate and quiet reports include retained trace sequences",
      notRetainedDeletedTrace: "deleted diagnostics root reports diagnosticsMissing",
      localLabels: ["detection_route=provider_candidate", "proposal_outcome=unknown"],
      phase4Gap:
        "Current local labels approximate approve/reject/edit with detection and proposal outcome labels; Phase 4 still owns human approval/correction lifecycle labels."
    },
    nativeReports: {
      providerCandidateRetained: readJson(providerPath),
      quietProviderRejection: readJson(quietPath),
      deletedDiagnosticsRoot: readJson(deletedPath),
      sinkConflict: readJson(sinkPath)
    }
  }
  assertCoverage(report)
  fs.writeFileSync(outPath, `${JSON.stringify(report, null, 2)}\n`)
  rejectForbiddenContent(outPath)
}

function assertClean(args) {
  if (args.length < 1) {
    usage()
    process.exit(64)
  }
  for (const path of args) {
    rejectForbiddenContent(path)
  }
}

const [command, ...args] = process.argv.slice(2)
switch (command) {
  case "build":
    buildReport(args)
    break
  case "assert-clean":
    assertClean(args)
    break
  default:
    usage()
    process.exit(64)
}
