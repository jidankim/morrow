#!/usr/bin/env node
import { spawn } from "node:child_process"
import { access, mkdir, rm, stat, writeFile } from "node:fs/promises"
import path from "node:path"
import process from "node:process"
import { setTimeout as delay } from "node:timers/promises"
import { chromium } from "playwright"
import { verifyVisualState } from "./visual-chat-discovery-checks.mjs"

const baseUrl = "http://127.0.0.1:4173"
const requiredStates = [
  "loading",
  "unverified",
  "permissionDenied",
  "unavailable",
  "empty",
  "ready-multiple",
  "ready-selected",
  "stale-selection"
]
const viewportWidths = [375, 768, 1280]
const screenshotPrefix = "visual-native-chat-discovery-ui-friendly"
let activeServer
let cleanupStarted = false

for (const signal of ["SIGINT", "SIGTERM"]) {
  process.once(signal, () => {
    void cleanupServer(activeServer).finally(() => {
      process.exit(signal === "SIGINT" ? 130 : 143)
    })
  })
}

async function main() {
  const evidenceArg = process.argv[2]
  if (evidenceArg === undefined) {
    throw new Error("Usage: node scripts/visual-chat-discovery-qa.mjs <evidence-dir>")
  }

  const evidenceDir = path.resolve(process.cwd(), evidenceArg)
  await access(evidenceDir)
  const startedAtMs = Date.now()
  const serverLog = []
  const server = startDevServer(serverLog)
  activeServer = server
  let browser
  const results = []

  try {
    await waitForServer(server, serverLog)
    browser = await chromium.launch()
    for (const state of requiredStates) {
      for (const width of viewportWidths) {
        const screenshotPath = screenshotFilePath(evidenceDir, state, width)
        await rm(screenshotPath, { force: true })
        const page = await browser.newPage({ viewport: { width, height: 900 } })
        try {
          await page.goto(stateUrl(state), { waitUntil: "networkidle" })
          await page.waitForSelector(`[data-visual-qa-state="${state}"]`, { timeout: 5_000 })
          const checks = await verifyVisualState(page, state, width)
          await page.screenshot({ path: screenshotPath, fullPage: true })
          results.push({
            state,
            width,
            screenshotPath,
            screenshot: await verifyFreshScreenshot(screenshotPath, startedAtMs),
            checks
          })
        } finally {
          await page.close()
        }
      }
    }
  } finally {
    if (browser !== undefined) {
      await browser.close()
    }
    await cleanupServer(server)
  }

  await writeReport(evidenceDir, {
    command: "node scripts/visual-chat-discovery-qa.mjs .omo/evidence",
    baseUrl,
    requiredStates,
    viewportWidths,
    startedAt: new Date(startedAtMs).toISOString(),
    completedAt: new Date().toISOString(),
    devServerPid: server.pid,
    cleanup: { childExited: server.exitCode !== null, exitCode: server.exitCode, signal: server.signalCode },
    results
  })
  console.log(
    `visual chat discovery QA passed: ${results.length} screenshots, ${requiredStates.length} states, ${viewportWidths.length} widths`
  )
}

function startDevServer(serverLog) {
  const server = spawn("npm", ["run", "dev", "--", "--port", "4173", "--strictPort"], {
    cwd: process.cwd(),
    env: { ...process.env, VITE_DISABLE_REACT_DEVTOOLS: "1" },
    stdio: ["ignore", "pipe", "pipe"]
  })
  server.stdout.on("data", (chunk) => serverLog.push(String(chunk)))
  server.stderr.on("data", (chunk) => serverLog.push(String(chunk)))
  return server
}

async function waitForServer(server, serverLog) {
  const deadline = Date.now() + 20_000
  while (Date.now() < deadline) {
    if (server.exitCode !== null) {
      throw new Error(`Vite exited before it became ready:\n${serverLog.join("")}`)
    }
    if (await serverResponds()) {
      return
    }
    await delay(250)
  }
  throw new Error(`Timed out waiting for ${baseUrl}:\n${serverLog.join("")}`)
}

async function serverResponds() {
  try {
    const controller = new AbortController()
    const timeout = setTimeout(() => controller.abort(), 500)
    const response = await fetch(baseUrl, { signal: controller.signal })
    clearTimeout(timeout)
    return response.ok
  } catch {
    return false
  }
}

async function cleanupServer(server) {
  if (server === undefined || cleanupStarted) {
    return
  }
  cleanupStarted = true
  if (server.exitCode !== null) {
    return
  }
  server.kill("SIGTERM")
  const exited = new Promise((resolve) => server.once("exit", () => resolve(true)))
  if (await Promise.race([exited, delay(5_000).then(() => false)])) {
    return
  }
  if (server.exitCode === null) {
    server.kill("SIGKILL")
    await new Promise((resolve) => server.once("exit", () => resolve(true)))
  }
}

function stateUrl(state) {
  const url = new URL(baseUrl)
  url.searchParams.set("visualQa", "chat-discovery")
  url.searchParams.set("state", state)
  return url.toString()
}

function screenshotFilePath(evidenceDir, state, width) {
  return path.join(evidenceDir, `${screenshotPrefix}-${state}-${width}.png`)
}

async function verifyFreshScreenshot(screenshotPath, startedAtMs) {
  const info = await stat(screenshotPath)
  if (info.size <= 0 || info.mtimeMs < startedAtMs - 1_000) {
    throw new Error(`Screenshot was not regenerated during this run: ${screenshotPath}`)
  }
  return { bytes: info.size, mtime: info.mtime.toISOString() }
}

async function writeReport(evidenceDir, report) {
  await mkdir(evidenceDir, { recursive: true })
  await writeFile(
    path.join(evidenceDir, `${screenshotPrefix}-report.json`),
    `${JSON.stringify(report, null, 2)}\n`
  )
}

main().catch((error) => {
  if (error instanceof Error) {
    console.error(error.message)
  } else {
    console.error(error)
  }
  process.exitCode = 1
})
