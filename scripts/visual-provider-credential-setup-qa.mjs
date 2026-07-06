#!/usr/bin/env node
import { mkdir, rm } from "node:fs/promises"
import path from "node:path"
import process from "node:process"
import { chromium } from "playwright"
import { assertFocusable, assertNoHorizontalOverflow, assertVisibleBox, cleanupReceipt, cleanupServer, errorMessage, startDevServer, verifyFreshScreenshot, waitForServer, writeReport } from "./visual-qa-support.mjs"

const port = "4175"
const baseUrl = `http://127.0.0.1:${port}`
const requiredStates = [
  "missing-cli",
  "install-confirming",
  "installing",
  "not-logged-in",
  "login-polling",
  "ready",
  "failed"
]
const expectedStateCopy = {
  "missing-cli": {
    buttons: [{ label: "Install Codex CLI", enabled: true }],
    status: "Codex CLI is not installed."
  },
  "install-confirming": {
    buttons: [
      { label: "Install", enabled: true },
      { label: "Cancel", enabled: true }
    ],
    status: "Install Codex CLI now?"
  },
  installing: {
    buttons: [{ label: "Installing Codex CLI", enabled: false }],
    status: "Installing Codex CLI..."
  },
  "not-logged-in": {
    buttons: [{ label: "Start Codex login", enabled: true }],
    status: "Codex CLI is installed. ChatGPT login is required."
  },
  "login-polling": {
    buttons: [{ label: "Waiting for browser login", enabled: false }],
    status: "Waiting for browser login to complete..."
  },
  ready: {
    buttons: [{ label: "Refresh readiness", enabled: true }],
    status: "Codex provider is ready."
  },
  failed: {
    buttons: [{ label: "Retry setup", enabled: true }],
    status: "Codex provider setup could not be completed."
  }
}
const viewportWidths = [375, 768, 1280]
const screenshotPrefix = "visual-provider-credential-setup"
let activeServer

for (const signal of ["SIGINT", "SIGTERM"]) {
  process.once(signal, () => {
    void cleanupServer(activeServer).finally(() => {
      process.exit(signal === "SIGINT" ? 130 : 143)
    })
  })
}

