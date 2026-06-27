import { z } from "zod"
import { isValidTimeZone } from "./timeZone"

export const APP_SHELL_STATE_KEY = "morrow.appShellState.v1"

export type AppMode = "scanning" | "paused" | "error"
export type MenuStatusKind = "setup-needed" | AppMode
export type CalendarSource = "apple-calendar"
export type ChatId = string

export type ChatOption = { readonly id: ChatId; readonly label: string }

export type SelectedChat = ChatOption & { readonly backfillPromptEnabled: boolean }

export type AppConfig = {
  readonly referenceTimezone: string
  readonly calendarSource: CalendarSource
  readonly permissionsGranted: boolean
  readonly launchAtLogin: boolean
  readonly sourceExcerptsEnabled: boolean
  readonly firstProposalGuidanceEnabled: boolean
  readonly telemetryEnabled: false
  readonly crashLogExcerptsEnabled: false
}

export type AppShellState = {
  readonly mode: AppMode
  readonly errorMessage?: string | undefined
  readonly config: AppConfig
  readonly selectedChats: readonly SelectedChat[]
  readonly pendingProposalCount: number
}

export type NativeAppShellState = { readonly mode: AppMode; readonly errorMessage?: string | undefined; readonly onboardingComplete: boolean; readonly pendingProposalCount: number }

export type AppShellEvent =
  | { readonly type: "pause" }
  | { readonly type: "resume" }
  | { readonly type: "fail"; readonly message: string }
  | { readonly type: "clearError" }
  | { readonly type: "updateConfig"; readonly config: AppConfig }
  | { readonly type: "toggleSelectedChat"; readonly chatId: ChatId }
  | { readonly type: "setChatBackfillPrompt"; readonly chatId: ChatId; readonly enabled: boolean }
  | { readonly type: "syncCompleted"; readonly pendingProposalCount: number }

export type MenuModel = {
  readonly statusKind: MenuStatusKind
  readonly statusLabel: "Setup needed" | "Scanning" | "Paused" | "Error"
  readonly detail: string
  readonly pauseResumeLabel: "Pause" | "Resume"
  readonly syncNowEnabled: boolean
  readonly pendingProposalLabel: string
}

export type ShellStorage = {
  readonly get: (key: string) => string | undefined
  readonly set: (key: string, value: string) => void
}

export const AVAILABLE_CHAT_OPTIONS: readonly ChatOption[] = [] as const

const appModeSchema = z.union([z.literal("scanning"), z.literal("paused"), z.literal("error")])
const calendarSourceSchema = z.literal("apple-calendar")
const chatIdSchema = z.string().min(1).max(240)

const timeZoneSchema = z.string().refine((value) => isValidTimeZone(value), {
  message: "Reference timezone must be a supported IANA timezone."
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

const selectedChatSchema = z.object({ id: chatIdSchema, label: z.string().min(1).max(80), backfillPromptEnabled: z.boolean() })

const appShellStateSchema = z.object({
  mode: appModeSchema,
  errorMessage: z.string().min(1).optional(),
  config: appConfigSchema,
  selectedChats: z.array(selectedChatSchema),
  pendingProposalCount: z.number().int().min(0)
})

export function createBrowserShellStorage(storage: Storage): ShellStorage {
  return {
    get: (key) => storage.getItem(key) ?? undefined,
    set: (key, value) => storage.setItem(key, value)
  }
}

export function createDefaultAppShellState(): AppShellState {
  return {
    mode: "scanning",
    config: {
      referenceTimezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
      calendarSource: "apple-calendar",
      permissionsGranted: false,
      launchAtLogin: false,
      sourceExcerptsEnabled: true,
      firstProposalGuidanceEnabled: true,
      telemetryEnabled: false,
      crashLogExcerptsEnabled: false
    },
    selectedChats: [],
    pendingProposalCount: 0
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
    case "toggleSelectedChat":
      return toggleSelectedChat(state, event.chatId)
    case "setChatBackfillPrompt":
      return setChatBackfillPrompt(state, event.chatId, event.enabled)
    case "syncCompleted":
      return { ...state, pendingProposalCount: event.pendingProposalCount }
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
  return appShellStateSchema.parse(parsedJson)
}

export function saveAppShellState(state: AppShellState, storage: ShellStorage): void {
  storage.set(APP_SHELL_STATE_KEY, JSON.stringify(appShellStateSchema.parse(state)))
}

export function getMenuModel(state: AppShellState): MenuModel {
  const onboardingComplete = isOnboardingComplete(state)
  const pendingProposalLabel = formatPendingProposalCount(state.pendingProposalCount)
  switch (state.mode) {
    case "scanning":
      if (!onboardingComplete) {
        return {
          statusKind: "setup-needed", statusLabel: "Setup needed",
          detail: "Complete setup and choose chats before Sync Now can scan.",
          pauseResumeLabel: "Pause", syncNowEnabled: false, pendingProposalLabel
        }
      }
      return {
        statusKind: "scanning", statusLabel: "Scanning",
        detail: "Ready to reconcile calendars, then scan selected chats.",
        pauseResumeLabel: "Pause", syncNowEnabled: true, pendingProposalLabel
      }
    case "paused":
      return {
        statusKind: "paused", statusLabel: "Paused",
        detail: "Scanning is paused on this Mac.",
        pauseResumeLabel: "Resume", syncNowEnabled: false, pendingProposalLabel
      }
    case "error":
      return {
        statusKind: "error", statusLabel: "Error",
        detail: state.errorMessage ?? "Morrow needs attention.",
        pauseResumeLabel: "Resume", syncNowEnabled: onboardingComplete, pendingProposalLabel
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
  if (!state.config.permissionsGranted) {
    warnings.push("Complete required permissions before scanning.")
  }
  if (state.selectedChats.length === 0) {
    warnings.push("Select at least one chat before scanning.")
  }
  return warnings
}

export function formatPendingProposalCount(count: number): string {
  return count >= 9 ? "9+" : String(count)
}

function toggleSelectedChat(state: AppShellState, chatId: ChatId): AppShellState {
  const existing = state.selectedChats.find((chat) => chat.id === chatId)
  if (existing !== undefined) {
    return {
      ...state,
      selectedChats: state.selectedChats.filter((chat) => chat.id !== chatId)
    }
  }

  const option = AVAILABLE_CHAT_OPTIONS.find((chat) => chat.id === chatId)
  if (option === undefined) {
    return state
  }

  return {
    ...state,
    selectedChats: [...state.selectedChats, { ...option, backfillPromptEnabled: true }]
  }
}

function setChatBackfillPrompt(
  state: AppShellState,
  chatId: ChatId,
  enabled: boolean
): AppShellState {
  return {
    ...state,
    selectedChats: state.selectedChats.map((chat) =>
      chat.id === chatId ? { ...chat, backfillPromptEnabled: enabled } : chat
    )
  }
}

function assertNever(value: never): never {
  throw new UnhandledAppShellVariantError(String(value))
}

class UnhandledAppShellVariantError extends Error {
  readonly renderedValue: string

  constructor(renderedValue: string) {
    super(`Unhandled app shell variant: ${renderedValue}`)
    this.name = "UnhandledAppShellVariantError"
    this.renderedValue = renderedValue
  }
}
