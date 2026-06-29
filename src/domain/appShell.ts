import { z } from "zod"
import { UnhandledAppShellVariantError } from "./appShellErrors"
import {
  chatDiscoverySchema,
  defaultChatDiscovery,
  reconcileSelectedChats,
  selectedChatSchema,
  setChatBackfillPromptSelection,
  toggleSelectedChatSelection,
  type ChatDiscovery,
  type ChatId,
  type DiscoveredChat,
  type SelectedChat
} from "./chatDiscovery"
import {
  appConfigSchema,
  createDefaultAppConfig,
  type AppConfig
} from "./appConfig"
import {
  emptySyncResultCounts,
  syncResultCountsFrom,
  syncResultCountsSchema,
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

export const APP_SHELL_STATE_KEY = "morrow.appShellState.v1"

export type AppMode = "scanning" | "paused" | "error"
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
}

export type NativeAppShellState = {
  readonly mode: AppMode
  readonly errorMessage?: string | undefined
  readonly onboardingComplete: boolean
  readonly pendingProposalCount: number
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
  | ({ readonly type: "syncCompleted" } & PartialSyncResultCounts)

export type ShellStorage = {
  readonly get: (key: string) => string | undefined
  readonly set: (key: string, value: string) => void
}

const appModeSchema = z.union([
  z.literal("scanning"),
  z.literal("paused"),
  z.literal("error")
])
const providerCredentialStatusSchema = z.union([
  z.literal("unchecked"),
  z.literal("configured"),
  z.literal("missing")
])

const appShellStateSchema = z
  .object({
    mode: appModeSchema,
    errorMessage: z.preprocess((value) => (value === null ? undefined : value), z.string().min(1).optional()),
    config: appConfigSchema,
    providerCredentialStatus: providerCredentialStatusSchema.default("unchecked"),
    discovery: chatDiscoverySchema.default({ status: "unverified", chats: [] }),
    selectedChats: z.array(selectedChatSchema)
  })
  .and(syncResultCountsSchema)

export function createBrowserShellStorage(storage: Storage): ShellStorage {
  return {
    get: (key) => storage.getItem(key) ?? undefined,
    set: (key, value) => storage.setItem(key, value)
  }
}

export function createDefaultAppShellState(
  browserTimeZone = Intl.DateTimeFormat().resolvedOptions().timeZone
): AppShellState {
  return {
    mode: "scanning",
    config: createDefaultAppConfig(browserTimeZone),
    providerCredentialStatus: "unchecked",
    discovery: defaultChatDiscovery,
    selectedChats: [],
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
    case "syncCompleted":
      return {
        ...state,
        ...syncResultCountsFrom(event)
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
  const stored = storage.get(APP_SHELL_STATE_KEY)
  if (stored === undefined) {
    return createDefaultAppShellState()
  }

  const parsedJson: unknown = JSON.parse(stored)
  const reloaded: AppShellState = { ...appShellStateSchema.parse(parsedJson), providerCredentialStatus: "unchecked" }
  return reloaded.mode === "error" ? { ...reloaded, mode: "scanning", errorMessage: undefined } : reloaded
}

export function saveAppShellState(state: AppShellState, storage: ShellStorage): void {
  storage.set(
    APP_SHELL_STATE_KEY,
    JSON.stringify(appShellStateSchema.parse({ ...state, providerCredentialStatus: "unchecked" }))
  )
}

function assertNever(value: never): never {
  throw new UnhandledAppShellVariantError(String(value))
}
