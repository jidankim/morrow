import { chromium } from "playwright"
import { mkdir } from "node:fs/promises"
import { dirname } from "node:path"

const baseUrl = process.env.MORROW_SMOKE_URL ?? "http://127.0.0.1:1420"
const screenshotPath =
  process.env.MORROW_SMOKE_SCREENSHOT ?? ".omo/evidence/task-1-app-shell-smoke.png"

await mkdir(dirname(screenshotPath), { recursive: true })

const browser = await chromium.launch({ channel: "chrome", headless: true })
const page = await browser.newPage({ viewport: { width: 1280, height: 800 } })

await page.goto(baseUrl, { waitUntil: "networkidle" })
await page.getByRole("button", { name: "Pause" }).click()
await page.reload({ waitUntil: "networkidle" })

const syncButton = page.getByRole("button", { name: "Sync Now" })
const statusLabel = await page.getByTestId("status-label").textContent()
const syncState = await page.getByTestId("sync-state").textContent()
const syncDisabled = await syncButton.isDisabled()

await page.screenshot({ path: screenshotPath, fullPage: true })
await browser.close()

const smokeResult = {
  baseUrl,
  screenshotPath,
  statusAfterReload: statusLabel,
  syncStateAfterReload: syncState,
  syncDisabledAfterReload: syncDisabled
}

console.log(JSON.stringify(smokeResult, null, 2))

if (statusLabel !== "Paused" || syncState !== "Disabled" || !syncDisabled) {
  throw new Error("paused state did not persist with Sync Now disabled")
}
