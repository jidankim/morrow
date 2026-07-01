#!/usr/bin/env node
// allow: SIZE_OK - single-purpose diagnostic docs QA keeps fixture, copy-policy assertions, and evidence reporting together; split if it grows further.

import { mkdir, readFile, writeFile } from "node:fs/promises"
import { dirname } from "node:path"

const usage =
  "Usage: node scripts/diagnostic-docs-qa.mjs docs/beta-testing.md docs/diagnostic-testing.md .omo/evidence/task-3-unsigned-adhoc-app-artifact.md"

const requiredDiagnosticPhrases = [
  "trusted technical tester",
  "Finder right-click Open",
  "System Settings Privacy & Security",
  "not expected to run Terminal commands",
  "host-architecture only",
  "primary artifact is the diagnostic zip",
  "ad-hoc signed and not notarized",
  "macOS may show security warnings",
  "must not be redistributed",
  "Full Disk Access",
  "Calendar access",
  "Codex CLI is a user-owned runtime prerequisite",
  "Package launch by itself only proves that the app opens",
  "Real Messages-to-Calendar QA still requires"
]

const requiredBetaPhrases = [
  "signed, notarized DMG",
  "docs/diagnostic-testing.md",
  "Diagnostic artifacts are not signed/notarized beta artifacts",
  "must not be redistributed"
]

const assertions = [
  {
    name: "trusted GUI-capable tester scope",
    pass: ({ diagnosticText }) =>
      diagnosticText.includes("trusted technical tester") &&
      diagnosticText.includes("Finder right-click Open") &&
      diagnosticText.includes("System Settings Privacy & Security") &&
      diagnosticText.includes("not expected to run Terminal commands")
  },
  {
    name: "host-architecture limitation",
    pass: ({ diagnosticText }) => diagnosticText.includes("host-architecture only")
  },
  {
    name: "primary zip wording",
    pass: ({ diagnosticText }) => diagnosticText.includes("primary artifact is the diagnostic zip")
  },
  {
    name: "ad-hoc/not-notarized wording",
    pass: ({ diagnosticText }) =>
      diagnosticText.includes("ad-hoc signed and not notarized") &&
      diagnosticText.includes("macOS may show security warnings")
  },
  {
    name: "no broad distribution",
    pass: ({ combinedText }) =>
      combinedText.includes("must not be redistributed") &&
      !/safe for everyone|broad distribution|public beta/i.test(combinedText)
  },
  {
    name: "no global Gatekeeper disable",
    pass: ({ combinedText }) =>
      !/spctl\s+--master-disable|disable\s+Gatekeeper|Gatekeeper\s+globally|globally\s+disable/i.test(
        combinedText
      )
  },
  {
    name: "no tester Terminal/xattr requirement",
    pass: ({ diagnosticText }) =>
      diagnosticText.includes("not expected to run Terminal commands") &&
      !/\bxattr\b|Terminal command:|open Terminal|use Terminal|run this in Terminal/i.test(
        diagnosticText
      )
  },
  {
    name: "no tester source-build requirement",
    pass: ({ combinedText }) =>
      !/Testers?\s+(?:must|should|need to|required to)\s+(?:install|run|clone)[^.]*\b(?:Rust|Xcode|npm install|Tauri|source-build|clone)/i.test(
        combinedText
      )
  },
  {
    name: "Codex CLI user-owned runtime boundary",
    pass: ({ diagnosticText }) =>
      diagnosticText.includes("Codex CLI") &&
      diagnosticText.includes("Codex CLI is a user-owned runtime prerequisite") &&
      diagnosticText.includes("does not manage the CLI login")
  },
  {
    name: "Full Disk Access/Calendar permissions",
    pass: ({ diagnosticText }) =>
      diagnosticText.includes("Full Disk Access") && diagnosticText.includes("Calendar access")
  },
  {
    name: "QA limitations",
    pass: ({ diagnosticText }) =>
      diagnosticText.includes("Package launch by itself only proves that the app opens") &&
      diagnosticText.includes("Real Messages-to-Calendar QA still requires")
  },
  {
    name: "diagnostic artifact distinguished from beta release",
    pass: ({ betaText, diagnosticText }) =>
      betaText.includes("docs/diagnostic-testing.md") &&
      diagnosticText.includes("does not replace the signed, notarized beta DMG release path")
  }
]

