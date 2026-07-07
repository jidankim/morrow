#!/usr/bin/env node

import { mkdir, readFile, writeFile } from "node:fs/promises"
import { dirname } from "node:path"
import { forbiddenPatterns, requiredClaims } from "./wider-product-rollout/docs-qa-rules.mjs"

const usage =
  "Usage: node scripts/wider-product-rollout-docs-qa.mjs docs/wider-product-rollout.md docs/privacy.md README.md docs/beta-testing.md .omo/evidence/phase-7-wider-product-rollout/task-4/docs-qa.md"

class DocsQaError extends Error {
  constructor(message, failures, evidenceRecorded = false) {
    super(message)
    this.name = "DocsQaError"
    this.failures = failures
    this.evidenceRecorded = evidenceRecorded
  }
}

function parseArgs(args) {
  const [rolloutPath, privacyPath, readmePath, betaPath, evidencePath, ...extraArgs] = args
  if (
    rolloutPath === undefined ||
    privacyPath === undefined ||
    readmePath === undefined ||
    betaPath === undefined ||
    evidencePath === undefined ||
    extraArgs.length > 0
  ) {
    throw new DocsQaError(usage, [
      "expected exactly five arguments: <rollout doc> <privacy doc> <README> <beta doc> <evidence path>"
    ])
  }
  return { rolloutPath, privacyPath, readmePath, betaPath, evidencePath }
}

function requiredClaimResults(combinedText) {
  return requiredClaims.map((claim) => ({
    name: claim.name,
    passed: claim.phrases.some((phrase) => combinedText.includes(phrase)),
    phrases: claim.phrases
  }))
}

function forbiddenResults(combinedText) {
  return forbiddenPatterns.map((claim) => ({
    name: claim.name,
    present: claim.pattern.test(combinedText)
  }))
}

function evaluateDocs(docs) {
  const combinedText = docs.map((doc) => doc.text).join("\n")
  const emptyDocs = docs.filter((doc) => doc.text.trim().length === 0).map((doc) => doc.path)
  const required = requiredClaimResults(combinedText)
  const forbidden = forbiddenResults(combinedText)
  const failures = [
    ...emptyDocs.map((path) => `${path} is empty`),
    ...required.filter((claim) => !claim.passed).map((claim) => `missing required claim: ${claim.name}`),
    ...forbidden.filter((claim) => claim.present).map((claim) => `forbidden overclaim or private content present: ${claim.name}`)
  ]
  return { emptyDocs, required, forbidden, failures }
}

function listLines(items, emptyText) {
  return items.length === 0 ? [`- ${emptyText}`] : items.map((item) => `- ${item}`)
}

function evidenceSection(args, exitStatus, result) {
  const status = exitStatus === 0 ? "PASS" : "FAIL"
  return [
    `# Wider Product Rollout Docs QA`,
    "",
    `Scenario: public rollout and privacy docs block overclaims`,
    `Invocation: \`node scripts/wider-product-rollout-docs-qa.mjs ${args.join(" ")}\``,
    `Binary observable: process exit status ${exitStatus}`,
    `Result: ${status}`,
    `Recorded at: ${new Date().toISOString()}`,
    "",
    "Empty docs:",
    ...listLines(result.emptyDocs, "none"),
    "",
    "Required claims:",
    ...result.required.map((claim) => `- ${claim.name}: ${claim.passed ? "PASS" : "FAIL"}`),
    "",
    "Forbidden overclaims and private-content classes:",
    ...result.forbidden.map((claim) => `- ${claim.name}: ${claim.present ? "FAIL present" : "PASS absent"}`),
    "",
    "Failures:",
    ...listLines(result.failures, "none"),
    "",
    "| Assertion | Result |",
    "| --- | --- |",
    ...result.required.map((claim) => `| required: ${claim.name} | ${claim.passed ? "PASS" : "FAIL"} |`),
    ...result.forbidden.map((claim) => `| forbidden: ${claim.name} | ${claim.present ? "FAIL" : "PASS"} |`),
    ""
  ].join("\n")
}

async function writeEvidence(evidencePath, section) {
  await mkdir(dirname(evidencePath), { recursive: true })
  await writeFile(evidencePath, `${section}\n`)
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
  console.error("Unknown wider product rollout docs QA failure")
  process.exitCode = 1
}

async function main() {
  const args = process.argv.slice(2)
  const evidencePath = args[4]
  try {
    const invocation = parseArgs(args)
    const paths = [
      invocation.rolloutPath,
      invocation.privacyPath,
      invocation.readmePath,
      invocation.betaPath
    ]
    const texts = await Promise.all(paths.map((path) => readFile(path, "utf8")))
    const result = evaluateDocs(paths.map((path, index) => ({ path, text: texts[index] })))
    if (result.failures.length > 0) {
      await writeEvidence(invocation.evidencePath, evidenceSection(args, 1, result))
      throw new DocsQaError("Wider product rollout docs QA failed", result.failures, true)
    }
    await writeEvidence(invocation.evidencePath, evidenceSection(args, 0, result))
    console.log("PASS wider product rollout docs QA")
    console.log(`evidence: ${invocation.evidencePath}`)
  } catch (error) {
    if (evidencePath !== undefined && !(error instanceof DocsQaError && error.evidenceRecorded)) {
      const result = {
        emptyDocs: [],
        required: requiredClaims.map((claim) => ({ name: claim.name, passed: false })),
        forbidden: forbiddenPatterns.map((claim) => ({ name: claim.name, present: false })),
        failures: error instanceof DocsQaError ? error.failures : [error.message]
      }
      await writeEvidence(evidencePath, evidenceSection(args, 1, result))
    }
    reportFailure(error)
  }
}

main().catch(reportFailure)
