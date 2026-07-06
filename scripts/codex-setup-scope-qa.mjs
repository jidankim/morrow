#!/usr/bin/env node

import { mkdir, readFile, writeFile } from "node:fs/promises"
import { existsSync } from "node:fs"
import { dirname, extname } from "node:path"
import { spawnSync } from "node:child_process"

const usage = "Usage: node scripts/codex-setup-scope-qa.mjs <evidence path> [file ...]"
const defaultGitStatusTimeoutMs = 5000

const forbiddenRules = [
  ["curl_pipe_shell", /curl\s+https:\/\/chatgpt[.]com\/codex\/install[.]sh\s*[|]\s*(?:sh|bash)\b/i],
  ["shell_command_interpolation", /\bsh\s+-c\b|Command::new\(\"\/bin\/sh\"\).*arg\(\"-c\"\)/i],
  ["silent_auto_install_claim", /\bauto-installs Codex\b|\bauto install Codex\b/i],
  ["bundled_cli_claim", /\bbundles Codex CLI\b/i],
  ["credential_env_var", /\bCODEX_ACCESS_TOKEN\b/i],
  ["codex_auth_file", /\bauth[.]json\b|[~]\/[.]codex\b/i],
  ["openai_secret_token", /\bsk-[A-Za-z0-9]{8,}\b/],
  ["copy_login_command_ui", /\bCopy login command\b|\bCODEX_LOGIN_COMMAND\b|navigator[.]clipboard/]
]

class ScopeQaError extends Error {
  constructor(message, failures, evidenceRecorded = false) {
    super(message)
    this.name = "ScopeQaError"
    this.failures = failures
    this.evidenceRecorded = evidenceRecorded
  }
}

function gitStatusTimeoutMs() {
  const rawValue = process.env.CODEX_SETUP_SCOPE_QA_GIT_STATUS_TIMEOUT_MS
  if (rawValue === undefined) {
    return defaultGitStatusTimeoutMs
  }
  const parsedValue = Number.parseInt(rawValue, 10)
  if (!Number.isSafeInteger(parsedValue) || parsedValue <= 0 || String(parsedValue) !== rawValue) {
    throw new ScopeQaError(
      "Invalid git status timeout",
      ["CODEX_SETUP_SCOPE_QA_GIT_STATUS_TIMEOUT_MS must be a positive integer millisecond value"]
    )
  }
  return parsedValue
}

function parseArgs(args) {
  const [evidencePath, ...files] = args
  if (evidencePath === undefined) {
    throw new ScopeQaError(usage, ["expected <evidence path>"])
  }
  return { evidencePath, files }
}

function changedFiles() {
  const timeout = gitStatusTimeoutMs()
  const result = spawnSync("git", ["status", "--porcelain=v1"], {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
    timeout,
    killSignal: "SIGKILL"
  })
  if (result.error !== undefined) {
    if (result.error.code === "ETIMEDOUT") {
      throw new ScopeQaError(
        `git status timed out after ${timeout}ms`,
        ["git status exceeded the configured timeout before scope scan could identify changed files"]
      )
    }
    throw new ScopeQaError(
      "git status failed before scope scan",
      ["git status exited non-zero before scope scan could identify changed files"]
    )
  }
  if (result.status !== 0) {
    throw new ScopeQaError(
      "git status failed before scope scan",
      ["git status exited non-zero before scope scan could identify changed files"]
    )
  }
  const output = result.stdout
  return output
    .split("\n")
    .filter((line) => line.trim().length > 0)
    .map((line) => line.slice(3).replace(/^"|"$/g, ""))
    .filter((path) => path.length > 0)
}

function isScannable(path) {
  if (path === "README.md") {
    return true
  }
  if (path.startsWith("docs/") || path.startsWith("src/") || path.startsWith("src-tauri/")) {
    return true
  }
  if (path.startsWith("scripts/")) {
    return [".mjs", ".js", ".sh", ".ts", ".rs"].includes(extname(path))
  }
  return path.startsWith(".omo/evidence/")
}

function isInsideArray(lines, index, declarationPattern) {
  let sawDeclaration = false
  for (let cursor = index; cursor >= 0; cursor -= 1) {
    const line = lines[cursor]
    if (declarationPattern.test(line)) {
      sawDeclaration = true
      break
    }
    if (line.includes("]")) {
      break
    }
  }
  if (!sawDeclaration) {
    return false
  }
  for (let cursor = index; cursor < lines.length; cursor += 1) {
    if (lines[cursor].includes("]")) {
      return true
    }
  }
  return false
}

function isAllowedDocsQaAssertionContext(path, lines, index) {
  if (path === "scripts/codex-setup-docs-qa.mjs") {
    return isInsideArray(lines, index, /^const forbiddenPhrases = \[/)
  }
  if (path === "scripts/beta-release-docs-qa.mjs") {
    const line = lines[index]
    return (
      isInsideArray(lines, index, /^const forbiddenClaims = \[/) ||
      (line.includes("[\"paste tokens\"") && line.includes("].includes(claim)"))
    )
  }
  return false
}

function isAllowedEvidenceAssertionContext(line) {
  if (/\b[A-Z_]*TOKEN\b\s*=|\bsk-[A-Za-z0-9]{8,}\b/.test(line)) {
    return false
  }
  return /\b(?:assertion context|fixture description|forbidden (?:phrase|pattern|rule) (?:list|assertion)|expected sanitized failure|sanitized (?:failure|rule id))\b/i.test(line)
}

function isAllowedMatch(path, lines, index, ruleId) {
  const line = lines[index]
  const lowerLine = line.toLowerCase()
  if (path === "scripts/codex-setup-scope-qa.mjs") {
    return true
  }
  if (isAllowedDocsQaAssertionContext(path, lines, index)) {
    return true
  }
  if (path.includes(".test.") || path.includes("/tests/")) {
    if (["credential_env_var", "codex_auth_file", "openai_secret_token"].includes(ruleId)) {
      return true
    }
    return /redact|fixture|fake|secret|raw|expect|assert|leak|forbidden/i.test(line)
  }
  if (path.startsWith(".omo/evidence/")) {
    return isAllowedEvidenceAssertionContext(line)
  }
  return ruleId === "codex_auth_file" && /does not read|does not store|do not read|must not/i.test(lowerLine)
}

function evaluateFile(path, text) {
  const failures = []
  const lines = text.split(/\r?\n/)
  lines.forEach((line, index) => {
    forbiddenRules.forEach(([ruleId, pattern]) => {
      if (pattern.test(line) && !isAllowedMatch(path, lines, index, ruleId)) {
        failures.push({ path, line: index + 1, ruleId })
      }
    })
  })
  return failures
}

function evidenceSection(args, exitStatus, scannedFiles, failures) {
  const status = exitStatus === 0 ? "PASS" : "FAIL"
  const failureRows = failures.length === 0
    ? ["- none"]
    : failures.map((failure) => `- ${failure.path}:${failure.line} ${failure.ruleId}`)
  return [
    "",
    `## Codex Setup Scope QA - ${new Date().toISOString()}`,
    "",
    "Scenario: changed source/docs/evidence exclude credential and silent-install scope creep",
    `Invocation: node scripts/codex-setup-scope-qa.mjs ${args.join(" ")}`,
    `Binary observable: process exit status ${exitStatus}`,
    `Result: ${status}`,
    "",
    "Scanned files:",
    ...(scannedFiles.length === 0 ? ["- none"] : scannedFiles.map((path) => `- ${path}`)),
    "",
    "Sanitized failures:",
    ...failureRows,
    ""
  ].join("\n")
}

async function appendEvidence(evidencePath, section) {
  await mkdir(dirname(evidencePath), { recursive: true })
  await writeFile(evidencePath, `${section}\n`, { flag: "a" })
}

async function run(args) {
  const { evidencePath, files } = parseArgs(args)
  const candidateFiles = files.length > 0 ? files : [...changedFiles(), evidencePath]
  const scopedFiles = files.length > 0 ? candidateFiles : candidateFiles.filter(isScannable)
  const scannedFiles = [...new Set(scopedFiles)].filter((path) => existsSync(path)).sort()
  const failures = []
  for (const path of scannedFiles) {
    const text = await readFile(path, "utf8")
    failures.push(...evaluateFile(path, text))
  }
  if (failures.length > 0) {
    await appendEvidence(evidencePath, evidenceSection(args, 1, scannedFiles, failures))
    throw new ScopeQaError("Codex setup scope QA failed", failures, true)
  }
  await appendEvidence(evidencePath, evidenceSection(args, 0, scannedFiles, failures))
  console.log("PASS codex setup scope QA")
}

function reportFailure(error) {
  if (error instanceof ScopeQaError) {
    console.error(error.message)
    for (const failure of error.failures) {
      if (typeof failure === "string") {
        console.error(`- ${failure}`)
      } else {
        console.error(`- ${failure.path}:${failure.line} ${failure.ruleId}`)
      }
    }
    process.exitCode = 1
    return
  }
  if (error instanceof Error) {
    console.error(error.message)
    process.exitCode = 1
    return
  }
  console.error("Unknown Codex setup scope QA failure")
  process.exitCode = 1
}

run(process.argv.slice(2)).catch(reportFailure)
