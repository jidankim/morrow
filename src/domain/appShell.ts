import { z } from "zod"
import { UnhandledAppShellVariantError } from "./appShellErrors"
import {
  chatDiscoverySchema,
  chatDiscoveryWarning,
  defaultChatDiscovery,
  reconcileSelectedChats,
  selectedChatsAreVerified,
  selectedChatSchema,
  setChatBackfillPromptSelection,
  toggleSelectedChatSelection,
  type ChatDiscovery,
  type ChatId,
  type DiscoveredChat,
  type SelectedChat
} from "./chatDiscovery"
import { emptySyncResultCounts, formatSyncResultEvidence, syncResultCountsFrom, syncResultCountsSchema, type PartialSyncResultCounts, type SyncResultCounts } from "./syncResultCounts"
import { isSupportedReferenceTimeZone, normalizeReferenceTimeZone } from "./timeZone"
export { getSyncReadinessItems, type SyncReadinessItem, type SyncReadinessItemId, type SyncReadinessItemStatus, type SyncReadinessOptions } from "./syncReadiness"

export const APP_SHELL_STATE_KEY = "morrow.appShellState.v1"

export type AppMode = "scanning" | "paused" | "error"
export type MenuStatusKind = "setup-needed" | AppMode
export type CalendarSource = "apple-calendar"
export type ProviderCredentialStatus = "unchecked" | "configured" | "missing"
export type { ChatDiscovery, ChatDiscoveryStatus, ChatId, ChatOption, DiscoveredChat, ParticipantId, SelectedChat } from "./chatDiscovery"

export type AppConfig = {
  readonly referenceTimezone: string; readonly calendarSource: CalendarSource
  readonly permissionsGranted: boolean; readonly launchAtLogin: boolean
  readonly sourceExcerptsEnabled: boolean; readonly firstProposalGuidanceEnabled: boolean
  readonly telemetryEnabled: false; readonly crashLogExcerptsEnabled: false
}

export type AppShellState = SyncResultCounts & {
  readonly mode: AppMode; readonly errorMessage?: string | undefined
  readonly config: AppConfig; readonly providerCredentialStatus: ProviderCredentialStatus
  readonly discovery: ChatDiscovery; readonly selectedChats: readonly SelectedChat[]
}

export type NativeAppShellState = { readonly mode: AppMode; readonly errorMessage?: string | undefined; readonly onboardingComplete: boolean; readonly pendingProposalCount: number }

export type AppShellEvent =
  | { readonly type: "pause" }
  | { readonly type: "resume" }
  | { readonly type: "fail"; readonly message: string }
  | { readonly type: "clearError" }
  | { readonly type: "updateConfig"; readonly config: AppConfig }
  | { readonly type: "setProviderCredentialStatus"; readonly providerCredentialStatus: ProviderCredentialStatus }
  | { readonly type: "updateChatDiscovery"; readonly discovery: ChatDiscovery }
  | { readonly type: "toggleSelectedChat"; readonly chatId: ChatId; readonly chat?: DiscoveredChat }
  | { readonly type: "setChatBackfillPrompt"; readonly chatId: ChatId; readonly enabled: boolean }
  | ({ readonly type: "syncCompleted" } & PartialSyncResultCounts)

export type MenuModel = {
  readonly statusKind: MenuStatusKind
  readonly statusLabel: "Setup needed" | "Scanning" | "Paused" | "Error"
  readonly detail: string
  readonly pauseResumeLabel: "Pause" | "Resume"
  readonly syncNowEnabled: boolean
  readonly pendingProposalLabel: string
  readonly syncResultLabel: string
}

export type ShellStorage = { readonly get: (key: string) => string | undefined; readonly set: (key: string, value: string) => void }

const appModeSchema = z.union([z.literal("scanning"), z.literal("paused"), z.literal("error")])
const calendarSourceSchema = z.literal("apple-calendar")
const providerCredentialStatusSchema = z.union([z.literal("unchecked"), z.literal("configured"), z.literal("missing")])
const optionalErrorMessageSchema = z.preprocess(
  (value) => (value === null ? undefined : value),
  z.string().min(1).optional()
)
const providerCredentialWarnings = {
  unchecked: "Morrow is checking Codex provider readiness before scanning.",
  configured: undefined,
  missing: "Finish Codex CLI setup in Settings before scanning."
} as const satisfies Record<ProviderCredentialStatus, string | undefined>

const timeZoneSchema = z.string().refine((value) => isSupportedReferenceTimeZone(value), {
  message: "Reference timezone must be supported by Morrow Calendar replay."
})

