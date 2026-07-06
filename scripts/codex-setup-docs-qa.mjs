#!/usr/bin/env node

import { mkdir, readFile, writeFile } from "node:fs/promises"
import { dirname } from "node:path"

const usage =
  "Usage: node scripts/codex-setup-docs-qa.mjs README.md docs/beta-testing.md docs/diagnostic-testing.md .omo/evidence/task-5-native-codex-login-flow.md"

const requiredPhrases = [
  "Install Codex CLI",
  "Start Codex login",
  "explicit confirmation",
  "Codex owns the login",
  "Morrow does not store provider tokens",
  "provider-backed Sync Now"
]

const forbiddenPhrases = [
  "auto-installs Codex",
  "bundles Codex CLI",
  "paste tokens",
  "share auth files",
  "auth.json",
  "~/.codex",
  "CODEX_ACCESS_TOKEN",
  "sk-",
  "Morrow manages Codex login"
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
  const [readmePath, betaDocPath, diagnosticDocPath, evidencePath, ...extraArgs] = args
  if (
    readmePath === undefined ||
    betaDocPath === undefined ||
    diagnosticDocPath === undefined ||
    evidencePath === undefined ||
    extraArgs.length > 0
  ) {
    throw new DocsQaError(usage, [
      "expected exactly four arguments: <README path> <beta-testing path> <diagnostic-testing path> <evidence path>"
    ])
  }
  return { readmePath, betaDocPath, diagnosticDocPath, evidencePath }
}

function missingPhrases(text, phrases) {
  return phrases.filter((phrase) => !text.includes(phrase))
}

function presentPhrases(text, phrases) {
  return phrases.filter((phrase) => text.includes(phrase))
}

function evaluateDocs(readmeText, betaDocText, diagnosticDocText) {
  const docs = [
    ["README", readmeText],
    ["docs/beta-testing.md", betaDocText],
    ["docs/diagnostic-testing.md", diagnosticDocText]
  ]
  const combinedText = docs.map(([, text]) => text).join("\n")
  const emptyDocs = docs.filter(([, text]) => text.trim().length === 0).map(([name]) => name)
  const missingRequired = missingPhrases(combinedText, requiredPhrases)
  const presentForbidden = presentPhrases(combinedText, forbiddenPhrases)
  const docCoverage = docs.map(([name, text]) => ({
    name,
    mentionsInstall: text.includes("Install Codex CLI"),
    mentionsLogin: text.includes("Start Codex login"),
    mentionsCredentialBoundary:
      text.includes("Codex owns the login") &&
      text.includes("Morrow does not store provider tokens")
  }))
  const docsMissingCoverage = docCoverage
    .filter(
      (doc) => !doc.mentionsInstall || !doc.mentionsLogin || !doc.mentionsCredentialBoundary
    )
    .map((doc) => doc.name)

  const failures = [
    ...emptyDocs.map((name) => `${name} is empty`),
    ...missingRequired.map((phrase) => `missing required phrase: ${phrase}`),
    ...presentForbidden.map((phrase) => `forbidden phrase present: ${phrase}`),
    ...docsMissingCoverage.map(
      (name) => `${name} does not cover app-assisted install, login, and credential boundary`
    )
  ]

  return {
    emptyDocs,
    missingRequired,
    presentForbidden,
    docCoverage,
    failures
  }
}

function listLines(items, emptyText) {
  return items.length === 0 ? [`- ${emptyText}`] : items.map((item) => `- ${item}`)
}

function rowsForPhrases(label, phrases, badPhrases, passingStatus, failingStatus) {
  return phrases.map((phrase) => {
    const status = badPhrases.includes(phrase) ? failingStatus : passingStatus
    return `| ${label}: \`${phrase}\` | ${status} |`
  })
}

function evidenceSection(args, exitStatus, result) {
  const status = exitStatus === 0 ? "PASS" : "FAIL"
  return [
    "",
    `## Codex Setup Docs QA - ${new Date().toISOString()}`,
    "",
    `Scenario: docs describe app-assisted Codex setup without credential handling`,
    `Invocation: \`node scripts/codex-setup-docs-qa.mjs ${args.join(" ")}\``,
    `Binary observable: process exit status ${exitStatus}`,
    `Result: ${status}`,
    "",
    "Required phrases missing:",
    ...listLines(result.missingRequired, "none"),
    "",
    "Forbidden phrases present:",
    ...listLines(result.presentForbidden, "none"),
    "",
    "Empty docs:",
    ...listLines(result.emptyDocs, "none"),
    "",
    "Per-document coverage:",
    ...result.docCoverage.map(
      (doc) =>
        `- ${doc.name}: install=${doc.mentionsInstall ? "PASS" : "FAIL"}, login=${
          doc.mentionsLogin ? "PASS" : "FAIL"
        }, credential_boundary=${doc.mentionsCredentialBoundary ? "PASS" : "FAIL"}`
    ),
    "",
    "Failures:",
    ...listLines(result.failures, "none"),
    "",
    "| Assertion | Result |",
    "| --- | --- |",
    ...rowsForPhrases("required", requiredPhrases, result.missingRequired, "PASS present", "FAIL missing"),
    ...rowsForPhrases("forbidden", forbiddenPhrases, result.presentForbidden, "PASS absent", "FAIL present"),
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
  console.error("Unknown Codex setup docs QA failure")
  process.exitCode = 1
}

async function main() {
  const args = process.argv.slice(2)
  const evidencePath = args[3]
  try {
    const invocation = parseArgs(args)
    const [readmeText, betaDocText, diagnosticDocText] = await Promise.all([
      readFile(invocation.readmePath, "utf8"),
      readFile(invocation.betaDocPath, "utf8"),
      readFile(invocation.diagnosticDocPath, "utf8")
    ])
    const result = evaluateDocs(readmeText, betaDocText, diagnosticDocText)
    if (result.failures.length > 0) {
      await appendEvidence(invocation.evidencePath, evidenceSection(args, 1, result))
      throw new DocsQaError("Codex setup docs QA failed", result.failures, true)
    }
    await appendEvidence(invocation.evidencePath, evidenceSection(args, 0, result))
    console.log("PASS codex setup docs QA")
  } catch (error) {
    if (evidencePath !== undefined && !(error instanceof DocsQaError && error.evidenceRecorded)) {
      const result = {
        emptyDocs: [],
        missingRequired: [],
        presentForbidden: [],
        docCoverage: [],
        failures: error instanceof DocsQaError ? error.failures : [error.message]
      }
      await appendEvidence(evidencePath, evidenceSection(args, 1, result))
    }
    reportFailure(error)
  }
}

main().catch(reportFailure)
