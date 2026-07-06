import { useCallback, useEffect, useMemo, useRef, useState } from "react"
import type { DeleteAllState } from "./SettingsView"
import {
  APP_SHELL_STATE_KEY,
  createBrowserShellStorage,
  createDefaultAppShellState,
  getMenuModel,
  getOnboardingWarnings,
  isOnboardingComplete,
  isSyncNowEnabled,
  reduceAppShellState,
  saveAppShellState,
  type AppConfig,
  type AppMode,
  type AppShellState,
  type ChatId
} from "./domain/appShell"
import {
  DELETE_ALL_CONFIRMATION_TEXT,
  scrubCrashLogText,
  type DeleteAllOptions
} from "./domain/privacyControls"
import { privacyPaneName, requireDeleteReceipt } from "./domain/privacySettings"
import {
  createNativeShellBridge,
  type NativeMenuCommand,
  type PrivacySettingsPane
} from "./tauriBridge"
import { assertNeverNativeMenuCommand, loadInitialState, routeFromHash } from "./appRuntime"
import { nativeErrorMessage } from "./nativeErrors"
import { useMessagesDiscoveryActions } from "./useMessagesDiscovery"
import { useMessagesPreviewDisclosure } from "./useMessagesPreviewDisclosure"
import { useNativeShellState } from "./useNativeShellState"
import { useProviderUsageController } from "./useProviderUsageController"
import { useProviderCredentialActions } from "./useProviderCredentialActions"
import { useSyncSchedulerOrchestrator } from "./useSyncSchedulerOrchestrator"

