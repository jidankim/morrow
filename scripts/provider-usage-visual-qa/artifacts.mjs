import { mkdir, writeFile } from "node:fs/promises"
import os from "node:os"
import path from "node:path"
import { bannedPatternRules, bannedStrings, workspaceRoot } from "./constants.mjs"

export async function writeJson(evidenceDir, fileName, value) {
  await mkdir(evidenceDir, { recursive: true })
  await writeFile(path.join(evidenceDir, fileName), `${JSON.stringify(value, null, 2)}\n`)
}

export function sanitizeText(value) {
  return value.split(workspaceRoot).join("<workspace>").split(os.homedir()).join("<home>")
}

export function relativePath(filePath) {
  return path.relative(workspaceRoot, filePath)
}

export function stateUrl(baseUrl, state) {
  const url = new URL(baseUrl)
  url.searchParams.set("visualQa", "provider-usage")
  url.searchParams.set("state", state)
  url.hash = "usage"
  return url.toString()
}

export function scanRetainedText(value) {
  const text = JSON.stringify(value, null, 2)
  const matches = bannedTextMatches(text)
  if (matches.length > 0) {
    throw new Error(`Privacy smoke failed: retained metadata/log text contains ${matches.join(", ")}`)
  }
  return {
    passed: true,
    scanned: ["report metadata", "screenshot metadata", "dev server log", "cleanup receipt"],
    bannedPatternCount: bannedStrings.length + bannedPatternRules.length
  }
}

export function scanDashboardVisibleText(value) {
  const matches = bannedTextMatches(value)
  if (matches.length > 0) {
    throw new Error(`Privacy smoke failed: dashboard visible DOM text contains ${matches.join(", ")}`)
  }
  return {
    passed: true,
    scanned: ["dashboard visible DOM text"],
    bannedPatternCount: bannedStrings.length + bannedPatternRules.length
  }
}

function redactedBannedString(value) {
  if (value === workspaceRoot) {
    return "<workspace-absolute-path>"
  }
  if (value === os.homedir()) {
    return "<home-absolute-path>"
  }
  return value
}

function bannedTextMatches(text) {
  const stringMatches = bannedStrings
    .filter((banned) => banned.length > 0 && text.includes(banned))
    .map((banned) => redactedBannedString(banned))
  const patternMatches = bannedPatternRules
    .filter((rule) => rule.pattern.test(text))
    .map((rule) => rule.label)
  return [...stringMatches, ...patternMatches]
}
