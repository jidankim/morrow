#!/usr/bin/env node
import { mkdir } from "node:fs/promises"
import path from "node:path"
import process from "node:process"
import { chromium } from "playwright"
import { relativePath, sanitizeText, scanRetainedText, writeJson } from "./provider-usage-visual-qa/artifacts.mjs"
import { captureState, assertFreshStateCoverage } from "./provider-usage-visual-qa/browser-checks.mjs"
import { cleanupResources, initializeCleanup, registerCleanupHandlers } from "./provider-usage-visual-qa/cleanup.mjs"
import { evidenceRoot, requiredStates, viewportWidths, workspaceRoot } from "./provider-usage-visual-qa/constants.mjs"
import { findAvailablePort, startDevServer, waitForServer } from "./provider-usage-visual-qa/dev-server.mjs"

registerCleanupHandlers()

async function main() {
  const evidenceDir = parseEvidenceDir(process.argv.slice(2))
  await mkdir(evidenceDir, { recursive: true })

  const startedAtMs = Date.now()
  const serverLog = []
  const cleanupContext = initializeCleanup(evidenceDir)
  const results = []
  let baseUrl = "not-started"

  try {
    const port = await findAvailablePort()
    baseUrl = `http://127.0.0.1:${port}`
    const server = startDevServer(port, serverLog)
    cleanupContext.server = server
    cleanupContext.cleanup.devServerPid = server.pid
    await waitForServer(baseUrl, server, serverLog)

    const browser = await chromium.launch()
    cleanupContext.browser = browser

    for (const state of requiredStates) {
      for (const width of viewportWidths) {
        const result = await captureState(browser, evidenceDir, baseUrl, state, width, startedAtMs)
        results.push(result)
      }
    }

    const staleState = assertFreshStateCoverage(results)
    const cleanup = await cleanupResources("normal")
    const report = {
      command: "node scripts/visual-provider-usage-qa.mjs --evidence-dir <dir>",
      baseUrl,
      requiredStates,
      viewportWidths,
      startedAt: new Date(startedAtMs).toISOString(),
      completedAt: new Date().toISOString(),
      results,
      adversarial: { staleState },
      cleanup,
      privacySmoke: scanRetainedText({
        cleanup,
        results,
        serverLog: sanitizeText(serverLog.join(""))
      })
    }
    await writeJson(evidenceDir, "report.json", report)
    await writeJson(evidenceDir, "privacy-scan.json", report.privacySmoke)
    console.log(`provider usage visual QA passed: ${results.length} screenshots, report ${relativePath(path.join(evidenceDir, "report.json"))}`)
  } catch (error) {
    const cleanup = await cleanupResources("error")
    await writeJson(evidenceDir, "failure-report.json", {
      command: "node scripts/visual-provider-usage-qa.mjs --evidence-dir <dir>",
      error: error instanceof Error ? sanitizeText(error.message) : sanitizeText(String(error)),
      cleanup,
      serverLog: sanitizeText(serverLog.join(""))
    })
    throw error
  }
}

function parseEvidenceDir(args) {
  if (args.length !== 2 || args[0] !== "--evidence-dir" || args[1] === undefined || args[1].length === 0) {
    throw new Error("Usage: node scripts/visual-provider-usage-qa.mjs --evidence-dir <dir>")
  }
  const evidenceDir = path.resolve(workspaceRoot, args[1])
  const allowedRoot = path.join(workspaceRoot, evidenceRoot)
  const relativeToRoot = path.relative(allowedRoot, evidenceDir)
  if (relativeToRoot === "" || (!relativeToRoot.startsWith("..") && !path.isAbsolute(relativeToRoot))) {
    return evidenceDir
  }
  throw new Error(`Evidence directory must stay under ${evidenceRoot}`)
}

main().catch((error) => {
  if (error instanceof Error) {
    console.error(sanitizeText(error.message))
  } else {
    console.error(sanitizeText(String(error)))
  }
  process.exitCode = 1
})