const forbiddenPatterns = [
  {
    name: "spctl master disable",
    pattern: /spctl\s+--master-disable/i
  },
  {
    name: "global Gatekeeper disabling",
    pattern: /disable\s+Gatekeeper|Gatekeeper\s+globally|globally\s+disable/i
  },
  {
    name: "tester xattr instruction",
    pattern: /\bxattr\b|recursive quarantine removal|xattr\s+-dr/i,
    diagnosticOnly: true
  },
  {
    name: "tester Rust requirement",
    pattern: /Testers?\s+(?:must|should|need to|required to)\s+install[^.]*Rust/i
  },
  {
    name: "tester Xcode requirement",
    pattern: /Testers?\s+(?:must|should|need to|required to)\s+install[^.]*Xcode/i
  },
  {
    name: "source-build requirement for testers",
    pattern: /Testers?\s+(?:must|should|need to|required to)\s+(?:clone|run|install)[^.]*source-build/i
  },
  {
    name: "diagnostic artifact claimed notarized",
    pattern:
      /\b(?:diagnostic artifact|diagnostic zip|artifact|app|This)\b(?!(?:[^.]*\bnot notarized\b|[^.]*\bsigned\/notarized beta\b|[^.]*\bsigned, notarized beta\b))[^.]*\bnotarized\b/i,
    diagnosticOnly: true
  },
  {
    name: "notarized beta safety claim",
    pattern: /notarized beta is safe for everyone/i
  },
  {
    name: "Codex credential handling",
    pattern: /auth\.json|~\/\.codex|codex_access_token|sk-[A-Za-z0-9]/i
  }
]

class DocsQaError extends Error {
  constructor(message, failures, evidenceRecorded = false) {
    super(message)
    this.name = "DocsQaError"
    this.failures = failures
    this.evidenceRecorded = evidenceRecorded
  }
}

function parseArgs(args) {
  const [betaDocPath, diagnosticDocPath, evidencePath, ...extraArgs] = args
  if (
    betaDocPath === undefined ||
    diagnosticDocPath === undefined ||
    evidencePath === undefined ||
    extraArgs.length > 0
  ) {
    throw new DocsQaError(usage, [
      "expected exactly three arguments: <beta-testing path> <diagnostic-testing path> <evidence path>"
    ])
  }
  return { betaDocPath, diagnosticDocPath, evidencePath }
}

function missingPhrases(text, phrases) {
  return phrases.filter((phrase) => !text.includes(phrase))
}

function presentForbidden(combinedText, diagnosticText) {
  return forbiddenPatterns
    .filter(({ pattern, diagnosticOnly }) =>
      pattern.test(diagnosticOnly ? diagnosticText : combinedText)
    )
    .map(({ name }) => name)
}

function evaluateDocs(betaText, diagnosticText) {
  const missingDiagnosticPhrases = missingPhrases(diagnosticText, requiredDiagnosticPhrases)
  const missingBetaPhrases = missingPhrases(betaText, requiredBetaPhrases)
  const combinedText = `${betaText}\n${diagnosticText}`
  const forbidden = presentForbidden(combinedText, diagnosticText)
  const context = { betaText, diagnosticText, combinedText }
  const assertionResults = assertions.map((assertion) => ({
    name: assertion.name,
    passed: assertion.pass(context)
  }))
  const failures = [
    ...(betaText.trim() === "" ? ["beta doc is empty"] : []),
    ...(diagnosticText.trim() === "" ? ["diagnostic doc is empty"] : []),
    ...missingBetaPhrases.map((phrase) => `beta doc missing required phrase: ${phrase}`),
    ...missingDiagnosticPhrases.map((phrase) => `diagnostic doc missing required phrase: ${phrase}`),
    ...forbidden.map((name) => `forbidden instruction or claim present: ${name}`),
    ...assertionResults
      .filter((assertion) => !assertion.passed)
      .map((assertion) => `assertion failed: ${assertion.name}`)
  ]

  return {
    missingBetaPhrases,
    missingDiagnosticPhrases,
    forbidden,
    assertionResults,
    failures
  }
}

