import { useCallback, useEffect, useRef, useState } from "react"
import { type ProviderCredentialState } from "./SettingsView"
import type { ProviderCredentialStatus } from "./domain/appShell"
import type { CodexProviderAuthReadiness, NativeShellBridge } from "./tauriBridge"
import {
  pollProviderLogin,
  providerStateFromReadiness,
  readinessFailure
} from "./providerCredentialSetupState"

type ProviderCredentialActions = {
  readonly providerCredentialState: ProviderCredentialState
  readonly checkProviderCredential: () => Promise<void>
  readonly confirmInstallCodexCli: () => Promise<void>
  readonly cancelInstallCodexCli: () => void
  readonly startCodexLogin: () => Promise<void>
}

export function useProviderCredentialActions(
  nativeBridge: NativeShellBridge,
  onProviderCredentialStatusChange?: (status: ProviderCredentialStatus) => void
): ProviderCredentialActions {
  const [providerCredentialState, setProviderCredentialState] =
    useState<ProviderCredentialState>({ status: "idle" })
  const activeOperationIdRef = useRef(0)
  const actionInFlightRef = useRef(false)

  useEffect(() => {
    return () => {
      activeOperationIdRef.current += 1
      actionInFlightRef.current = false
    }
  }, [])

  const applyReadiness = useCallback(
    (readiness: CodexProviderAuthReadiness | undefined): void => {
      const nextState = providerStateFromReadiness(readiness)
      onProviderCredentialStatusChange?.(nextState.status === "ready" ? "configured" : "missing")
      setProviderCredentialState(nextState)
    },
    [onProviderCredentialStatusChange]
  )

  const checkProviderCredential = useCallback(async (): Promise<void> => {
    if (actionInFlightRef.current) {
      return
    }
    actionInFlightRef.current = true
    const operationId = activeOperationIdRef.current + 1
    activeOperationIdRef.current = operationId
    setProviderCredentialState({ status: "checking" })
    try {
      const readiness = await nativeBridge.checkProviderAuth()
      if (activeOperationIdRef.current === operationId) {
        applyReadiness(readiness)
      }
    } catch (error: unknown) {
      if (isHandledProviderReadinessError(error)) {
        onProviderCredentialStatusChange?.("missing")
        setProviderCredentialState(readinessFailure("Codex provider readiness could not be checked.", "refresh"))
        return
      }
      throw error
    } finally {
      if (activeOperationIdRef.current === operationId) {
        actionInFlightRef.current = false
      }
    }
  }, [applyReadiness, nativeBridge, onProviderCredentialStatusChange])

  const confirmInstallCodexCli = useCallback(async (): Promise<void> => {
    if (actionInFlightRef.current) {
      return
    }
    if (
      providerCredentialState.status === "missingCli" ||
      (providerCredentialState.status === "failed" && providerCredentialState.recoveryAction === "setup")
    ) {
      setProviderCredentialState({ status: "installConfirming" })
      return
    }
    if (providerCredentialState.status !== "installConfirming") {
      return
    }

    actionInFlightRef.current = true
    const operationId = activeOperationIdRef.current + 1
    activeOperationIdRef.current = operationId
    setProviderCredentialState({ status: "installing" })
    try {
      const receipt = await nativeBridge.installCodexCli()
      if (activeOperationIdRef.current !== operationId) {
        return
      }
      if (receipt === undefined) {
        setProviderCredentialState(readinessFailure("Codex CLI setup could not complete.", "setup"))
        onProviderCredentialStatusChange?.("missing")
        return
      }
      switch (receipt.status) {
        case "installed":
        case "alreadyInstalled":
          applyReadiness(await nativeBridge.checkProviderAuth())
          return
        case "alreadyRunning":
          setProviderCredentialState(
            readinessFailure("Codex CLI setup is already running. Wait for it to finish, then retry setup.", "setup")
          )
          onProviderCredentialStatusChange?.("missing")
          return
        case "failed":
        case "timeout":
        case "unsupported":
          setProviderCredentialState(readinessFailure("Codex CLI setup could not complete.", "setup"))
          onProviderCredentialStatusChange?.("missing")
          return
        default:
          assertNever(receipt.status)
      }
    } catch (error: unknown) {
      if (isHandledProviderReadinessError(error)) {
        setProviderCredentialState(readinessFailure("Codex CLI setup could not complete.", "setup"))
        onProviderCredentialStatusChange?.("missing")
        return
      }
      throw error
    } finally {
      if (activeOperationIdRef.current === operationId) {
        actionInFlightRef.current = false
      }
    }
  }, [applyReadiness, nativeBridge, onProviderCredentialStatusChange, providerCredentialState])

  const cancelInstallCodexCli = useCallback((): void => {
    if (actionInFlightRef.current) {
      return
    }
    setProviderCredentialState({ status: "missingCli" })
  }, [])

  const startCodexLogin = useCallback(async (): Promise<void> => {
    const canStartLogin =
      providerCredentialState.status === "notLoggedIn" ||
      (providerCredentialState.status === "failed" && providerCredentialState.recoveryAction === "login")
    if (actionInFlightRef.current || !canStartLogin) {
      return
    }
    actionInFlightRef.current = true
    const operationId = activeOperationIdRef.current + 1
    activeOperationIdRef.current = operationId
    setProviderCredentialState({ status: "loginLaunching" })
    try {
      const receipt = await nativeBridge.startCodexLogin()
      if (activeOperationIdRef.current !== operationId) {
        return
      }
      if (receipt === undefined) {
        onProviderCredentialStatusChange?.("missing")
        setProviderCredentialState(readinessFailure("Codex login could not be started.", "login"))
        return
      }
      switch (receipt.status) {
        case "launched":
        case "alreadyRunning":
          setProviderCredentialState({ status: "loginPolling" })
          {
            const pollResult = await pollProviderLogin(
              nativeBridge,
              () => activeOperationIdRef.current === operationId
            )
            switch (pollResult.status) {
              case "cancelled":
                return
              case "timeout":
                onProviderCredentialStatusChange?.("missing")
                setProviderCredentialState(
                  readinessFailure("Codex login was not detected within 180 seconds.", "refresh")
                )
                return
              case "ready": {
                applyReadiness(pollResult.readiness)
                return
              }
              default:
                return assertNever(pollResult)
            }
          }
        case "missingCli":
          onProviderCredentialStatusChange?.("missing")
          setProviderCredentialState({ status: "missingCli" })
          return
        case "failedToStart":
          onProviderCredentialStatusChange?.("missing")
          setProviderCredentialState(readinessFailure("Codex login could not be started.", "login"))
          return
        default:
          assertNever(receipt.status)
      }
    } catch (error: unknown) {
      if (isHandledProviderReadinessError(error)) {
        onProviderCredentialStatusChange?.("missing")
        setProviderCredentialState(readinessFailure("Codex login could not be started.", "login"))
        return
      }
      throw error
    } finally {
      if (activeOperationIdRef.current === operationId) {
        actionInFlightRef.current = false
      }
    }
  }, [applyReadiness, nativeBridge, onProviderCredentialStatusChange, providerCredentialState])

  return {
    providerCredentialState,
    checkProviderCredential,
    confirmInstallCodexCli,
    cancelInstallCodexCli,
    startCodexLogin
  }
}

function isHandledProviderReadinessError(error: unknown): error is Error | string {
  return error instanceof Error || typeof error === "string"
}

function assertNever(value: never): never {
  throw new Error(`Unhandled provider credential variant: ${String(value)}`)
}
