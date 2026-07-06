import { UnhandledAppShellVariantError } from "./appShellErrors"
import {
  defaultChatDiscovery,
  reconcileSelectedChats,
  setChatBackfillPromptSelection,
  toggleSelectedChatSelection,
  type ChatDiscovery,
  type ChatId,
  type DiscoveredChat,
  type SelectedChat
} from "./chatDiscovery"
import { createDefaultAppConfig, type AppConfig } from "./appConfig"
import {
  emptyDecisionEvidenceReport,
  type DecisionEvidenceReport
} from "./decisionEvidence"
import {
  loadPersistedAppShellState,
  savePersistedAppShellState,
  type ShellStorage
} from "./appShellStorage"
import {
  emptySyncResultCounts,
  syncResultCountsFrom,
  type PartialSyncResultCounts,
  type SyncResultCounts
} from "./syncResultCounts"
export {
  getSyncReadinessItems,
  type SyncReadinessItem,
  type SyncReadinessItemId,
  type SyncReadinessItemStatus,
  type SyncReadinessOptions
} from "./syncReadiness"
export {
  formatPendingProposalCount,
  getMenuModel,
  getOnboardingWarnings,
  isOnboardingComplete,
  isSyncNowEnabled,
  type MenuModel,
  type MenuStatusKind
} from "./appMenuModel"

export { APP_SHELL_STATE_KEY, createBrowserShellStorage } from "./appShellStorage"
export type { ShellStorage } from "./appShellStorage"

export type AppMode = "scanning" | "paused" | "error"
export type AutomaticSyncStatusLabel = "Off" | "On" | "Cooling Down" | "Needs Action"
export type ProviderCredentialStatus = "unchecked" | "configured" | "missing"
export type { AppConfig, CalendarSource } from "./appConfig"
export type {
  ChatDiscovery,
  ChatDiscoveryStatus,
  ChatId,
  ChatOption,
  DiscoveredChat,
  ParticipantId,
  SelectedChat
} from "./chatDiscovery"

export type AppShellState = SyncResultCounts & {
  readonly mode: AppMode
  readonly errorMessage?: string | undefined
  readonly config: AppConfig
  readonly providerCredentialStatus: ProviderCredentialStatus
  readonly discovery: ChatDiscovery
  readonly selectedChats: readonly SelectedChat[]
  readonly decisionEvidence: DecisionEvidenceReport
}

export type NativeAppShellState = {
  readonly mode: AppMode
  readonly errorMessage?: string | undefined
  readonly onboardingComplete: boolean
  readonly pendingProposalCount: number
  readonly syncNowRunning: boolean
  readonly automaticSyncEnabled: boolean
  readonly automaticSyncStatusLabel: AutomaticSyncStatusLabel
  readonly automaticSyncDetail: string
}

export type AppShellEvent =
  | { readonly type: "pause" }
  | { readonly type: "resume" }
  | {
      readonly type: "fail"
      readonly message: string
    }
  | { readonly type: "clearError" }
  | {
      readonly type: "updateConfig"
      readonly config: AppConfig
    }
  | {
      readonly type: "setProviderCredentialStatus"
      readonly providerCredentialStatus: ProviderCredentialStatus
    }
  | {
      readonly type: "updateChatDiscovery"
      readonly discovery: ChatDiscovery
    }
  | {
      readonly type: "toggleSelectedChat"
      readonly chatId: ChatId
      readonly chat?: DiscoveredChat
    }
  | {
      readonly type: "setChatBackfillPrompt"
      readonly chatId: ChatId
      readonly enabled: boolean
    }
  | { readonly type: "clearDecisionEvidence" }
  | ({ readonly type: "syncCompleted"; readonly decisionEvidence?: DecisionEvidenceReport } & PartialSyncResultCounts)

export function createDefaultAppShellState(
  browserTimeZone = Intl.DateTimeFormat().resolvedOptions().timeZone
): AppShellState {
  return {
    mode: "scanning",
    config: createDefaultAppConfig(browserTimeZone),
    providerCredentialStatus: "unchecked",
    discovery: defaultChatDiscovery,
    selectedChats: [],
    decisionEvidence: emptyDecisionEvidenceReport,
    ...emptySyncResultCounts
  }
}

export function reduceAppShellState(state: AppShellState, event: AppShellEvent): AppShellState {
  switch (event.type) {
    case "pause":
      return {
        ...state,
        mode: "paused",
        errorMessage: undefined
      }
    case "resume":
      return {
        ...state,
        mode: "scanning",
        errorMessage: undefined
      }
    case "fail":
      return {
        ...state,
        mode: "error",
        errorMessage: event.message
      }
    case "clearError":
      return {
        ...state,
        mode: "scanning",
        errorMessage: undefined
      }
    case "updateConfig":
      return {
        ...state,
        config: event.config
      }
    case "setProviderCredentialStatus":
      return {
        ...state,
        providerCredentialStatus: event.providerCredentialStatus
      }
    case "updateChatDiscovery":
      return {
        ...state,
        discovery: event.discovery,
        selectedChats: reconcileSelectedChats(event.discovery, state.selectedChats)
      }
    case "toggleSelectedChat":
      return {
        ...state,
        selectedChats: toggleSelectedChatSelection(
          state.selectedChats,
          state.discovery,
          event.chatId,
          event.chat
        )
      }
    case "setChatBackfillPrompt":
      return {
        ...state,
        selectedChats: setChatBackfillPromptSelection(
          state.selectedChats,
          event.chatId,
          event.enabled
        )
      }
    case "clearDecisionEvidence":
      return {
        ...state,
        decisionEvidence: emptyDecisionEvidenceReport
      }
    case "syncCompleted":
      return {
        ...state,
        ...syncResultCountsFrom(event),
        decisionEvidence: event.decisionEvidence ?? state.decisionEvidence
      }
    default:
      return assertNever(event)
  }
}

export function applyNativeAppShellState(
  state: AppShellState,
  nativeState: NativeAppShellState
): AppShellState {
  switch (nativeState.mode) {
    case "scanning":
      return reduceAppShellState(state, { type: "resume" })
    case "paused":
      return reduceAppShellState(state, { type: "pause" })
    case "error":
      return reduceAppShellState(state, {
        type: "fail",
        message: nativeState.errorMessage ?? "Morrow needs attention."
      })
    default:
      return assertNever(nativeState.mode)
  }
}

export function loadAppShellState(storage: ShellStorage): AppShellState {
  const persisted = loadPersistedAppShellState(storage)
  if (persisted === undefined) {
    return createDefaultAppShellState()
  }
  const reloaded: AppShellState = {
    ...persisted,
    providerCredentialStatus: "unchecked",
    decisionEvidence: emptyDecisionEvidenceReport
  }
  return reloaded.mode === "error" ? { ...reloaded, mode: "scanning", errorMessage: undefined } : reloaded
}

export function saveAppShellState(state: AppShellState, storage: ShellStorage): void {
  savePersistedAppShellState(state, storage)
}

function assertNever(value: never): never {
  throw new UnhandledAppShellVariantError(String(value))
}