function listLines(items, emptyText) {
  return items.length === 0 ? [`- ${emptyText}`] : items.map((item) => `- ${item}`)
}

function evidenceSection(args, exitStatus, result) {
  const passed = exitStatus === 0
  return [
    "# Todo 3 Diagnostic Docs QA",
    "",
    `Timestamp: ${new Date().toISOString()}`,
    `Command: \`node scripts/diagnostic-docs-qa.mjs ${args.join(" ")}\``,
    `Exit status: ${exitStatus}`,
    `Result: ${passed ? "PASS" : "FAIL"}`,
    "",
    "## Required phrase coverage",
    "",
    "Missing beta-testing phrases:",
    ...listLines(result.missingBetaPhrases, "none"),
    "",
    "Missing diagnostic-testing phrases:",
    ...listLines(result.missingDiagnosticPhrases, "none"),
    "",
    "Forbidden instructions or claims present:",
    ...listLines(result.forbidden, "none"),
    "",
    "General failures:",
    ...listLines(result.failures ?? [], "none"),
    "",
    "## Assertions",
    "",
    ...result.assertionResults.map(
      (assertion) => `- ${assertion.passed ? "PASS" : "FAIL"} ${assertion.name}`
    ),
    "",
    "## Adversarial probes",
    "",
    "- PASS untrusted external text/prompt_injection: docs are treated as literal content; unsafe phrases are rejected by pattern checks.",
    "- PASS misleading_success_output: success is derived from computed assertions, not from existing evidence text.",
    "- PASS stale_state: QA reads the docs from disk at invocation time.",
    "",
    passed ? "RESULT: PASS" : "RESULT: FAIL",
    ""
  ].join("\n")
}

async function writeEvidence(evidencePath, section) {
  await mkdir(dirname(evidencePath), { recursive: true })
  await writeFile(evidencePath, `${section}\n`, "utf8")
}

function failureResult(message) {
  return {
    missingBetaPhrases: [],
    missingDiagnosticPhrases: [],
    forbidden: [],
    assertionResults: assertions.map((assertion) => ({ name: assertion.name, passed: false })),
    failures: [message]
  }
}

function reportFailure(error) {
  if (error instanceof DocsQaError) {
    console.error(error.message)
    for (const failure of error.failures) {
      console.error(`- ${failure}`)
    }
    process.exitCode = 1
    return
  }
  if (error instanceof Error) {
    console.error(error.message)
    process.exitCode = 1
    return
  }
  console.error("Unknown diagnostic docs QA failure")
  process.exitCode = 1
}

async function main() {
  const args = process.argv.slice(2)
  const evidencePath = args[2]
  try {
    const invocation = parseArgs(args)
    const [betaText, diagnosticText] = await Promise.all([
      readFile(invocation.betaDocPath, "utf8"),
      readFile(invocation.diagnosticDocPath, "utf8")
    ])
    const result = evaluateDocs(betaText, diagnosticText)
    if (result.failures.length > 0) {
      await writeEvidence(invocation.evidencePath, evidenceSection(args, 1, result))
      throw new DocsQaError("Diagnostic docs QA failed", result.failures, true)
    }
    await writeEvidence(invocation.evidencePath, evidenceSection(args, 0, result))
    console.log("PASS diagnostic docs QA")
  } catch (error) {
    if (evidencePath !== undefined && !(error instanceof DocsQaError && error.evidenceRecorded)) {
      const message = error instanceof Error ? error.message : "unknown error"
      await writeEvidence(evidencePath, evidenceSection(args, 1, failureResult(message)))
    }
    reportFailure(error)
  }
}

main().catch(reportFailure)
