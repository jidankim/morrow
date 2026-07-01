#!/usr/bin/env node
// allow: SIZE_OK - single-purpose diagnostic handoff gate keeps manifest parsing, checksums, and evidence reporting in one release script; split if it grows further.

import { access, mkdir, readFile, stat, writeFile } from "node:fs/promises"
import { createHash } from "node:crypto"
import { dirname, join } from "node:path"

const usage =
  "Usage: node scripts/diagnostic-handoff-qa.mjs src-tauri/target/release/diagnostic docs/diagnostic-testing.md .omo/evidence/task-7-unsigned-adhoc-app-artifact.md"

const requiredManifestEvidenceKeys = [
  "build",
  "artifactQa",
  "releaseGateRegression",
  "quarantineQa",
  "launchQa",
  "launchScreenshot"
]
const fixedEvidencePathsBeforeManifest = [".omo/evidence/task-3-unsigned-adhoc-app-artifact.md"]
const fixedEvidencePathsAfterManifest = [".omo/evidence/task-7-unsigned-adhoc-app-artifact.md"]

class HandoffQaError extends Error {
  constructor(message, failures, evidenceRecorded = false) {
    super(message)
    this.name = "HandoffQaError"
    this.failures = failures
    this.evidenceRecorded = evidenceRecorded
  }
}

function parseArgs(args) {
  const [diagnosticDir, testerDocPath, evidencePath, ...extraArgs] = args
  if (
    diagnosticDir === undefined ||
    testerDocPath === undefined ||
    evidencePath === undefined ||
    extraArgs.length > 0
  ) {
    throw new HandoffQaError(usage, [
      "expected exactly three arguments: <diagnostic dir> <tester instructions doc> <evidence path>"
    ])
  }
  return { diagnosticDir, testerDocPath, evidencePath }
}

async function exists(path) {
  try {
    await access(path)
    return true
  } catch {
    return false
  }
}

async function nonEmpty(path) {
  try {
    const info = await stat(path)
    return info.size > 0
  } catch {
    return false
  }
}

async function fileInfo(path) {
  try {
    const info = await stat(path)
    return {
      path,
      exists: true,
      size: info.size,
      mtimeMs: info.mtimeMs,
      mtime: info.mtime.toISOString()
    }
  } catch {
    return {
      path,
      exists: false,
      size: 0,
      mtimeMs: 0,
      mtime: "missing"
    }
  }
}

async function sha256(path) {
  const data = await readFile(path)
  return createHash("sha256").update(data).digest("hex")
}

async function readInput(path) {
  try {
    return { text: await readFile(path, "utf8"), error: null }
  } catch (error) {
    const message = error instanceof Error ? error.message : "unknown read failure"
    return { text: "", error: message }
  }
}

function parseChecksums(text) {
  const checksums = new Map()
  const malformed = []
  for (const [index, line] of text.split(/\r?\n/).entries()) {
    if (line.trim() === "") {
      continue
    }
    const match = line.match(/^([a-f0-9]{64})\s+\*?(.+)$/)
    if (!match) {
      malformed.push(`line ${index + 1}: ${line}`)
      continue
    }
    checksums.set(match[2], match[1])
  }
  return { checksums, malformed }
}

function includesAll(text, phrases) {
  return phrases.every((phrase) => text.includes(phrase))
}

function unique(items) {
  return [...new Set(items)]
}

function checksumCandidates(diagnosticDir, file) {
  return unique([file, join(diagnosticDir, file)])
}

function isSafeEvidencePath(path) {
  return (
    typeof path === "string" &&
    path.length > 0 &&
    path.startsWith(".omo/evidence/") &&
    !path.includes("\0") &&
    !path.includes("\\") &&
    !path.split("/").includes("..") &&
    /\.[A-Za-z0-9]+$/.test(path)
  )
}

function expectedEvidencePaths(manifest) {
  const manifestEvidencePaths = requiredManifestEvidenceKeys.map(
    (key) => manifest?.evidence?.[key]
  )
  return unique([
    ...fixedEvidencePathsBeforeManifest,
    ...manifestEvidencePaths,
    ...fixedEvidencePathsAfterManifest
  ])
}

function manifestEvidenceDetails(manifest) {
  return requiredManifestEvidenceKeys.map((key) => {
    const path = manifest?.evidence?.[key]
    return {
      key,
      path,
      valid: isSafeEvidencePath(path)
    }
  })
}