const appConfigSchema = z.object({
  referenceTimezone: timeZoneSchema,
  calendarSource: calendarSourceSchema,
  permissionsGranted: z.boolean(),
  launchAtLogin: z.boolean(),
  sourceExcerptsEnabled: z.boolean(),
  firstProposalGuidanceEnabled: z.boolean(),
  telemetryEnabled: z.literal(false).default(false),
  crashLogExcerptsEnabled: z.literal(false).default(false)
})

const appShellStateSchema = z.object({
  mode: appModeSchema,
  errorMessage: optionalErrorMessageSchema,
  config: appConfigSchema,
  providerCredentialStatus: providerCredentialStatusSchema.default("unchecked"),
  discovery: chatDiscoverySchema.default({ status: "unverified", chats: [] }),
  selectedChats: z.array(selectedChatSchema)
}).and(syncResultCountsSchema)

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
    config: {
      referenceTimezone: normalizeReferenceTimeZone(browserTimeZone), calendarSource: "apple-calendar",
      permissionsGranted: false, launchAtLogin: false,
      sourceExcerptsEnabled: true, firstProposalGuidanceEnabled: true,
      telemetryEnabled: false,
      crashLogExcerptsEnabled: false
    },
    providerCredentialStatus: "unchecked",
    discovery: defaultChatDiscovery,
    selectedChats: [],
    ...emptySyncResultCounts
  }
}

export function reduceAppShellState(state: AppShellState, event: AppShellEvent): AppShellState {
  switch (event.type) {
    case "pause":
      return { ...state, mode: "paused", errorMessage: undefined }
    case "resume":
      return { ...state, mode: "scanning", errorMessage: undefined }
    case "fail":
      return { ...state, mode: "error", errorMessage: event.message }
    case "clearError":
      return { ...state, mode: "scanning", errorMessage: undefined }
    case "updateConfig":
      return { ...state, config: event.config }
    case "setProviderCredentialStatus":
      return { ...state, providerCredentialStatus: event.providerCredentialStatus }
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
      return { ...state, ...syncResultCountsFrom(event) }
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
  storage.set(APP_SHELL_STATE_KEY, JSON.stringify(appShellStateSchema.parse({ ...state, providerCredentialStatus: "unchecked" })))
}

export function getMenuModel(state: AppShellState): MenuModel {
  const onboardingComplete = isOnboardingComplete(state)
  const pendingProposalLabel = formatPendingProposalCount(state.pendingProposalCount)
  const syncResultLabel = formatSyncResultEvidence(state)
  switch (state.mode) {
    case "scanning":
      if (!onboardingComplete) {
        return {
          statusKind: "setup-needed", statusLabel: "Setup needed",
          detail: "Complete setup and choose chats before Sync Now can scan.",
          pauseResumeLabel: "Pause", syncNowEnabled: false, pendingProposalLabel, syncResultLabel
        }
      }
      return {
        statusKind: "scanning", statusLabel: "Scanning",
        detail: "Ready to reconcile calendars, then scan selected chats.",
        pauseResumeLabel: "Pause", syncNowEnabled: true, pendingProposalLabel, syncResultLabel
      }
    case "paused":
      return {
        statusKind: "paused", statusLabel: "Paused",
        detail: "Scanning is paused on this Mac.",
        pauseResumeLabel: "Resume", syncNowEnabled: false, pendingProposalLabel, syncResultLabel
      }
    case "error":
      return {
        statusKind: "error", statusLabel: "Error",
        detail: state.errorMessage ?? "Morrow needs attention.",
        pauseResumeLabel: "Resume", syncNowEnabled: onboardingComplete, pendingProposalLabel, syncResultLabel
      }
    default:
      return assertNever(state.mode)
  }
}

export function isSyncNowEnabled(state: AppShellState): boolean {
  return getMenuModel(state).syncNowEnabled
}

export function isOnboardingComplete(state: AppShellState): boolean {
  return getOnboardingWarnings(state).length === 0
}

export function getOnboardingWarnings(state: AppShellState): readonly string[] {
  const warnings: string[] = []
  const discoveryWarning = chatDiscoveryWarning(state.discovery)
  if (discoveryWarning !== undefined) warnings.push(discoveryWarning)
  if (state.selectedChats.length === 0) {
    warnings.push("Select at least one chat before scanning.")
  } else if (!selectedChatsAreVerified(state.discovery, state.selectedChats)) {
    warnings.push("Refresh chat discovery before scanning selected chats.")
  }
  const providerCredentialWarning = providerCredentialWarnings[state.providerCredentialStatus]
  if (providerCredentialWarning !== undefined) warnings.push(providerCredentialWarning)
  return warnings
}

export function formatPendingProposalCount(count: number): string {
  return count >= 9 ? "9+" : String(count)
}

function assertNever(value: never): never {
  throw new UnhandledAppShellVariantError(String(value))
}