export function useAppShellController() {
  const storage = useMemo(() => createBrowserShellStorage(window.localStorage), [])
  const nativeBridge = useMemo(() => createNativeShellBridge(), [])
  const [state, setState] = useState<AppShellState>(() => ({
    ...loadInitialState(storage),
    discovery: { status: "loading", chats: [] }
  }))
  const [route, setRoute] = useState(() => routeFromHash())
  const [syncing, setSyncing] = useState(false)
  const [deleteAllState, setDeleteAllState] = useState<DeleteAllState>({ status: "idle" })
  const syncInFlight = useRef(false)
  const runSyncNowRef = useRef<() => void>(() => undefined)
  const toggleAutomaticSyncRef = useRef<() => void>(() => undefined)
  const menu = getMenuModel(state)
  const warnings = getOnboardingWarnings(state)
  const syncEnabled = isSyncNowEnabled(state)
  const previewDisclosureActions = useMessagesPreviewDisclosure({ discovery: state.discovery, nativeBridge })
  const syncSchedulerActions = useSyncSchedulerOrchestrator({
    nativeBridge,
    state,
    setState,
    setSyncing,
    syncInFlight
  })
  const providerUsage = useProviderUsageController(route, nativeBridge)

  const openSettingsRoute = useCallback((): void => {
    window.location.hash = "#settings"
    setRoute("settings")
  }, [])

  useEffect(() => {
    const syncRoute = (): void => {
      setRoute(routeFromHash())
    }
    window.addEventListener("hashchange", syncRoute)
    return () => window.removeEventListener("hashchange", syncRoute)
  }, [])

  useNativeShellState(nativeBridge, setState)

  useEffect(() => {
    saveAppShellState(state, storage)
    void nativeBridge.setShellState({
      mode: state.mode,
      errorMessage: state.errorMessage,
      onboardingComplete: isOnboardingComplete(state),
      pendingProposalCount: state.pendingProposalCount,
      syncNowRunning: syncing,
      ...syncSchedulerActions.schedulerShellState
    })
  }, [nativeBridge, state, storage, syncing, syncSchedulerActions.schedulerShellState])

  const setMode = (mode: AppMode): void => {
    setState((current) => {
      if (mode === "paused") {
        return reduceAppShellState(current, { type: "pause" })
      }
      return reduceAppShellState(current, { type: "resume" })
    })
  }

  const updateConfig = (config: AppConfig): void => {
    setState((current) => reduceAppShellState(current, { type: "updateConfig", config }))
  }

  const setProviderCredentialStatus = useCallback(
    (providerCredentialStatus: AppShellState["providerCredentialStatus"]): void => {
      setState((current) =>
        reduceAppShellState(current, {
          type: "setProviderCredentialStatus",
          providerCredentialStatus
        })
      )
    },
    []
  )

  const {
    providerCredentialState,
    checkProviderCredential,
    confirmInstallCodexCli,
    cancelInstallCodexCli,
    startCodexLogin
  } = useProviderCredentialActions(nativeBridge, setProviderCredentialStatus)

  const toggleChat = (chatId: ChatId): void => {
    setState((current) => reduceAppShellState(current, { type: "toggleSelectedChat", chatId }))
  }

  const toggleBackfillPrompt = (chatId: ChatId, enabled: boolean): void => {
    setState((current) =>
      reduceAppShellState(current, { type: "setChatBackfillPrompt", chatId, enabled })
    )
  }

  const runSyncNow = (): void => {
    syncSchedulerActions.runSyncNow()
  }

  const deleteAllMorrowData = (options: DeleteAllOptions): void => {
    setDeleteAllState({ status: "deleting" })
    void nativeBridge
      .deleteMorrowData({
        confirmation: DELETE_ALL_CONFIRMATION_TEXT,
        cleanupProposedItems: options.cleanupProposedItems,
        deleteEmptyProposalContainers: options.deleteEmptyProposalContainers,
        revokeProviderOAuth: options.revokeProviderOAuth
      })
      .then((receipt) => {
        const deleteReceipt = requireDeleteReceipt(receipt)
        window.localStorage.removeItem(APP_SHELL_STATE_KEY)
        setState(createDefaultAppShellState())
        setProviderCredentialStatus("missing")
        setDeleteAllState({ status: "succeeded", receipt: deleteReceipt })
      })
      .catch((error: unknown) => {
        const message = scrubCrashLogText(
          nativeErrorMessage(error, "Delete all Morrow data could not complete.")
        )
        void nativeBridge.recordCrashLog({ message })
        setDeleteAllState({ status: "failed", message })
        setState((current) => reduceAppShellState(current, { type: "fail", message }))
      })
  }

  const openPrivacySettings = useCallback(
    async (pane: PrivacySettingsPane): Promise<void> => {
      const receipt = await nativeBridge.openPrivacySettings({ pane })
      if (receipt === undefined || !receipt.opened) {
        throw new Error(`${privacyPaneName(pane)} could not be opened.`)
      }
    },
    [nativeBridge]
  )

  const { loadMessagesDiscovery, openFullDiskAccess } = useMessagesDiscoveryActions({
    nativeBridge,
    openPrivacySettings,
    setState
  })

  runSyncNowRef.current = syncSchedulerActions.runSyncNow
  toggleAutomaticSyncRef.current = syncSchedulerActions.toggleAutomaticSync

  useEffect(() => {
    loadMessagesDiscovery()
  }, [loadMessagesDiscovery])

  useEffect(() => {
    void checkProviderCredential()
  }, [checkProviderCredential])

  useEffect(() => {
    let active = true
    let unsubscribe: (() => void) | undefined

    const handleMenuCommand = (command: NativeMenuCommand): void => {
      if (!active) {
        return
      }

      switch (command) {
        case "sync-now":
          runSyncNowRef.current()
          return
        case "open-settings":
        case "open-calendar":
        case "open-reminders":
          openSettingsRoute()
          return
        case "toggle-automatic-sync":
          toggleAutomaticSyncRef.current()
          return
        default:
          assertNeverNativeMenuCommand(command)
      }
    }

    void nativeBridge
      .subscribeMenuCommand(handleMenuCommand)
      .then((nextUnsubscribe) => {
        if (active) {
          unsubscribe = nextUnsubscribe
          return
        }
        nextUnsubscribe?.()
      })
      .catch((error: unknown) => {
        const message = nativeErrorMessage(error, "Native menu commands could not be read.")
        setState((current) => reduceAppShellState(current, { type: "fail", message }))
      })

    return () => {
      active = false
      unsubscribe?.()
    }
  }, [nativeBridge, openSettingsRoute])

  return {
    checkProviderCredential,
    changeAutomaticSyncInterval: syncSchedulerActions.changeAutomaticSyncInterval,
    cancelInstallCodexCli,
    confirmInstallCodexCli,
    deleteAllMorrowData,
    deleteAllState,
    loadMessagesDiscovery,
    menu,
    openFullDiskAccess,
    openPrivacySettings,
    openSettingsRoute,
    providerCredentialState,
    providerUsage,
    previewDisclosure: previewDisclosureActions.previewDisclosure,
    route,
    hidePreviews: previewDisclosureActions.hidePreviews,
    revealPreviews: previewDisclosureActions.revealPreviews,
    runSyncNow,
    setMode,
    startCodexLogin,
    state,
    syncScheduler: syncSchedulerActions.syncScheduler,
    syncSchedulerNowUnixSeconds: syncSchedulerActions.syncSchedulerNowUnixSeconds,
    syncEnabled,
    syncing,
    toggleAutomaticSync: syncSchedulerActions.toggleAutomaticSync,
    toggleBackfillPrompt,
    toggleChat,
    updateConfig,
    warnings
  } as const
}
