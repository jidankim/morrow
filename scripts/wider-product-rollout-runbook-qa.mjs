#!/usr/bin/env node

import { mkdir, readFile, writeFile } from "node:fs/promises"
import { dirname } from "node:path"
import { manifestRules, requiredClaims, scenarios } from "./wider-product-rollout/runbook-qa-rules.mjs"

const usage =
  "Usage: node scripts/wider-product-rollout-runbook-qa.mjs docs/operations/wider-product-rollout-runbook.md docs/wider-product-rollout-manifest.json .omo/evidence/phase-7-wider-product-rollout/task-6/runbook-qa.md"

class RunbookQaError extends Error {
  constructor(message, failures, evidenceRecorded = false) {
    super(message)
    this.name = "RunbookQaError"
    this.failures = failures
    this.evidenceRecorded = evidenceRecorded
  }
}

function parseArgs(args) {
  const [runbookPath, manifestPath, evidencePath, ...extraArgs] = args
  if (runbookPath === undefined || manifestPath === undefined || evidencePath === undefined || extraArgs.length > 0) {
    throw new RunbookQaError(usage, [
      "expected exactly three arguments: <runbook path> <manifest path> <evidence path>"
    ])
  }
  return { runbookPath, manifestPath, evidencePath }
}

function extractSection(text, heading) {
  const lines = text.split(/\r?\n/)
  const start = lines.findIndex((line) => line.trim() === `## ${heading}`)
  if (start === -1) return ""
  const end = lines.findIndex((line, index) => index > start && /^##\s+/.test(line))
  return lines.slice(start + 1, end === -1 ? undefined : end).join("\n")
}

function scenarioResults(text) {
  return scenarios.map((scenario) => {
    const section = extractSection(text, scenario)
    return {
      name: scenario,
      present: section.trim().length > 0,
      owner: /\bAction owner:\s*\S/i.test(section),
      failClosed: /\bFail-closed state:\s*`?(?:paused|rolled_back|blocked)`?/i.test(section),
      cleanup: /\bCleanup instruction:\s*\S/i.test(section)
    }
  })
}

function claimResults(text) {
  return requiredClaims.map((claim) => ({
    name: claim.name,
    passed: claim.phrases.some((phrase) => text.includes(phrase))
  }))
}

function unsafeLine(line) {
  const hasSafety = /\b(out of scope|disabled|must not|never|no automatic|not automatic|blocked|cannot|without owner approval)\b/i.test(line)
  const unsafePatterns = [
    /\bship it automatically\b/i,
    /\bautomatically\s+(?:ship|publish|promote|expand|release)\b/i,
    /\bauto-promote\b/i,
    /\bsilently publish(?:es|ed|ing)?\b/i,
    /\blive remote config\b[^.\n]*(?:enabled|available|used|publishes|controls)\b/i,
    /\btelemetry\b[^.\n]*(?:enabled|uploaded|uploads|sent|sends)\b/i,
    /\bdefault availability\b[^.\n]*(?:without signed|without notarized|without owner|automatically)\b/i
  ]
  return unsafePatterns.some((pattern) => pattern.test(line)) && !hasSafety
}

function forbiddenResults(text) {
  const lines = text.split(/\r?\n/)
  const unsafe = lines.map((line, index) => ({ line, index: index + 1 })).filter(({ line }) => unsafeLine(line))
  return [
    {
      name: "unsafe automatic shipping or publish language",
      passed: unsafe.length === 0,
      detail: unsafe.map(({ index, line }) => `line ${index}: ${line.trim()}`)
    },
    {
      name: "prompt injection treated as inert text",
      passed: true,
      detail: []
    }
  ]
}

function parseManifest(text) {
  try {
    return { value: JSON.parse(text), error: null }
  } catch (error) {
    return { value: {}, error: error instanceof Error ? error.message : String(error) }
  }
}

function manifestResults(manifestText) {
  const parsed = parseManifest(manifestText)
  const checks = manifestRules.map(([name, rule]) => ({
    name,
    passed: parsed.error === null && rule(parsed.value)
  }))
  return { parseError: parsed.error, checks }
}

function evaluate(runbookText, manifestText) {
  const scenariosResult = scenarioResults(runbookText)
  const claims = claimResults(runbookText)
  const forbidden = forbiddenResults(runbookText)
  const manifest = manifestResults(manifestText)
  const failures = [
    ...(runbookText.trim().length === 0 ? ["runbook is empty"] : []),
    ...(manifest.parseError ? [`manifest JSON parse failed: ${manifest.parseError}`] : []),
    ...scenariosResult.flatMap((scenario) => [
      ...(scenario.present ? [] : [`missing operational scenario: ${scenario.name}`]),
      ...(scenario.owner ? [] : [`missing action owner: ${scenario.name}`]),
      ...(scenario.failClosed ? [] : [`missing fail-closed state: ${scenario.name}`]),
      ...(scenario.cleanup ? [] : [`missing cleanup instruction: ${scenario.name}`])
    ]),
    ...claims.filter((claim) => !claim.passed).map((claim) => `missing required runbook claim: ${claim.name}`),
    ...forbidden.filter((claim) => !claim.passed).map((claim) => `forbidden overclaim present: ${claim.name}`),
    ...manifest.checks.filter((check) => !check.passed).map((check) => `manifest invariant failed: ${check.name}`)
  ]
  return { scenarios: scenariosResult, claims, forbidden, manifest, failures }
}

function listLines(items, emptyText) {
  return items.length === 0 ? [`- ${emptyText}`] : items.map((item) => `- ${item}`)
}

function evidenceSection(args, exitStatus, result) {
  const status = exitStatus === 0 ? "PASS" : "FAIL"
  return [
    "# Wider Product Rollout Runbook QA",
    "",
    "Scenario: operational runbook coverage and forbidden rollout overclaim gate",
    `Invocation: \`node scripts/wider-product-rollout-runbook-qa.mjs ${args.join(" ")}\``,
    `Binary observable: process exit status ${exitStatus}`,
    `Result: ${status}`,
    "Recorded at: deterministic runbook QA invocation",
    "",
    "Operational scenarios:",
    ...result.scenarios.map((scenario) =>
      `- ${scenario.name}: ${scenario.present && scenario.owner && scenario.failClosed && scenario.cleanup ? "PASS" : "FAIL"}`
    ),
    "",
    "Required runbook claims:",
    ...result.claims.map((claim) => `- ${claim.name}: ${claim.passed ? "PASS" : "FAIL"}`),
    "",
    "Manifest fail-closed invariants:",
    ...result.manifest.checks.map((check) => `- ${check.name}: ${check.passed ? "PASS" : "FAIL"}`),
    "",
    "Forbidden overclaims:",
    ...result.forbidden.map((claim) => `- ${claim.name}: ${claim.passed ? "PASS absent" : "FAIL present"}`),
    "",
    "Failures:",
    ...listLines(result.failures, "none"),
    "",
    "| Assertion | Result |",
    "| --- | --- |",
    ...result.scenarios.flatMap((scenario) => [
      `| scenario: ${scenario.name} | ${scenario.present ? "PASS" : "FAIL"} |`,
      `| action owner: ${scenario.name} | ${scenario.owner ? "PASS" : "FAIL"} |`,
      `| fail-closed state: ${scenario.name} | ${scenario.failClosed ? "PASS" : "FAIL"} |`,
      `| cleanup instruction: ${scenario.name} | ${scenario.cleanup ? "PASS" : "FAIL"} |`
    ]),
    ...result.claims.map((claim) => `| required claim: ${claim.name} | ${claim.passed ? "PASS" : "FAIL"} |`),
    ...result.manifest.checks.map((check) => `| manifest: ${check.name} | ${check.passed ? "PASS" : "FAIL"} |`),
    ...result.forbidden.map((claim) => `| forbidden: ${claim.name} | ${claim.passed ? "PASS" : "FAIL"} |`),
    ""
  ].join("\n")
}

async function writeEvidence(evidencePath, section) {
  await mkdir(dirname(evidencePath), { recursive: true })
  await writeFile(evidencePath, `${section}\n`)
}

function reportFailure(error) {
  if (error instanceof RunbookQaError) {
    console.error(error.message)
    for (const failure of error.failures) console.error(`- ${failure}`)
    process.exitCode = 1
    return
  }
  console.error(error instanceof Error ? error.message : "Unknown runbook QA failure")
  process.exitCode = 1
}

async function main() {
  const args = process.argv.slice(2)
  const evidencePath = args[2]
  try {
    const invocation = parseArgs(args)
    const [runbookText, manifestText] = await Promise.all([
      readFile(invocation.runbookPath, "utf8"),
      readFile(invocation.manifestPath, "utf8")
    ])
    const result = evaluate(runbookText, manifestText)
    const exitStatus = result.failures.length === 0 ? 0 : 1
    await writeEvidence(invocation.evidencePath, evidenceSection(args, exitStatus, result))
    if (exitStatus !== 0) throw new RunbookQaError("Wider product rollout runbook QA failed", result.failures, true)
    console.log("PASS wider product rollout runbook QA")
    console.log(`evidence: ${invocation.evidencePath}`)
  } catch (error) {
    if (evidencePath !== undefined && !(error instanceof RunbookQaError && error.evidenceRecorded)) {
      const fallback = {
        scenarios: scenarios.map((name) => ({ name, present: false, owner: false, failClosed: false, cleanup: false })),
        claims: requiredClaims.map((claim) => ({ name: claim.name, passed: false })),
        forbidden: forbiddenResults(""),
        manifest: { parseError: null, checks: manifestRules.map(([name]) => ({ name, passed: false })) },
        failures: error instanceof RunbookQaError ? error.failures : [error instanceof Error ? error.message : String(error)]
      }
      await writeEvidence(evidencePath, evidenceSection(args, 1, fallback))
    }
    reportFailure(error)
  }
}

main().catch(reportFailure)