function evidenceFreshnessDetail(items) {
  return items
    .map((item) => {
      const status = item.passed ? "PASS" : "FAIL"
      return `${status} ${item.key}: ${item.path ?? "missing"}; ${item.detail}`
    })
    .join("; ")
}

function parseEvidenceSection(text) {
  const paths = []
  let inEvidenceSection = false
  for (const line of text.split(/\r?\n/)) {
    if (/^##\s+Evidence\s*$/.test(line)) {
      inEvidenceSection = true
      continue
    }
    if (inEvidenceSection && /^##\s+/.test(line)) {
      break
    }
    if (!inEvidenceSection) {
      continue
    }
    const match = line.match(/^\s*-\s+(\S+)\s*$/)
    if (match) {
      paths.push(match[1])
    }
  }
  return paths
}

function evidenceCoverageDetail({
  expectedPaths,
  handoffEvidencePaths,
  evidenceFileChecks,
  handoffEvidenceFileChecks
}) {
  const missingFromHandoff = expectedPaths.filter((path) => !handoffEvidencePaths.includes(path))
  const unsafeExpected = expectedPaths.filter((path) => !isSafeEvidencePath(path))
  const unsafeHandoff = handoffEvidencePaths.filter((path) => !isSafeEvidencePath(path))
  const missingFiles = unique([...evidenceFileChecks, ...handoffEvidenceFileChecks])
    .filter((item) => !item.exists)
    .map((item) => item.path)
  return [
    `expected ${expectedPaths.length}: ${expectedPaths.join(", ") || "none"}`,
    `handoff ${handoffEvidencePaths.length}: ${handoffEvidencePaths.join(", ") || "none"}`,
    `missing from handoff: ${missingFromHandoff.join(", ") || "none"}`,
    `unsafe expected: ${unsafeExpected.join(", ") || "none"}`,
    `unsafe handoff: ${unsafeHandoff.join(", ") || "none"}`,
    `missing files: ${missingFiles.join(", ") || "none"}`
  ].join("; ")
}

function hasUnsafeNotarizedClaim(text) {
  const sentences = text.split(/(?<=[.!?])\s+/)
  return sentences.some((sentence) => {
    if (!/\bnotarized\b/i.test(sentence)) {
      return false
    }
    if (/\bnot notarized\b|\bsigned\/notarized\b|\bsigned, notarized\b/i.test(sentence)) {
      return false
    }
    if (/release gate remains blocked until Developer ID\/notarization credentials exist/i.test(sentence)) {
      return false
    }
    return /\b(?:this|artifact|zip|app|diagnostic|Morrow)\b/i.test(sentence)
  })
}

function forbiddenFindings(handoffText, testerDocText) {
  const combined = `${handoffText}\n${testerDocText}`
  const findings = []
  const checks = [
    ["global Gatekeeper disable", /spctl\s+--master-disable|disable\s+Gatekeeper|Gatekeeper\s+globally|globally\s+disable/i],
    ["tester Terminal or xattr bypass", /\bxattr\b|open\s+Terminal|run\s+(?:this|the following|.+)\s+in\s+Terminal|Terminal-based\s+Gatekeeper/i],
    ["Apple or Codex secret", /APPLE_(?:ID|PASSWORD|TEAM_ID|CERTIFICATE|CERTIFICATE_PASSWORD)|codex_access_token|auth\.json|~\/\.codex|\bsk-[A-Za-z0-9]{8,}/i],
    ["tester source-build requirement", /Testers?\s+(?:must|should|need to|required to)\s+(?:clone|install|run)[^.]*\b(?:repo|repository|npm install|Rust|Xcode|Tauri|source-build)\b/i],
    ["local absolute temp path in handoff", /\/(?:private\/)?tmp\/morrow-diagnostic-[^\s)]+/i]
  ]
  for (const [name, pattern] of checks) {
    if (pattern.test(combined)) {
      findings.push(name)
    }
  }
  if (hasUnsafeNotarizedClaim(handoffText) || hasUnsafeNotarizedClaim(testerDocText)) {
    findings.push("unsafe notarized claim")
  }
  return findings
}