async function main() {
  const options = parseArgs(process.argv.slice(2))
  const evidenceDir = path.resolve(process.cwd(), options.evidenceDir)
  const startedAtMs = Date.now()
  await mkdir(evidenceDir, { recursive: true })
  const reportPath = path.join(evidenceDir, `${screenshotPrefix}-report.json`)
  const serverLog = []
  const report = {
    command: `node ${process.argv.slice(1).join(" ")}`,
    baseUrl,
    requiredStates: options.states,
    viewportWidths,
    startedAt: new Date(startedAtMs).toISOString(),
    completedAt: undefined,
    devServerPid: undefined,
    cleanup: undefined,
    results: [],
    errors: []
  }

  try {
    assertSupportedStates(options.states)
    const server = startDevServer({ port, serverLog })
    activeServer = server
    report.devServerPid = server.pid
    let browser
    try {
      await waitForServer({ baseUrl, server, serverLog })
      browser = await chromium.launch()
      for (const state of options.states) {
        for (const width of viewportWidths) {
          const screenshotPath = screenshotFilePath(evidenceDir, state, width)
          await rm(screenshotPath, { force: true })
          const page = await browser.newPage({ viewport: { width, height: 900 } })
          try {
            await page.goto(stateUrl(state), { waitUntil: "networkidle" })
            await page.waitForSelector(`[data-visual-qa-state="${state}"]`, { timeout: 5_000 })
            const checks = await verifyProviderState(page, state, width)
            await page.screenshot({ path: screenshotPath, fullPage: true })
            report.results.push({
              state,
              width,
              screenshotPath: path.relative(process.cwd(), screenshotPath),
              screenshot: await verifyFreshScreenshot({ screenshotPath, startedAtMs }),
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
      report.cleanup = await cleanupReceipt({ baseUrl, server })
    }
  } catch (error) {
    report.errors.push(errorMessage(error))
    process.exitCode = 1
  } finally {
    report.completedAt = new Date().toISOString()
    await writeReport(reportPath, report)
  }

  if (report.errors.length > 0) {
    console.error(`visual provider credential setup QA failed: ${reportPath}`)
    for (const error of report.errors) {
      console.error(error)
    }
    return
  }
  console.log(
    `visual provider credential setup QA passed: ${report.results.length} screenshots, report ${reportPath}`
  )
}

function parseArgs(args) {
  const evidenceDir = args[0]
  if (evidenceDir === undefined) {
    throw new Error("Usage: node scripts/visual-provider-credential-setup-qa.mjs <evidence-dir> [--require-state <state>]")
  }
  const requireStateIndex = args.indexOf("--require-state")
  if (requireStateIndex === -1) {
    return { evidenceDir, states: requiredStates }
  }
  const requiredState = args[requireStateIndex + 1]
  if (requiredState === undefined) {
    throw new Error("--require-state requires a state name")
  }
  return { evidenceDir, states: [requiredState] }
}

function assertSupportedStates(states) {
  for (const state of states) {
    if (!requiredStates.includes(state)) {
      throw new Error(`Unsupported provider credential setup visual state: ${state}`)
    }
  }
}

function stateUrl(state) {
  const url = new URL(baseUrl)
  url.searchParams.set("visualQa", "provider-credential-setup")
  url.searchParams.set("state", state)
  return url.toString()
}

async function verifyProviderState(page, state, width) {
  await assertNoHorizontalOverflow({ page, state, width })
  const section = page.locator("section[aria-labelledby='provider-credential-heading']")
  const sectionBox = await assertVisibleBox({ label: "provider credential section", locator: section, state, width })
  const statusText = await section.locator(".inline-status").innerText()
  const expected = expectedStateCopy[state]
  if (statusText !== expected.status) {
    throw new Error(`${state}@${width}: expected status ${JSON.stringify(expected.status)}, found ${JSON.stringify(statusText)}`)
  }
  const buttons = section.getByRole("button")
  const labels = await buttons.allInnerTexts()
  const trimmedLabels = labels.map((label) => label.trim())
  const expectedLabels = expected.buttons.map((button) => button.label)
  assertExactLabels(trimmedLabels, expectedLabels, state, width)
  const buttonChecks = []
  for (let index = 0; index < expected.buttons.length; index += 1) {
    const expectedButton = expected.buttons[index]
    const button = buttons.nth(index)
    const box = await assertVisibleBox({ label: expectedButton.label, locator: button, state, width })
    const disabled = await button.isDisabled()
    if (disabled === expectedButton.enabled) {
      throw new Error(`${state}@${width}: ${expectedButton.label} enabled=${expectedButton.enabled} but disabled=${disabled}`)
    }
    if (expectedButton.enabled) {
      await assertFocusable({ label: expectedButton.label, locator: button, state, width })
    }
    buttonChecks.push({ label: expectedButton.label, enabled: expectedButton.enabled, box })
  }
  const visibleBoxes = await assertVisibleTextAndControls(section, state, width)
  return {
    sectionBox,
    statusText,
    buttonChecks,
    visibleBoxes,
    nonblankDom: await assertNonblankDom(page, state, width)
  }
}

function assertExactLabels(actual, expected, state, width) {
  if (actual.length !== expected.length || actual.some((label, index) => label !== expected[index])) {
    throw new Error(`${state}@${width}: expected buttons ${JSON.stringify(expected)}, found ${JSON.stringify(actual)}`)
  }
}

async function assertVisibleTextAndControls(section, state, width) {
  const problems = await section.evaluate((root) => {
    const elements = Array.from(root.querySelectorAll("h1,h2,h3,p,strong,code,button,input,select"))
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
  return { checkedBoxes: await section.locator("h1,h2,h3,p,strong,code,button,input,select").count() }
}

async function assertNonblankDom(page, state, width) {
  const nonblank = await page.evaluate(() => {
    const root = document.querySelector("[data-visual-qa-state]")
    const rect = root?.getBoundingClientRect()
    return {
      textLength: document.body.innerText.trim().length,
      rootArea: rect === undefined ? 0 : Math.round(rect.width * rect.height),
      bodyBackground: window.getComputedStyle(document.body).backgroundColor
    }
  })
  if (nonblank.textLength <= 0 || nonblank.rootArea <= 0) {
    throw new Error(`${state}@${width}: rendered page is blank`)
  }
  return nonblank
}

function screenshotFilePath(evidenceDir, state, width) {
  return path.join(evidenceDir, `${screenshotPrefix}-${state}-${width}.png`)
}

main().catch((error) => {
  console.error(errorMessage(error))
  process.exitCode = 1
})
