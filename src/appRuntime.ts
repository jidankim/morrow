import {
  createDefaultAppShellState,
  loadAppShellState,
  reduceAppShellState,
  type AppShellState,
  type createBrowserShellStorage
} from "./domain/appShell"
import type { ProviderUsageLoadRequest } from "./domain/providerUsage"
import type { NativeMenuCommand } from "./tauriBridge"

export type Route = "status" | "settings" | "usage" | "list-intake"

export const DEFAULT_PROVIDER_USAGE_REQUEST = {
  windowKey: "30d"
} as const satisfies ProviderUsageLoadRequest

export const routeFromHash = (): Route => {
  switch (window.location.hash) {
    case "#settings":
      return "settings"
    case "#usage":
      return "usage"
    case "#list-intake":
      return "list-intake"
    case "":
    case "#status":
      return "status"
    default:
      return "status"
  }
}

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

export function assertNeverRoute(route: never): never {
  throw new UnsupportedRouteError(route)
}

class UnsupportedNativeMenuCommandError extends Error {
  readonly command: NativeMenuCommand

  constructor(command: NativeMenuCommand) {
    super(`Unsupported native menu command: ${command}`)
    this.name = "UnsupportedNativeMenuCommandError"
    this.command = command
  }
}

class UnsupportedRouteError extends Error {
  readonly route: Route

  constructor(route: Route) {
    super(`Unsupported app route: ${route}`)
    this.name = "UnsupportedRouteError"
    this.route = route
  }
}
