import {
  createDefaultAppShellState,
  loadAppShellState,
  reduceAppShellState,
  type AppShellState,
  type createBrowserShellStorage
} from "./domain/appShell"
import type { NativeMenuCommand } from "./tauriBridge"

export type Route = "status" | "settings"

export const routeFromHash = (): Route =>
  window.location.hash === "#settings" ? "settings" : "status"

export function loadInitialState(
  storage: ReturnType<typeof createBrowserShellStorage>
): AppShellState {
  try {
    return loadAppShellState(storage)
  } catch (error: unknown) {
    if (error instanceof Error) {
      return reduceAppShellState(createDefaultAppShellState(), {
        type: "fail",
        message: "Local configuration could not be read."
      })
    }
    throw error
  }
}

export function assertNeverNativeMenuCommand(command: never): never {
  throw new UnsupportedNativeMenuCommandError(command)
}

class UnsupportedNativeMenuCommandError extends Error {
  readonly command: NativeMenuCommand

  constructor(command: NativeMenuCommand) {
    super(`Unsupported native menu command: ${command}`)
    this.name = "UnsupportedNativeMenuCommandError"
    this.command = command
  }
}
