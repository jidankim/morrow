import { useCallback, type Dispatch, type SetStateAction } from "react"
import { reduceAppShellState, type AppShellState } from "./domain/appShell"
import { chatDiscoveryFromReport } from "./messagesDiscoveryBridge"
import { nativeErrorMessage } from "./nativeErrors"
import type {
  NativeShellBridge,
  PrivacySettingsPane
} from "./tauriBridge"

type MessagesDiscoveryParams = {
  readonly nativeBridge: NativeShellBridge
  readonly setState: Dispatch<SetStateAction<AppShellState>>
  readonly openPrivacySettings: (pane: PrivacySettingsPane) => Promise<void>
}

type MessagesDiscoveryActions = {
  readonly loadMessagesDiscovery: () => void
  readonly openFullDiskAccess: () => void
}

export function useMessagesDiscoveryActions({
  nativeBridge,
  setState,
  openPrivacySettings
}: MessagesDiscoveryParams): MessagesDiscoveryActions {
  const openFullDiskAccess = useCallback((): void => {
    void openPrivacySettings("fullDiskAccess").catch((error: unknown) => {
      const message = nativeErrorMessage(error, "Full Disk Access settings could not be opened.")
      setState((current) => reduceAppShellState(current, { type: "fail", message }))
    })
  }, [openPrivacySettings, setState])

  const loadMessagesDiscovery = useCallback((): void => {
    setState((current) =>
      reduceAppShellState(current, {
        type: "updateChatDiscovery",
        discovery: { status: "loading", chats: [] }
      })
    )
    void nativeBridge
      .discoverMessagesChats()
      .then((report) => {
        setState((current) =>
          reduceAppShellState(current, {
            type: "updateChatDiscovery",
            discovery: chatDiscoveryFromReport(report)
          })
        )
      })
      .catch(() => {
        setState((current) =>
          reduceAppShellState(current, {
            type: "updateChatDiscovery",
            discovery: { status: "unavailable", chats: [] }
          })
        )
      })
  }, [nativeBridge, setState])

  return { loadMessagesDiscovery, openFullDiskAccess }
}