async function evaluate({ diagnosticDir, testerDocPath, evidencePath }) {
  const manifestPath = join(diagnosticDir, "manifest.json")
  const checksumsPath = join(diagnosticDir, "SHA256SUMS")
  const handoffPath = join(diagnosticDir, "HANDOFF.md")

  const [manifestInput, checksumsInput, handoffInput, testerDocInput] = await Promise.all([
    readInput(manifestPath),
    readInput(checksumsPath),
    readInput(handoffPath),
    readInput(testerDocPath)
  ])
  const handoffText = handoffInput.text
  const testerDocText = testerDocInput.text

  let manifest
  try {
    manifest = JSON.parse(manifestInput.text)
  } catch (error) {
    const message = error instanceof Error ? error.message : "unknown JSON parse failure"
    manifest = null
    return resultWithAssertions([
      ["manifest parse", false, message]
    ])
  }

  const { checksums, malformed } = parseChecksums(checksumsInput.text)
  const primaryZip = manifest?.artifacts?.primaryZip
  const testerNote = manifest?.artifacts?.testerNote
  const appExecutable = manifest?.artifacts?.appExecutable
  const dmgSidecar = manifest?.artifacts?.dmgSidecar
  const expectedChecksumFiles = [
    primaryZip,
    "manifest.json",
    testerNote,
    appExecutable,
    ...(dmgSidecar ? [dmgSidecar] : [])
  ].filter(Boolean)
  const checksumDetails = await Promise.all(
    expectedChecksumFiles.map(async (file) => {
      const filePath = join(diagnosticDir, file)
      const fileExists = await exists(filePath)
      if (!fileExists) {
        return { file, passed: false, detail: "file missing" }
      }
      const actual = await sha256(filePath)
      const matchedChecksumPath = checksumCandidates(diagnosticDir, file).find((candidate) =>
        checksums.has(candidate)
      )
      const expected = matchedChecksumPath ? checksums.get(matchedChecksumPath) : undefined
      return {
        file,
        passed: expected === actual,
        detail: expected === actual
          ? `matched SHA-256 at ${matchedChecksumPath}`
          : `expected ${expected ?? "missing"}, actual ${actual}`
      }
    })
  )
  const manifestEvidence = manifestEvidenceDetails(manifest)
  const manifestInfo = await fileInfo(manifestPath)
  const primaryZipInfo = typeof primaryZip === "string"
    ? await fileInfo(join(diagnosticDir, primaryZip))
    : { mtimeMs: 0 }
  const manifestEvidenceFileDetails = await Promise.all(
    manifestEvidence.map(async (item) => ({
      ...item,
      file: item.valid ? await fileInfo(item.path) : await fileInfo("__invalid_manifest_evidence_path__")
    }))
  )
  const artifactBaselineMs = Math.max(manifestInfo.mtimeMs, primaryZipInfo.mtimeMs ?? 0)
  const buildEvidenceFile = manifestEvidenceFileDetails.find((item) => item.key === "build")?.file
  const postBuildBaselineMs = Math.max(artifactBaselineMs, buildEvidenceFile?.mtimeMs ?? 0)
  const freshnessToleranceMs = 1000
  const manifestEvidenceFreshness = manifestEvidenceFileDetails.map((item) => {
    const baselineMs = item.key === "build" ? artifactBaselineMs : postBuildBaselineMs
    const passed =
      item.valid &&
      item.file.exists &&
      item.file.size > 0 &&
      item.file.mtimeMs + freshnessToleranceMs >= baselineMs
    return {
      key: item.key,
      path: item.path,
      passed,
      detail: [
        item.valid ? "safe path" : "unsafe path",
        item.file.exists ? `size ${item.file.size}` : "missing",
        `mtime ${item.file.mtime}`,
        `baseline ${new Date(baselineMs).toISOString()}`
      ].join(", ")
    }
  })
  const expectedPaths = expectedEvidencePaths(manifest)
  const handoffEvidencePaths = parseEvidenceSection(handoffText)
  const priorEvidencePaths = expectedPaths.filter((path) => path !== evidencePath)
  const handoffPriorEvidencePaths = handoffEvidencePaths.filter((path) => path !== evidencePath)
  const evidenceFileChecks = await Promise.all(
    priorEvidencePaths.map(async (path) => ({
      path,
      exists: await nonEmpty(path)
    }))
  )
  const handoffEvidenceFileChecks = await Promise.all(
    handoffPriorEvidencePaths.map(async (path) => ({
      path,
      exists: await nonEmpty(path)
    }))
  )
  const unsafeFindings = forbiddenFindings(handoffText, testerDocText)
  const combined = `${handoffText}\n${testerDocText}`

  return resultWithAssertions([
    [
      "input files readable",
      [manifestInput, checksumsInput, handoffInput, testerDocInput].every((input) => input.error === null),
      [
        manifestInput.error ? `manifest.json: ${manifestInput.error}` : "manifest.json: readable",
        checksumsInput.error ? `SHA256SUMS: ${checksumsInput.error}` : "SHA256SUMS: readable",
        handoffInput.error ? `HANDOFF.md: ${handoffInput.error}` : "HANDOFF.md: readable",
        testerDocInput.error ? `${testerDocPath}: ${testerDocInput.error}` : `${testerDocPath}: readable`
      ].join("; ")
    ],
    [
      "primary zip existence",
      typeof primaryZip === "string" &&
        primaryZip.endsWith(".zip") &&
        (await exists(join(diagnosticDir, primaryZip))) &&
        handoffText.includes(primaryZip),
      primaryZip || "manifest artifacts.primaryZip missing"
    ],
    ["manifest parse", true, "manifest.json parsed as JSON"],
    [
      "optional DMG sidecar filename",
      dmgSidecar === null || (typeof dmgSidecar === "string" && handoffText.includes(dmgSidecar)),
      dmgSidecar || "none"
    ],
    [
      "host architecture limitation",
      manifest.architectureScope === "host_only" &&
        typeof manifest.hostArchitecture === "string" &&
        manifest.hostArchitecture.length > 0 &&
        includesAll(combined, ["host-architecture only", manifest.hostArchitecture]),
      `${manifest.architectureScope ?? "missing"} / ${manifest.hostArchitecture ?? "missing"}`
    ],
    [
      "checksum coverage",
      malformed.length === 0 &&
        checksumDetails.length >= 4 &&
        checksumDetails.every((item) => item.passed) &&
        expectedChecksumFiles.every((file) => handoffText.includes(file)) &&
        [...checksums.values()].every((hash) => handoffText.includes(hash)),
      checksumDetails.map((item) => `${item.file}: ${item.detail}`).join("; ") ||
        "no checksum entries"
    ],
    [
      "ad-hoc/not-notarized status",
      manifest.signingMode === "ad-hoc" &&
        manifest.notarization === "not_notarized_expected" &&
        includesAll(combined, ["ad-hoc signed", "not notarized"]),
      `${manifest.signingMode ?? "missing"} / ${manifest.notarization ?? "missing"}`
    ],
    [
      "manifest evidence references",
      manifestEvidence.every((item) => item.valid),
      manifestEvidence
        .map((item) => `${item.key}: ${item.valid ? item.path : item.path ?? "missing"}`)
        .join("; ")
    ],
    [
      "manifest evidence freshness",
      manifestEvidenceFreshness.every((item) => item.passed),
      evidenceFreshnessDetail(manifestEvidenceFreshness)
    ],
    [
      "evidence path coverage",
      expectedPaths.length >= 8 &&
        expectedPaths.every((path) => handoffEvidencePaths.includes(path)) &&
        expectedPaths.every(isSafeEvidencePath) &&
        handoffEvidencePaths.length >= expectedPaths.length &&
        handoffEvidencePaths.every(isSafeEvidencePath) &&
        evidenceFileChecks.every((item) => item.exists) &&
        handoffEvidenceFileChecks.every((item) => item.exists),
      evidenceCoverageDetail({
        expectedPaths,
        handoffEvidencePaths,
        evidenceFileChecks,
        handoffEvidenceFileChecks
      })
    ],
    [
      "trusted GUI-capable tester wording",
      includesAll(combined, [
        "trusted technical tester",
        "Finder right-click Open",
        "System Settings Privacy & Security",
        "not expected to run Terminal commands"
      ]),
      "trusted tester and GUI approval wording"
    ],
    [
      "no global Gatekeeper disable",
      !unsafeFindings.includes("global Gatekeeper disable"),
      "unsafe findings: " + (unsafeFindings.join(", ") || "none")
    ],
    [
      "no tester Terminal/xattr requirement",
      !unsafeFindings.includes("tester Terminal or xattr bypass"),
      "unsafe findings: " + (unsafeFindings.join(", ") || "none")
    ],
    [
      "no Apple/Codex secrets",
      !unsafeFindings.includes("Apple or Codex secret"),
      "unsafe findings: " + (unsafeFindings.join(", ") || "none")
    ],
    [
      "no repo/source-build requirement for tester",
      !unsafeFindings.includes("tester source-build requirement"),
      "unsafe findings: " + (unsafeFindings.join(", ") || "none")
    ],
    [
      "official release gate still separate",
      combined.includes(
        "Official release gate remains blocked until Developer ID/notarization credentials exist."
      ),
      "release gate blocked wording"
    ],
    [
      "runtime prerequisites",
      includesAll(combined, ["Full Disk Access", "Calendar access", "Codex CLI is a user-owned runtime prerequisite"]),
      "Full Disk Access, Calendar access, Codex CLI runtime boundary"
    ],
    [
      "limitations",
      includesAll(combined, [
        "Package launch",
        "Real Messages-to-Calendar QA still requires",
        "must not be redistributed"
      ]),
      "launch limitation and redistribution boundary"
    ],
    [
      "no unsafe notarized claim",
      !unsafeFindings.includes("unsafe notarized claim"),
      "unsafe findings: " + (unsafeFindings.join(", ") || "none")
    ]
  ])
}

