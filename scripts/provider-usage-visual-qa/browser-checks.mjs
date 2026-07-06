import { createHash } from "node:crypto"
import { rm, stat } from "node:fs/promises"
import path from "node:path"
import { relativePath, scanDashboardVisibleText, stateUrl } from "./artifacts.mjs"
import { requiredStates, screenshotPrefix, viewportWidths } from "./constants.mjs"
import { inspectPng } from "./png.mjs"

export async function captureState(browser, evidenceDir, baseUrl, state, width, startedAtMs) {
  const page = await browser.newPage({ viewport: { width, height: 900 } })
  const screenshotName = `${screenshotPrefix}-${state}-${width}.png`
  const screenshotPath = path.join(evidenceDir, screenshotName)
  await rm(screenshotPath, { force: true })

  try {
    await page.goto(stateUrl(baseUrl, state), { waitUntil: "networkidle" })
    await page.waitForSelector(`[data-visual-qa-state="${state}"]`, { timeout: 5_000 })
    const checks = await verifyProviderUsagePage(page, state, width)
    const screenshot = await page.screenshot({ path: screenshotPath, fullPage: true })
    const screenshotInfo = await verifyFreshScreenshot(screenshotPath, screenshot, startedAtMs)
    return {
      state,
      width,
      screenshot: { file: relativePath(screenshotPath), ...screenshotInfo },
      checks
    }
  } finally {
    await page.close()
  }
}

export function assertFreshStateCoverage(results) {
  const byWidth = new Map()
  for (const result of results) {
    const statesForWidth = byWidth.get(result.width) ?? new Map()
    statesForWidth.set(result.state, result.screenshot.sha256)
    byWidth.set(result.width, statesForWidth)
  }

  const comparisons = []
  for (const width of viewportWidths) {
    const statesForWidth = byWidth.get(width)
    if (statesForWidth === undefined || statesForWidth.size !== requiredStates.length) {
      throw new Error(`Missing screenshot metadata for width ${width}`)
    }
    const hashes = requiredStates.map((state) => statesForWidth.get(state))
    if (new Set(hashes).size !== hashes.length) {
      throw new Error(`Stale fixture suspected at width ${width}: state screenshots matched`)
    }
    comparisons.push({ width, states: requiredStates })
  }
  return { passed: true, comparisons }
}

async function verifyProviderUsagePage(page, expectedState, width) {
  const fixtureState = await page.locator(".visual-qa-shell").getAttribute("data-visual-qa-state")
  if (fixtureState !== expectedState) {
    throw new Error(`${expectedState}@${width}: fixture state mismatch: ${fixtureState}`)
  }

  const overflow = await page.evaluate(() => ({
    innerWidth: window.innerWidth,
    scrollWidth: document.documentElement.scrollWidth
  }))
  if (overflow.scrollWidth > overflow.innerWidth) {
    throw new Error(`${expectedState}@${width}: horizontal overflow ${overflow.scrollWidth}px > ${overflow.innerWidth}px`)
  }

  const heading = await page.getByRole("heading", { name: "Usage" }).count()
  if (heading !== 1) {
    throw new Error(`${expectedState}@${width}: expected one Usage heading, found ${heading}`)
  }

  const controls = page.locator(".usage-window-controls button")
  const controlCount = await controls.count()
  if (controlCount !== 4) {
    throw new Error(`${expectedState}@${width}: expected 4 window controls, found ${controlCount}`)
  }

  return {
    controlCount,
    privacy: await assertDashboardVisibleTextPrivacy(page, expectedState, width),
    focus: await assertKeyboardFocusVisible(page, expectedState, width),
    textLayout: await assertTextLayout(page, expectedState, width),
    stateContent: await assertStateContent(page, expectedState, width)
  }
}

async function assertDashboardVisibleTextPrivacy(page, state, width) {
  const visibleText = await page.locator(".provider-usage-panel").innerText({ timeout: 1_000 })
  try {
    return scanDashboardVisibleText(visibleText)
  } catch (error) {
    if (error instanceof Error) {
      throw new Error(`${state}@${width}: ${error.message}`)
    }
    throw error
  }
}

