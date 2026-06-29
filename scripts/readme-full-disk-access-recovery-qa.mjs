#!/usr/bin/env node

import { mkdir, readFile, writeFile } from "node:fs/promises"
import { homedir } from "node:os"
import { dirname } from "node:path"

const requiredPhrases = [
  "Full Disk Access",
  "Open Full Disk Access",
  "Morrow.app",
  "Cmd+Shift+G",
  "target/debug/morrow",
  "restart Morrow",
  "Retry chat discovery",
  "Terminal",
  "No OAuth is required for native Messages discovery"
]

const forbiddenClaims = [
  "Morrow grants Full Disk Access automatically",
  "bundling is required to open Full Disk Access",
  "Terminal Full Disk Access grants Morrow access"
]

const usage =
  "Usage: node scripts/readme-full-disk-access-recovery-qa.mjs README.md .omo/evidence/full-disk-access-guided-recovery/task-5-docs.md"

class ReadmeQaError extends Error {
  constructor(message, failures, evidenceRecorded = false) {
    super(message)
    this.name = "ReadmeQaError"
    this.failures = failures
    this.evidenceRecorded = evidenceRecorded
  }
}

function parseArgs(args) {
  const [readmePath, evidencePath, ...extraArgs] = args
  if (readmePath === undefined || evidencePath === undefined || extraArgs.length > 0) {
    throw new ReadmeQaError(usage, [
      "expected exactly two arguments: <README path> <evidence path>"
    ])
  }
  return { readmePath, evidencePath }
}

function findHomePathFindings(readmeText) {
  const currentHome = homedir()
  const findings = []
  if (currentHome !== "/" && readmeText.includes(currentHome)) {
    findings.push(`current user home path present: ${currentHome}`)
  }
  const genericHomePaths = readmeText.match(/\/Users\/[A-Za-z0-9._-]+(?:\/[^\s`'")\]]*)?/g)
  if (genericHomePaths !== null) {
    for (const homePath of genericHomePaths) {
      findings.push(`absolute home path present: ${homePath}`)
    }
  }
  return [...new Set(findings)]
}

function evaluateReadme(readmeText) {
  const missingRequired = requiredPhrases.filter((phrase) => !readmeText.includes(phrase))
  const presentForbidden = forbiddenClaims.filter((claim) => readmeText.includes(claim))
  const homePathFindings = findHomePathFindings(readmeText)

  return {
    argumentFailures: [],
    missingRequired,
    presentForbidden,
    homePathFindings,
    failures: [
      ...missingRequired.map((phrase) => `missing required phrase: ${phrase}`),
      ...presentForbidden.map((claim) => `forbidden claim present: ${claim}`),
      ...homePathFindings
    ]
  }
}

function markdownList(items, emptyText) {
  if (items.length === 0) {
    return [`- ${emptyText}`]
  }
  return items.map((item) => `- ${item}`)
}

function requiredPhraseRows(phrases, missingPhrases) {
  return phrases.map((phrase) => {
    const status = missingPhrases.includes(phrase) ? "missing required phrase" : "present"
    return `| \`${phrase}\` | ${status} |`
  })
}

function forbiddenPhraseRows(phrases, presentClaims) {
  return phrases.map((phrase) => {
    const status = presentClaims.includes(phrase) ? "forbidden claim present" : "absent"
    return `| \`${phrase}\` | ${status} |`
  })
}

function evidenceSection(args, exitStatus, result) {
  const status = exitStatus === 0 ? "PASS" : "FAIL"
  return [
    "",
    `## README Full Disk Access Recovery QA - ${new Date().toISOString()}`,
    "",
    `Command: \`node scripts/readme-full-disk-access-recovery-qa.mjs ${args.join(" ")}\``,
    `Exit status: ${exitStatus}`,
    `Result: ${status}`,
    "",
    "Missing required phrases:",
    ...markdownList(result.missingRequired, "none"),
    "",
    "Argument failures:",
    ...markdownList(result.argumentFailures, "none"),
    "",
    "Forbidden claims present:",
    ...markdownList(result.presentForbidden, "none"),
    "",
    "User-specific absolute home path findings:",
    ...markdownList(result.homePathFindings, "none"),
    "",
    "| Phrase | Assertion |",
    "| --- | --- |",
    ...requiredPhraseRows(requiredPhrases, result.missingRequired),
    ...forbiddenPhraseRows(forbiddenClaims, result.presentForbidden),
    `| user-specific absolute home path | ${result.homePathFindings.length === 0 ? "absent" : "present"} |`,
    ""
  ].join("\n")
}

async function appendEvidence(evidencePath, section) {
  await mkdir(dirname(evidencePath), { recursive: true })
  await writeFile(evidencePath, `${section}\n`, { flag: "a" })
}

function reportFailure(error) {
  if (error instanceof ReadmeQaError) {
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
  console.error("Unknown README Full Disk Access recovery QA failure")
  process.exitCode = 1
}

async function main() {
  const args = process.argv.slice(2)
  const evidencePath = args[1]
  try {
    const invocation = parseArgs(args)
    const readmeText = await readFile(invocation.readmePath, "utf8")
    const result = evaluateReadme(readmeText)
    if (result.failures.length > 0) {
      await appendEvidence(invocation.evidencePath, evidenceSection(args, 1, result))
      throw new ReadmeQaError("README Full Disk Access recovery QA failed", result.failures, true)
    }
    await appendEvidence(invocation.evidencePath, evidenceSection(args, 0, result))
    console.log("PASS README Full Disk Access recovery QA")
  } catch (error) {
    if (
      error instanceof ReadmeQaError &&
      evidencePath !== undefined &&
      !error.evidenceRecorded
    ) {
      const result = {
        argumentFailures: error.failures,
        missingRequired: [],
        presentForbidden: [],
        homePathFindings: []
      }
      await appendEvidence(evidencePath, evidenceSection(args, 1, result))
    }
    reportFailure(error)
  }
}

main().catch(reportFailure)
