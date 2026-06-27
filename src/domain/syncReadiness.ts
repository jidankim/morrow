import { selectedChatsAreVerified, type ChatDiscovery } from "./chatDiscovery"
import type { AppMode, AppShellState } from "./appShell"

export type SyncReadinessItemId =
  | "permissions"
  | "discovery"
  | "chat-selection"
  | "selected-chat-verification"
  | "pause-state"
  | "sync-activity"

export type SyncReadinessItemStatus = "complete" | "blocking" | "pending"

export type SyncReadinessOptions = {
  readonly syncing?: boolean
}

export type SyncReadinessItem = {
  readonly id: SyncReadinessItemId
  readonly label: string
  readonly status: SyncReadinessItemStatus
  readonly detail: string
}

const syncReadinessItemLabels = {
  permissions: "Required permissions",
  discovery: "Messages discovery",
  "chat-selection": "Chat selection",
  "selected-chat-verification": "Selected chat verification",
  "pause-state": "Scanning state",
  "sync-activity": "Sync Now activity"
} as const satisfies Record<SyncReadinessItemId, string>

export function getSyncReadinessItems(
  state: AppShellState,
  options: SyncReadinessOptions = {}
): readonly SyncReadinessItem[] {
  const selectedChatCount = state.selectedChats.length
  const selectedChatsVerified =
    selectedChatCount > 0 && selectedChatsAreVerified(state.discovery, state.selectedChats)
  return [
    syncReadinessItem(
      "permissions",
      state.config.permissionsGranted ? "complete" : "blocking",
      state.config.permissionsGranted
        ? "Required permissions are complete."
        : "Complete required permissions before scanning."
    ),
    getDiscoveryReadinessItem(state.discovery),
    syncReadinessItem(
      "chat-selection",
      selectedChatCount > 0 ? "complete" : "blocking",
      selectedChatCount > 0
        ? formatSelectedChatCount(selectedChatCount)
        : "Select at least one chat before scanning."
    ),
    syncReadinessItem(
      "selected-chat-verification",
      selectedChatCount === 0 ? "pending" : selectedChatsVerified ? "complete" : "blocking",
      getSelectedChatVerificationDetail(selectedChatCount, selectedChatsVerified)
    ),
    getPauseReadinessItem(state.mode),
    syncReadinessItem(
      "sync-activity",
      options.syncing === true ? "blocking" : "complete",
      options.syncing === true ? "Sync Now is already running." : "Sync Now is not already running."
    )
  ]
}

function getDiscoveryReadinessItem(discovery: ChatDiscovery): SyncReadinessItem {
  switch (discovery.status) {
    case "ready":
      return syncReadinessItem(
        "discovery",
        "complete",
        formatDiscoveredChatCount(discovery.chats.length)
      )
    case "unverified":
      return syncReadinessItem("discovery", "blocking", "Run chat discovery before scanning.")
    case "loading":
      return syncReadinessItem("discovery", "blocking", "Morrow is checking local Messages access.")
    case "empty":
      return syncReadinessItem("discovery", "blocking", "Messages discovery found no eligible chats.")
    case "permissionDenied":
      return syncReadinessItem("discovery", "blocking", "Grant Full Disk Access, then retry chat discovery.")
    case "unavailable":
      return syncReadinessItem("discovery", "blocking", "Retry chat discovery or check local Messages access.")
    default:
      return assertNever(discovery.status)
  }
}

function getSelectedChatVerificationDetail(selectedChatCount: number, selectedChatsVerified: boolean): string {
  if (selectedChatCount === 0) {
    return "Select a chat to verify it for scanning."
  }
  if (selectedChatsVerified) {
    return "Selected chats are verified for scanning."
  }
  return "Refresh chat discovery before scanning selected chats."
}

function getPauseReadinessItem(mode: AppMode): SyncReadinessItem {
  switch (mode) {
    case "scanning":
      return syncReadinessItem("pause-state", "complete", "Scanning is active.")
    case "paused":
      return syncReadinessItem("pause-state", "blocking", "Resume scanning to enable Sync Now.")
    case "error":
      return syncReadinessItem("pause-state", "complete", "Sync Now can retry after the current error.")
    default:
      return assertNever(mode)
  }
}

function formatDiscoveredChatCount(count: number): string {
  return `Messages discovery found ${count} eligible ${count === 1 ? "chat" : "chats"}.`
}

function formatSelectedChatCount(count: number): string {
  return `${count} ${count === 1 ? "chat" : "chats"} selected.`
}

function syncReadinessItem(
  id: SyncReadinessItemId,
  status: SyncReadinessItemStatus,
  detail: string
): SyncReadinessItem {
  return { id, label: syncReadinessItemLabels[id], status, detail }
}

function assertNever(value: never): never {
  throw new UnhandledSyncReadinessVariantError(String(value))
}

class UnhandledSyncReadinessVariantError extends Error {
  readonly renderedValue: string

  constructor(renderedValue: string) {
    super(`Unhandled sync readiness variant: ${renderedValue}`)
    this.name = "UnhandledSyncReadinessVariantError"
    this.renderedValue = renderedValue
  }
}