async function assertKeyboardFocusVisible(page, state, width) {
  await page.evaluate(() => {
    if (document.activeElement instanceof HTMLElement) {
      document.activeElement.blur()
    }
  })

  const focusedLabels = []
  for (let attempt = 0; attempt < 10 && focusedLabels.length < 4; attempt += 1) {
    await page.keyboard.press("Tab")
    const focus = await page.evaluate(() => {
      const active = document.activeElement
      if (!(active instanceof HTMLButtonElement) || !active.closest(".usage-window-controls")) {
        return undefined
      }
      const style = window.getComputedStyle(active)
      return {
        label: active.textContent?.trim() ?? "",
        outlineStyle: style.outlineStyle,
        outlineWidth: style.outlineWidth
      }
    })
    if (focus === undefined) {
      continue
    }
    if (focus.outlineStyle === "none" || focus.outlineWidth === "0px") {
      throw new Error(`${state}@${width}: focused window control has no visible outline`)
    }
    if (!focusedLabels.includes(focus.label)) {
      focusedLabels.push(focus.label)
    }
  }

  if (focusedLabels.length !== 4) {
    throw new Error(`${state}@${width}: keyboard reached ${focusedLabels.length} of 4 window controls`)
  }
  return { focusedLabels }
}

async function assertTextLayout(page, state, width) {
  const problems = await page.evaluate(() => {
    const selector = [
      ".panel-header h2",
      ".eyebrow",
      ".usage-window-controls button",
      ".usage-state",
      ".empty-state",
      ".usage-state-detail",
      ".usage-section-header h3",
      ".usage-section-header p",
      ".usage-summary dt",
      ".usage-summary dd",
      ".usage-scope",
      ".usage-label"
    ].join(",")
    return Array.from(document.querySelectorAll(selector)).flatMap((element) => {
      const rect = element.getBoundingClientRect()
      const style = window.getComputedStyle(element)
      const label = `${element.tagName.toLowerCase()}:${element.textContent?.trim().slice(0, 48) ?? ""}`
      const localProblems = []
      if (rect.width <= 0 || rect.height <= 0 || style.display === "none" || style.visibility === "hidden") {
        localProblems.push(`${label} has no visible box`)
      }
      if (rect.left < -1) {
        localProblems.push(`${label} is clipped on the left at ${rect.left.toFixed(1)}`)
      }
      if (!element.closest(".usage-table-wrap") && rect.right > window.innerWidth + 1) {
        localProblems.push(`${label} is clipped on the right at ${rect.right.toFixed(1)}`)
      }
      if (element.scrollWidth > element.clientWidth + 1 && !element.closest(".usage-table-wrap")) {
        localProblems.push(`${label} text overflows its container`)
      }
      return localProblems
    })
  })
  if (problems.length > 0) {
    throw new Error(`${state}@${width}: text layout problems:\n${problems.join("\n")}`)
  }
  return { checkedElements: await page.locator(".provider-usage-panel").locator("text=/\\S/").count() }
}

async function assertStateContent(page, state, width) {
  switch (state) {
    case "loaded": {
      const providerTextCount = await page.getByText("provider <script>alert(\"usage\")</script>").count()
      const scriptNodes = await page.locator(".provider-usage-panel script").count()
      const imageNodes = await page.locator(".provider-usage-panel img").count()
      if (providerTextCount < 2 || scriptNodes !== 0 || imageNodes !== 0) {
        throw new Error(`${state}@${width}: hostile labels were not inert text`)
      }
      return { hostileProviderTextCount: providerTextCount, scriptNodes, imageNodes }
    }
    case "empty":
      await page.getByText("No provider usage recorded yet.").waitFor({ timeout: 1_000 })
      return { emptyStateVisible: true }
    case "error":
      await page.getByRole("alert").waitFor({ timeout: 1_000 })
      return { errorStateVisible: true }
    default:
      throw new Error(`Unsupported state content check: ${state}`)
  }
}

async function verifyFreshScreenshot(screenshotPath, screenshot, startedAtMs) {
  const info = await stat(screenshotPath)
  if (info.size <= 0 || info.mtimeMs < startedAtMs - 1_000) {
    throw new Error(`Screenshot was not regenerated during this run: ${relativePath(screenshotPath)}`)
  }
  const png = inspectPng(screenshot)
  if (!png.nonblank) {
    throw new Error(`Screenshot is blank: ${relativePath(screenshotPath)}`)
  }
  return {
    bytes: info.size,
    mtime: info.mtime.toISOString(),
    sha256: createHash("sha256").update(screenshot).digest("hex"),
    ...png
  }
}
