import { spawn } from "node:child_process"
import { mkdir, stat, writeFile } from "node:fs/promises"
import path from "node:path"
import process from "node:process"
import { setTimeout as delay } from "node:timers/promises"

let cleanupStarted = false

export function startDevServer({ port, serverLog }) {
  const server = spawn("npm", ["run", "dev", "--", "--port", port, "--strictPort"], {
    cwd: process.cwd(),
    env: { ...process.env, VITE_DISABLE_REACT_DEVTOOLS: "1" },
    stdio: ["ignore", "pipe", "pipe"]
  })
  server.stdout.on("data", (chunk) => serverLog.push(String(chunk)))
  server.stderr.on("data", (chunk) => serverLog.push(String(chunk)))
  return server
}

export async function waitForServer({ baseUrl, server, serverLog }) {
  const deadline = Date.now() + 20_000
  while (Date.now() < deadline) {
    if (server.exitCode !== null) {
      throw new Error(`Vite exited before it became ready:\n${serverLog.join("")}`)
    }
    if (await serverResponds(baseUrl)) {
      return
    }
    await delay(250)
  }
  throw new Error(`Timed out waiting for ${baseUrl}:\n${serverLog.join("")}`)
}

export async function serverResponds(baseUrl) {
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

export async function cleanupServer(server) {
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

export async function cleanupReceipt({ baseUrl, server }) {
  return {
    childExited: server !== undefined ? server.exitCode !== null : true,
    exitCode: server?.exitCode ?? null,
    signal: server?.signalCode ?? null,
    portRespondsAfterCleanup: await serverResponds(baseUrl)
  }
}

export async function assertNoHorizontalOverflow({ page, state, width }) {
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

export async function assertVisibleBox({ label, locator, state, width }) {
  const box = await locator.boundingBox()
  if (box === null || box.width <= 0 || box.height <= 0) {
    throw new Error(`${state}@${width}: ${label} has no positive visible box`)
  }
  return box
}

export async function assertFocusable({ label, locator, state, width }) {
  await locator.focus()
  const focused = await locator.evaluate((element) => document.activeElement === element)
  if (!focused) {
    throw new Error(`${state}@${width}: ${label} is not focusable`)
  }
}

export async function verifyFreshScreenshot({ screenshotPath, startedAtMs }) {
  const info = await stat(screenshotPath)
  if (info.size <= 1_024 || info.mtimeMs < startedAtMs - 1_000) {
    throw new Error(`Screenshot was not regenerated during this run: ${screenshotPath}`)
  }
  return { bytes: info.size, mtime: info.mtime.toISOString() }
}

export async function writeReport(reportPath, report) {
  await mkdir(path.dirname(reportPath), { recursive: true })
  await writeFile(reportPath, `${JSON.stringify(report, null, 2)}\n`)
}

export function errorMessage(error) {
  return error instanceof Error ? error.message : String(error)
}
