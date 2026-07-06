import type { ProviderCredentialState } from "./SettingsProviderCredentialSection"
import type { CodexProviderAuthReadiness, NativeShellBridge } from "./tauriBridge"

export type ProviderCredentialFailureRecovery = "refresh" | "setup" | "login"

export type ProviderLoginPollResult =
  | { readonly status: "ready"; readonly readiness: CodexProviderAuthReadiness }
  | { readonly status: "timeout" }
  | { readonly status: "cancelled" }

const LOGIN_POLL_INTERVAL_MS = 2_000
const LOGIN_POLL_TIMEOUT_MS = 180_000

export function providerStateFromReadiness(
  readiness: CodexProviderAuthReadiness | undefined
): ProviderCredentialState {
  if (readiness?.ready === true && readiness.status === "loggedInUsingChatGpt") {
    return { status: "ready" }
  }
  const readinessStatus = readiness?.status
  switch (readinessStatus) {
    case "missingCli":
      return { status: "missingCli" }
    case "notLoggedIn":
      return { status: "notLoggedIn" }
    case "timeout":
      return readinessFailure("Codex provider readiness timed out.", "refresh")
    case "unknownFailure":
      return readinessFailure("Codex provider readiness could not be confirmed.", "refresh")
    case "loggedInUsingChatGpt":
    case undefined:
      return readinessFailure("Codex provider readiness could not be checked.", "refresh")
    default:
      return assertNever(readinessStatus)
  }
}

export async function pollProviderLogin(
  nativeBridge: NativeShellBridge,
  isOperationActive: () => boolean
): Promise<ProviderLoginPollResult> {
  for (
    let elapsedMs = 0;
    elapsedMs < LOGIN_POLL_TIMEOUT_MS;
    elapsedMs += LOGIN_POLL_INTERVAL_MS
  ) {
    await delay(LOGIN_POLL_INTERVAL_MS)
    if (!isOperationActive()) {
      return { status: "cancelled" }
    }
    const readiness = await nativeBridge.checkProviderAuth()
    if (readiness?.ready === true && readiness.status === "loggedInUsingChatGpt") {
      return { status: "ready", readiness }
    }
  }
  return { status: "timeout" }
}

export function readinessFailure(
  message: string,
  recoveryAction: ProviderCredentialFailureRecovery
): ProviderCredentialState {
  return { status: "failed", message, recoveryAction }
}

function delay(milliseconds: number): Promise<void> {
  return new Promise((resolve) => {
    window.setTimeout(resolve, milliseconds)
  })
}

function assertNever(value: never): never {
  throw new Error(`Unhandled provider credential setup variant: ${String(value)}`)
}
