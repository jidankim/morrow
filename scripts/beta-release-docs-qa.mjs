#!/usr/bin/env node

import { mkdir, readFile, writeFile } from "node:fs/promises"
import { dirname } from "node:path"

const usage =
  "Usage: node scripts/beta-release-docs-qa.mjs README.md docs/beta-testing.md .omo/evidence/task-4-beta-release-distribution.md"

const requiredReadmePhrases = [
  "Packaged beta testers",
  "docs/beta-testing.md",
  "Maintainer/source-build only",
  "npm install",
  "Rust",
  "Xcode/Command Line Tools",
  "SDKROOT",
  "npm run tauri:dev"
]

const requiredBetaPhrases = [
  "Download the beta DMG",
  "Open the DMG",
  "Drag `Morrow.app`",
  "Open Morrow",
  "Full Disk Access",
  "Calendar access",
  "Codex CLI",
  "`codex login`",
  "provider-backed `Sync Now`",
  "Manual QA flow",
  "Maintainer/source-build only"
]

const forbiddenClaims = [
  "bundles Codex CLI",
  "auto-installs Codex",
  "auto install Codex",
  "grants permissions automatically",
  "package launch proves real Messages-to-Calendar",
  "Testers must install Rust",
  "Testers must install Xcode",
  "paste tokens",
  "share auth files",
  "auth.json",
  "~/.codex",
  "sk-"
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
  const [readmePath, betaDocPath, evidencePath, ...extraArgs] = args
  if (
    readmePath === undefined ||
    betaDocPath === undefined ||
    evidencePath === undefined ||
    extraArgs.length > 0
  ) {
    throw new DocsQaError(usage, [
      "expected exactly three arguments: <README path> <beta-testing path> <evidence path>"
    ])
  }
  return { readmePath, betaDocPath, evidencePath }
}

function findMissingPhrases(text, phrases) {
  return phrases.filter((phrase) => !text.includes(phrase))
}

function findForbiddenClaims(text) {
  return forbiddenClaims.filter((claim) => text.includes(claim))
}

function evaluateDocs(readmeText, betaDocText) {
  const missingReadmePhrases = findMissingPhrases(readmeText, requiredReadmePhrases)
  const missingBetaPhrases = findMissingPhrases(betaDocText, requiredBetaPhrases)
  const emptyDocs = []
  if (readmeText.trim().length === 0) {
    emptyDocs.push("README is empty")
  }
  if (betaDocText.trim().length === 0) {
    emptyDocs.push("docs/beta-testing.md is empty")
  }

  const combinedText = `${readmeText}\n${betaDocText}`
  const presentForbidden = findForbiddenClaims(combinedText)
  const testerRequirementLeakPattern =
    /Packaged beta testers (?:must|should|need to|required to) (?:install|run) (?:npm install|Rust|Xcode\/Command Line Tools|Xcode|npm run tauri:dev)/i
  const testerVsMaintainerSeparated =
    missingReadmePhrases.length === 0 &&
    missingBetaPhrases.length === 0 &&
    !testerRequirementLeakPattern.test(combinedText)

  const failures = [
    ...emptyDocs,
    ...missingReadmePhrases.map((phrase) => `README missing required phrase: ${phrase}`),
    ...missingBetaPhrases.map((phrase) => `beta doc missing required phrase: ${phrase}`),
    ...presentForbidden.map((claim) => `forbidden claim present: ${claim}`)
  ]
  if (!testerVsMaintainerSeparated) {
    failures.push("tester-vs-maintainer prerequisite separation did not pass")
  }

  return {
    emptyDocs,
    missingReadmePhrases,
    missingBetaPhrases,
    presentForbidden,
    testerVsMaintainerSeparated,
    promptInjectionGuidanceClear: !presentForbidden.some((claim) =>
      ["paste tokens", "share auth files", "auth.json", "~/.codex", "sk-"].includes(claim)
    ),
    failures
  }
}