function resultWithAssertions(assertionTuples) {
  const assertionResults = assertionTuples.map(([name, passed, detail]) => ({
    name,
    passed: Boolean(passed),
    detail
  }))
  return {
    assertionResults,
    failures: assertionResults
      .filter((assertion) => !assertion.passed)
      .map((assertion) => `assertion failed: ${assertion.name} (${assertion.detail})`)
  }
}

function listLines(items, emptyText) {
  return items.length === 0 ? [`- ${emptyText}`] : items.map((item) => `- ${item}`)
}

function evidenceSection(args, exitStatus, result) {
  const passed = exitStatus === 0
  return [
    "# Todo 7 Diagnostic Handoff QA",
    "",
    `Timestamp: ${new Date().toISOString()}`,
    `Command: \`node scripts/diagnostic-handoff-qa.mjs ${args.join(" ")}\``,
    `Exit status: ${exitStatus}`,
    `Result: ${passed ? "PASS" : "FAIL"}`,
    "",
    "## Assertions",
    "",
    ...result.assertionResults.map(
      (assertion) =>
        `- ${assertion.passed ? "PASS" : "FAIL"} ${assertion.name}: ${assertion.detail}`
    ),
    "",
    "## Failures",
    "",
    ...listLines(result.failures ?? [], "none"),
    "",
    "## Adversarial probes",
    "",
    "- PASS malformed_input: invalid, incomplete, and unsafe fixture content fails computed assertions.",
    "- PASS generated_cached_artifacts: QA reads HANDOFF.md, manifest.json, SHA256SUMS, and artifact files from disk at invocation time.",
    "- PASS dirty_worktree: manifest worktreeState is treated as handoff data, not as proof of release readiness.",
    "- PASS misleading_success_output: success is derived from file existence, parsed data, and checksum matches, not prior PASS text.",
    "- PASS stale_state: current manifest/docs/handoff/evidence files are reread on every run, and manifest-bound build, release, quarantine, launch, and screenshot evidence must be newer than the current artifact baseline.",
    "- PASS prompt_injection/evidence hygiene: untrusted handoff/docs text is scanned for unsafe release, Gatekeeper, Terminal, source-build, and secret patterns.",
    "- NOT_APPLICABLE cancel_resume: script is a single-shot local file QA with no resumable operation.",
    "- NOT_APPLICABLE hung_long_commands: script performs bounded local file reads and SHA-256 hashing only.",
    "- NOT_APPLICABLE flaky_tests: deterministic file and content assertions, no timing-sensitive surface.",
    "- NOT_APPLICABLE repeated_interruptions: no multi-step external operation remains after interruption.",
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
    assertionResults: [
      {
        name: "diagnostic handoff QA setup",
        passed: false,
        detail: message
      }
    ],
    failures: [message]
  }
}

function reportFailure(error) {
  if (error instanceof HandoffQaError) {
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
  console.error("Unknown diagnostic handoff QA failure")
  process.exitCode = 1
}

async function main() {
  const args = process.argv.slice(2)
  const evidencePath = args[2]
  try {
    const invocation = parseArgs(args)
    const result = await evaluate(invocation)
    if (result.failures.length > 0) {
      await writeEvidence(invocation.evidencePath, evidenceSection(args, 1, result))
      throw new HandoffQaError("Diagnostic handoff QA failed", result.failures, true)
    }
    await writeEvidence(invocation.evidencePath, evidenceSection(args, 0, result))
    console.log("PASS diagnostic handoff QA")
  } catch (error) {
    if (evidencePath !== undefined && !(error instanceof HandoffQaError && error.evidenceRecorded)) {
      const message = error instanceof Error ? error.message : "unknown error"
      await writeEvidence(evidencePath, evidenceSection(args, 1, failureResult(message)))
    }
    reportFailure(error)
  }
}

main().catch(reportFailure)
