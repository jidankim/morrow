#!/usr/bin/env node

import { mkdir, readFile, writeFile } from "node:fs/promises"
import { dirname } from "node:path"

const requiredPhrases = [
  "Full Disk Access",
  "Retry chat discovery",
  "Select at least one chat",
  "Sync Now",
  "Messages discovery",
  "No OAuth is required for native Messages discovery",
  "latest-message body preview is not part of the default MVP"
]

const forbiddenClaims = [
  "Codex OAuth is required",
  "Apple OAuth is required",
  "latest message body preview is enabled by default",
  "Messages discovery creates calendar events"
]

class ReadmeQaError extends Error {
  constructor(message, failures) {
    super(message)
    this.name = "ReadmeQaError"
    this.failures = failures
  }
}

function parseArgs(args) {
  const [readmePath, evidencePath, ...extraArgs] = args
  if (readmePath === undefined || evidencePath === undefined || extraArgs.length > 0) {
    throw new ReadmeQaError(
      "Usage: node scripts/readme-chat-discovery-qa.mjs README.md .omo/evidence/task-7-native-chat-discovery-ui-friendly.md",
      ["expected exactly two arguments: <README path> <evidence path>"]
    )
  }
  return { readmePath, evidencePath }
}

function assertReadmeCoverage(readmeText) {
  const missingRequired = requiredPhrases.filter((phrase) => !readmeText.includes(phrase))
  const presentForbidden = forbiddenClaims.filter((claim) => readmeText.includes(claim))
  const failures = [
    ...missingRequired.map((phrase) => `missing required phrase: ${phrase}`),
    ...presentForbidden.map((claim) => `forbidden claim present: ${claim}`)
  ]
  if (failures.length > 0) {
    throw new ReadmeQaError("README chat discovery QA failed", failures)
  }
}

function markdownStatusRows(phrases, status) {
  return phrases.map((phrase) => `| \`${phrase}\` | ${status} |`)
}

function evidenceTable(readmePath) {
  return [
    "",
    "## README Chat Discovery Phrase QA",
    "",
    `Command: \`node scripts/readme-chat-discovery-qa.mjs ${readmePath} .omo/evidence/task-7-native-chat-discovery-ui-friendly.md\``,
    "",
    "Result: PASS, all README phrase assertions passed before this table was written.",
    "",
    "| Phrase | Assertion |",
    "| --- | --- |",
    ...markdownStatusRows(requiredPhrases, "required phrase present"),
    ...markdownStatusRows(forbiddenClaims, "forbidden claim absent"),
    ""
  ].join("\n")
}

async function appendEvidence(evidencePath, table) {
  await mkdir(dirname(evidencePath), { recursive: true })
  await writeFile(evidencePath, `${table}\n`, { flag: "a" })
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
  console.error("Unknown README chat discovery QA failure")
  process.exitCode = 1
}

async function main() {
  const { readmePath, evidencePath } = parseArgs(process.argv.slice(2))
  const readmeText = await readFile(readmePath, "utf8")
  assertReadmeCoverage(readmeText)
  await appendEvidence(evidencePath, evidenceTable(readmePath))
  console.log("PASS README chat discovery QA")
}

main().catch(reportFailure)
