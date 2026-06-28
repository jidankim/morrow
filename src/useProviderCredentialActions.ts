import { useCallback, useMemo, useState } from "react"
import { type ProviderCredentialState } from "./SettingsView"
import type { ProviderCredentialStatus } from "./domain/appShell"
import {
  MORROW_KEYCHAIN_SERVICE,
  MORROW_PROVIDER_TOKEN_KIND,
  type MorrowTokenLookupRequest,
  type NativeShellBridge
} from "./tauriBridge"
import { nativeErrorMessage } from "./nativeErrors"

type ProviderCredentialActions = {
  readonly providerCredentialState: ProviderCredentialState
  readonly checkProviderCredential: () => Promise<void>
  readonly deleteProviderCredential: () => Promise<void>
  readonly saveProviderCredential: (token: string) => Promise<void>
}

export function useProviderCredentialActions(
  nativeBridge: NativeShellBridge,
  onProviderCredentialStatusChange?: (status: ProviderCredentialStatus) => void
): ProviderCredentialActions {
  const [providerCredentialState, setProviderCredentialState] =
    useState<ProviderCredentialState>({ status: "idle" })
  const providerTokenLookup = useMemo<MorrowTokenLookupRequest>(
    () => ({
      service: MORROW_KEYCHAIN_SERVICE,
      tokenKind: MORROW_PROVIDER_TOKEN_KIND
    }),
    []
  )

  const saveProviderCredential = useCallback(
    async (token: string): Promise<void> => {
      setProviderCredentialState({ status: "saving" })
      try {
        const receipt = await nativeBridge.storeMorrowToken({
          ...providerTokenLookup,
          token
        })
        if (receipt === undefined || !receipt.stored) {
          throw new Error("Provider credential could not be saved.")
        }
        onProviderCredentialStatusChange?.("configured")
        setProviderCredentialState({ status: "saved" })
      } catch (error: unknown) {
        const message =
          error instanceof Error || typeof error === "string"
            ? nativeErrorMessage(error, "Provider credential could not be saved.")
            : "Provider credential could not be saved."
        setProviderCredentialState({ status: "failed", message })
      }
    },
    [nativeBridge, onProviderCredentialStatusChange, providerTokenLookup]
  )

  const checkProviderCredential = useCallback(async (): Promise<void> => {
    setProviderCredentialState({ status: "checking" })
    try {
      const response = await nativeBridge.readMorrowToken(providerTokenLookup)
      const credentialConfigured = response?.present === true
      onProviderCredentialStatusChange?.(credentialConfigured ? "configured" : "missing")
      setProviderCredentialState({ status: credentialConfigured ? "present" : "missing" })
    } catch (error: unknown) {
      const message =
        error instanceof Error || typeof error === "string"
          ? nativeErrorMessage(error, "Provider credential could not be checked.")
          : "Provider credential could not be checked."
      onProviderCredentialStatusChange?.("missing")
      setProviderCredentialState({ status: "failed", message })
    }
  }, [nativeBridge, onProviderCredentialStatusChange, providerTokenLookup])

  const deleteProviderCredential = useCallback(async (): Promise<void> => {
    setProviderCredentialState({ status: "deleting" })
    try {
      await nativeBridge.deleteMorrowToken(providerTokenLookup)
      onProviderCredentialStatusChange?.("missing")
      setProviderCredentialState({ status: "deleted" })
    } catch (error: unknown) {
      const message =
        error instanceof Error || typeof error === "string"
          ? nativeErrorMessage(error, "Provider credential could not be deleted.")
          : "Provider credential could not be deleted."
      setProviderCredentialState({ status: "failed", message })
    }
  }, [nativeBridge, onProviderCredentialStatusChange, providerTokenLookup])

  return {
    providerCredentialState,
    checkProviderCredential,
    deleteProviderCredential,
    saveProviderCredential
  }
}
