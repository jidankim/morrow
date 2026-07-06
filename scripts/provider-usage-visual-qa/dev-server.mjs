import { spawn } from "node:child_process"
import net from "node:net"
import process from "node:process"
import { setTimeout as delay } from "node:timers/promises"
import { sanitizeText } from "./artifacts.mjs"
import { workspaceRoot } from "./constants.mjs"

export function startDevServer(port, serverLog) {
  const server = spawn("npm", ["run", "dev", "--", "--port", String(port), "--strictPort"], {
    cwd: workspaceRoot,
    env: { ...process.env, VITE_DISABLE_REACT_DEVTOOLS: "1" },
    stdio: ["ignore", "pipe", "pipe"]
  })
  server.stdout.on("data", (chunk) => serverLog.push(sanitizeText(String(chunk))))
  server.stderr.on("data", (chunk) => serverLog.push(sanitizeText(String(chunk))))
  return server
}

export async function waitForServer(baseUrl, server, serverLog) {
  const deadline = Date.now() + 25_000
  while (Date.now() < deadline) {
    if (server.exitCode !== null) {
      throw new Error(`Vite exited before readiness:\n${serverLog.join("")}`)
    }
    if (await serverResponds(baseUrl)) {
      return
    }
    await delay(250)
  }
  throw new Error(`Timed out waiting for ${baseUrl}:\n${serverLog.join("")}`)
}

export async function findAvailablePort() {
  const server = net.createServer()
  await new Promise((resolve, reject) => {
    server.once("error", reject)
    server.listen(0, "127.0.0.1", resolve)
  })
  const address = server.address()
  await new Promise((resolve) => server.close(resolve))
  if (address === null || typeof address === "string") {
    throw new Error("Could not allocate a local dev server port")
  }
  return address.port
}

async function serverResponds(baseUrl) {
  const controller = new AbortController()
  const timeout = setTimeout(() => controller.abort(), 500)
  try {
    const response = await fetch(baseUrl, { signal: controller.signal })
    return response.ok
  } catch (error) {
    if (error instanceof Error || error instanceof DOMException) {
      return false
    }
    throw error
  } finally {
    clearTimeout(timeout)
  }
}
