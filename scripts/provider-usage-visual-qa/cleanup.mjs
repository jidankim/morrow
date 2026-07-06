import process from "node:process"
import { setTimeout as delay } from "node:timers/promises"
import { writeJson } from "./artifacts.mjs"

let cleanupContext

export function initializeCleanup(evidenceDir) {
  cleanupContext = {
    browser: undefined,
    cleanup: {
      devServerPid: undefined,
      devServerExitCode: undefined,
      devServerSignal: undefined,
      devServerKill: "not-started",
      browserClose: "not-started",
      tempCleanup: "not-needed",
      interruptSignal: undefined
    },
    evidenceDir,
    server: undefined
  }
  return cleanupContext
}

export function registerCleanupHandlers() {
  for (const signal of ["SIGINT", "SIGTERM"]) {
    process.once(signal, () => {
      void cleanupResources(signal).finally(() => {
        process.exit(signal === "SIGINT" ? 130 : 143)
      })
    })
  }
}

export async function cleanupResources(reason) {
  if (cleanupContext === undefined) {
    return { reason, devServerPid: undefined, devServerKill: "not-started", browserClose: "not-started", tempCleanup: "not-needed" }
  }

  const { browser, cleanup, evidenceDir, server } = cleanupContext
  cleanup.interruptSignal = reason === "SIGINT" || reason === "SIGTERM" ? reason : undefined

  if (browser !== undefined && cleanup.browserClose === "not-started") {
    await browser.close()
    cleanup.browserClose = "closed"
  }

  if (server === undefined) {
    cleanup.devServerKill = "not-started"
  } else if (server.exitCode !== null) {
    cleanup.devServerExitCode = server.exitCode
    cleanup.devServerSignal = server.signalCode
    cleanup.devServerKill = "already-exited"
  } else if (cleanup.devServerKill === "not-started") {
    server.kill("SIGTERM")
    const exited = new Promise((resolve) => server.once("exit", () => resolve(true)))
    const exitedAfterTerm = await Promise.race([exited, delay(5_000).then(() => false)])
    if (exitedAfterTerm) {
      cleanup.devServerKill = "sigterm"
    } else {
      server.kill("SIGKILL")
      await new Promise((resolve) => server.once("exit", () => resolve(true)))
      cleanup.devServerKill = "sigkill"
    }
    cleanup.devServerExitCode = server.exitCode
    cleanup.devServerSignal = server.signalCode
  }

  await writeJson(evidenceDir, "cleanup.json", cleanup)
  return cleanup
}