function markdownList(items, emptyText) {
  if (items.length === 0) {
    return [`- ${emptyText}`]
  }
  return items.map((item) => `- ${item}`)
}

function rowsForRequired(label, phrases, missingPhrases) {
  return phrases.map((phrase) => {
    const status = missingPhrases.includes(phrase) ? "FAIL missing" : "PASS present"
    return `| ${label}: \`${phrase}\` | ${status} |`
  })
}

function rowsForForbidden(phrases, presentClaims) {
  return phrases.map((phrase) => {
    const status = presentClaims.includes(phrase) ? "FAIL present" : "PASS absent"
    return `| forbidden: \`${phrase}\` | ${status} |`
  })
}

function evidenceSection(args, exitStatus, result) {
  const status = exitStatus === 0 ? "PASS" : "FAIL"
  return [
    "",
    `## Beta Release Docs QA - ${new Date().toISOString()}`,
    "",
    `Command: \`node scripts/beta-release-docs-qa.mjs ${args.join(" ")}\``,
    `Exit status: ${exitStatus}`,
    `Result: ${status}`,
    "",
    `Tester-vs-maintainer prerequisite separation: ${result.testerVsMaintainerSeparated ? "PASS" : "FAIL"}`,
    `Malformed or empty docs rejected: ${result.emptyDocs.length === 0 ? "PASS" : "FAIL"}`,
    `Prompt-injection/auth-file guidance check: ${result.promptInjectionGuidanceClear ? "PASS" : "FAIL"}`,
    "",
    "Missing README phrases:",
    ...markdownList(result.missingReadmePhrases, "none"),
    "",
    "Missing beta-testing phrases:",
    ...markdownList(result.missingBetaPhrases, "none"),
    "",
    "Forbidden claims present:",
    ...markdownList(result.presentForbidden, "none"),
    "",
    "General failures:",
    ...markdownList(result.failures ?? [], "none"),
    "",
    "| Assertion | Result |",
    "| --- | --- |",
    ...rowsForRequired("README", requiredReadmePhrases, result.missingReadmePhrases),
    ...rowsForRequired("beta", requiredBetaPhrases, result.missingBetaPhrases),
    ...rowsForForbidden(forbiddenClaims, result.presentForbidden),
    `| tester-vs-maintainer prerequisite separation | ${result.testerVsMaintainerSeparated ? "PASS" : "FAIL"} |`,
    ""
  ].join("\n")
}

async function appendEvidence(evidencePath, section) {
  await mkdir(dirname(evidencePath), { recursive: true })
  await writeFile(evidencePath, `${section}\n`, { flag: "a" })
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
  console.error("Unknown beta release docs QA failure")
  process.exitCode = 1
}

async function main() {
  const args = process.argv.slice(2)
  const evidencePath = args[2]
  try {
    const invocation = parseArgs(args)
    const [readmeText, betaDocText] = await Promise.all([
      readFile(invocation.readmePath, "utf8"),
      readFile(invocation.betaDocPath, "utf8")
    ])
    const result = evaluateDocs(readmeText, betaDocText)
    if (result.failures.length > 0) {
      await appendEvidence(invocation.evidencePath, evidenceSection(args, 1, result))
      throw new DocsQaError("Beta release docs QA failed", result.failures, true)
    }
    await appendEvidence(invocation.evidencePath, evidenceSection(args, 0, result))
    console.log("PASS beta release docs QA")
  } catch (error) {
    if (evidencePath !== undefined && !(error instanceof DocsQaError && error.evidenceRecorded)) {
      const result = {
        emptyDocs: [],
        missingReadmePhrases: [],
        missingBetaPhrases: [],
        presentForbidden: [],
        testerVsMaintainerSeparated: false,
        promptInjectionGuidanceClear: true,
        failures: error instanceof DocsQaError ? error.failures : [error.message]
      }
      await appendEvidence(evidencePath, evidenceSection(args, 1, result))
    }
    reportFailure(error)
  }
}

main().catch(reportFailure)
