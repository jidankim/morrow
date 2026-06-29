import { useCallback, useState } from "react"
import { type ProviderCredentialState } from "./SettingsView"
import type { ProviderCredentialStatus } from "./domain/appShell"
import type { CodexProviderAuthReadiness, NativeShellBridge } from "./tauriBridge"

type ProviderCredentialActions = {
  readonly providerCredentialState: ProviderCredentialState
  readonly checkProviderCredential: () => Promise<void>
}

export function useProviderCredentialActions(
  nativeBridge: NativeShellBridge,
  onProviderCredentialStatusChange?: (status: ProviderCredentialStatus) => void
): ProviderCredentialActions {
  const [providerCredentialState, setProviderCredentialState] =
    useState<ProviderCredentialState>({ status: "idle" })

  const checkProviderCredential = useCallback(async (): Promise<void> => {
    setProviderCredentialState({ status: "checking" })
    try {
      const readiness = await nativeBridge.checkProviderAuth()
      const nextState = providerStateFromReadiness(readiness)
      onProviderCredentialStatusChange?.(nextState.status === "ready" ? "configured" : "missing")
      setProviderCredentialState(nextState)
    } catch (error: unknown) {
      if (isHandledProviderReadinessError(error)) {
        onProviderCredentialStatusChange?.("missing")
        setProviderCredentialState({
          status: "failed",
          message: "Codex provider readiness could not be checked."
        })
        return
      }
      throw error
    }
  }, [nativeBridge, onProviderCredentialStatusChange])

  return {
    providerCredentialState,
    checkProviderCredential
  }
}

function providerStateFromReadiness(
  readiness: CodexProviderAuthReadiness | undefined
): ProviderCredentialState {
  if (readiness?.ready === true && readiness.status === "loggedInUsingChatGpt") {
    return { status: "ready" }
  }
  return { status: "missing", reason: readiness?.status ?? "unknownFailure" }
}

function isHandledProviderReadinessError(error: unknown): error is Error | string {
  return error instanceof Error || typeof error === "string"
}
