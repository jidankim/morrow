#!/usr/bin/env node
import { spawn } from "node:child_process"
import { mkdir, rm, stat, writeFile } from "node:fs/promises"
import path from "node:path"
import process from "node:process"
import { setTimeout as delay } from "node:timers/promises"
import { chromium } from "playwright"

const baseUrl = "http://127.0.0.1:4174", binaryPath = "/Users/example/workspace/morrow/src-tauri/target/debug/morrow"
const appBundlePath = "/Applications/Morrow.app"
const requiredStates = [
  "discovery-permission-denied-binary", "discovery-permission-denied-appBundle",
  "discovery-unavailable-binary", "settings-privacy-binary", "settings-privacy-appBundle"
]
const viewportWidths = [375, 768, 1280]
const screenshotPrefix = "visual-full-disk-access-recovery"
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
    throw new Error("Usage: node scripts/visual-full-disk-access-recovery-qa.mjs <evidence-dir>")
  }

  const evidenceDir = path.resolve(process.cwd(), evidenceArg)
  await mkdir(evidenceDir, { recursive: true })
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
          const checks = await verifyRecoveryState(page, state, width)
          await page.screenshot({ path: screenshotPath, fullPage: true })
          results.push({
            state,
            width,
            screenshotPath: path.relative(process.cwd(), screenshotPath),
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

  const reportPath = path.join(evidenceDir, `${screenshotPrefix}-report.json`)
  await writeReport(reportPath, {
    command: "node scripts/visual-full-disk-access-recovery-qa.mjs .omo/evidence/full-disk-access-guided-recovery/visual",
    baseUrl,
    requiredStates, viewportWidths,
    startedAt: new Date(startedAtMs).toISOString(), completedAt: new Date().toISOString(),
    devServerPid: server.pid,
    cleanup: { childExited: server.exitCode !== null, exitCode: server.exitCode, signal: server.signalCode },
    results
  })
  console.log(
    `visual Full Disk Access recovery QA passed: ${results.length} screenshots, report ${reportPath}`
  )
}

function startDevServer(serverLog) {
  const server = spawn("npm", ["run", "dev", "--", "--port", "4174", "--strictPort"], {
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
  url.searchParams.set("visualQa", "full-disk-access-recovery")
  url.searchParams.set("state", state)
  return url.toString()
}

async function verifyRecoveryState(page, state, width) {
  await assertNoHorizontalOverflow(page, state, width)
  const text = await page.locator("body").innerText()
  assertContains(text, "Open Full Disk Access", state, width)
  assertRuntimeCopy(text, state, width)
  const openButton = page.getByRole("button", { name: "Open Full Disk Access" })
  const openButtonBox = await assertVisibleBox(openButton, state, width, "Open Full Disk Access")
  await assertFocusable(openButton, state, width, "Open Full Disk Access")
  const copyButtonCheck = await assertCopyButton(page, state, width)
  const visibleBoxes = await assertVisibleTextAndControls(page, state, width)
  return { openButtonBox, copyButtonCheck, visibleBoxes }
}

async function assertNoHorizontalOverflow(page, state, width) {
  const overflow = await page.evaluate(() => ({
    scrollWidth: document.documentElement.scrollWidth,
    innerWidth: window.innerWidth
  }))
  if (overflow.scrollWidth > overflow.innerWidth) {
    throw new Error(
      `${state}@${width}: horizontal overflow ${overflow.scrollWidth}px > ${overflow.innerWidth}px`
    )
  }
  return overflow
}

function assertRuntimeCopy(text, state, width) {
  if (!state.includes("appBundle")) {
    assertContains(text, "Cmd+Shift+G", state, width)
    assertContains(text, binaryPath, state, width)
    return
  }
  assertContains(text, appBundlePath, state, width)
  if (text.includes("target/debug")) {
    throw new Error(`${state}@${width}: appBundle state rendered target/debug copy`)
  }
}

function assertContains(text, expected, state, width) {
  if (!text.includes(expected)) {
    throw new Error(`${state}@${width}: expected rendered copy missing ${JSON.stringify(expected)}`)
  }
}

async function assertCopyButton(page, state, width) {
  const copyButton = page.getByRole("button", { name: "Copy Morrow path" })
  const count = await copyButton.count()
  if (count !== 1) {
    throw new Error(`${state}@${width}: expected one copy button, found ${count}`)
  }
  const box = await assertVisibleBox(copyButton, state, width, "Copy Morrow path")
  await assertFocusable(copyButton, state, width, "Copy Morrow path")
  return { expected: true, visible: true, box }
}

async function assertVisibleBox(locator, state, width, label) {
  const box = await locator.boundingBox()
  if (box === null || box.width <= 0 || box.height <= 0) {
    throw new Error(`${state}@${width}: ${label} has no positive visible box`)
  }
  return box
}

async function assertFocusable(locator, state, width, label) {
  await locator.focus()
  const focused = await locator.evaluate((element) => document.activeElement === element)
  if (!focused) {
    throw new Error(`${state}@${width}: ${label} is not focusable`)
  }
}

async function assertVisibleTextAndControls(page, state, width) {
  const problems = await page.evaluate(() => {
    const root = document.querySelector("[data-visual-qa-state]")
    if (root === null) {
      return ["visual QA root is missing"]
    }
    const elements = Array.from(
      root.querySelectorAll("h1,h2,h3,p,strong,code,button,input,select")
    )
    return elements.flatMap((element) => {
      const label = element.textContent?.trim() || element.getAttribute("aria-label") || element.tagName
      const rect = element.getBoundingClientRect()
      const style = window.getComputedStyle(element)
      const hidden = style.display === "none" || style.visibility === "hidden"
      if (hidden || rect.width <= 0 || rect.height <= 0) {
        return [`${label} has no positive visible dimensions`]
      }
      return []
    })
  })
  if (problems.length > 0) {
    throw new Error(`${state}@${width}: visible box problems:\n${problems.join("\n")}`)
  }
  return { checkedBoxes: await page.locator("[data-visual-qa-state] h1,h2,h3,p,strong,code,button,input,select").count() }
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

async function writeReport(reportPath, report) {
  await mkdir(path.dirname(reportPath), { recursive: true })
  await writeFile(reportPath, `${JSON.stringify(report, null, 2)}\n`)
}

main().catch((error) => {
  if (error instanceof Error) {
    console.error(error.message)
  } else {
    console.error(error)
  }
  process.exitCode = 1
})
