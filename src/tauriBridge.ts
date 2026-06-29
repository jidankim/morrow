import { invoke } from "@tauri-apps/api/core"
import { listen } from "@tauri-apps/api/event"
import { z } from "zod"
import type { MenuModel, NativeAppShellState } from "./domain/appShell"
import { syncResultCountsSchema, type SyncResultCounts } from "./domain/syncResultCounts"
import {
  parseMessagesDiscoveryReport,
  parseSyncScanRequest,
  type MessagesDiscoveryReport,
  type SyncScanRequest
} from "./messagesDiscoveryBridge"
import {
  parseNativePermissionStatuses,
  type NativePermissionStatus
} from "./nativePermissionBridge"
import {
  parseCrashLogReceipt,
  parseMorrowDataDeleteReceipt,
  parsePrivacySettingsReceipt,
  type CrashLogReceipt,
  type CrashLogRequest,
  type MorrowDataDeleteReceipt,
  type MorrowDataDeleteRequest,
  type PrivacySettingsPane,
  type PrivacySettingsReceipt,
  type PrivacySettingsRequest
} from "./nativePrivacyBridge"
import {
  parseCodexProviderAuthReadiness,
  type CodexProviderAuthReadiness
} from "./providerAuthBridge"

export type { NativePermissionStatus } from "./nativePermissionBridge"
export { parseMessagesDiscoveryReport } from "./messagesDiscoveryBridge"
export type { MessagesDiscoveryReport, SyncScanRequest } from "./messagesDiscoveryBridge"
export type {
  CrashLogReceipt,
  CrashLogRequest,
  MorrowDataDeleteReceipt,
  MorrowDataDeleteRequest,
  PrivacySettingsPane,
  PrivacySettingsReceipt,
  PrivacySettingsRequest
} from "./nativePrivacyBridge"
export type { CodexAuthStatus, CodexProviderAuthReadiness } from "./providerAuthBridge"

const nativeAppShellStateSchema = z.object({
  mode: z.union([z.literal("scanning"), z.literal("paused"), z.literal("error")]),
  errorMessage: z.string().min(1).optional(),
  onboardingComplete: z.boolean(),
  pendingProposalCount: z.number().int().min(0)
})

const tokenStorageSurfaceSchema = z.literal("keychainBridge")

const tokenCommandReceiptSchema = z.object({
  storageSurface: tokenStorageSurfaceSchema,
  stored: z.boolean(),
  deleted: z.boolean()
})

const tokenReadResponseSchema = z.object({
  storageSurface: tokenStorageSurfaceSchema,
  present: z.boolean(),
  token: z.string().min(1).optional()
})

export const MORROW_KEYCHAIN_SERVICE = "com.morrow.desktop.token"
export const MORROW_TOKEN_KIND = "morrow-owned-token"
export const MORROW_PROVIDER_TOKEN_KIND = "morrow-openai-provider-api-key"

export type MorrowTokenKind = typeof MORROW_TOKEN_KIND | typeof MORROW_PROVIDER_TOKEN_KIND

export type MorrowTokenLookupRequest = {
  readonly service: typeof MORROW_KEYCHAIN_SERVICE
  readonly tokenKind: MorrowTokenKind
}
export type MorrowTokenWriteRequest = MorrowTokenLookupRequest & {
  readonly token: string
}
export type MorrowTokenCommandReceipt = z.infer<typeof tokenCommandReceiptSchema>
export type MorrowTokenReadResponse = z.infer<typeof tokenReadResponseSchema>
export type SyncScanResult = SyncResultCounts
export type NativeMenuCommand = "sync-now" | "open-settings" | "open-calendar" | "open-reminders"

export type NativeShellBridge = {
  readonly getState: () => Promise<NativeAppShellState | undefined>
  readonly setShellState: (state: NativeAppShellState) => Promise<MenuModel | undefined>
  readonly subscribeAppState: (
    onState: (state: NativeAppShellState) => void
  ) => Promise<(() => void) | undefined>
  readonly subscribeMenuCommand: (
    onCommand: (command: NativeMenuCommand) => void
  ) => Promise<(() => void) | undefined>
  readonly reconcileNow: () => Promise<void>
  readonly scanSelectedChats: (request: SyncScanRequest) => Promise<SyncScanResult | undefined>
  readonly discoverMessagesChats: () => Promise<MessagesDiscoveryReport | undefined>
  readonly getPermissionStatuses: () => Promise<readonly NativePermissionStatus[] | undefined>
  readonly storeMorrowToken: (
    request: MorrowTokenWriteRequest
  ) => Promise<MorrowTokenCommandReceipt | undefined>
  readonly readMorrowToken: (
    request: MorrowTokenLookupRequest
  ) => Promise<MorrowTokenReadResponse | undefined>
  readonly deleteMorrowToken: (
    request: MorrowTokenLookupRequest
  ) => Promise<MorrowTokenCommandReceipt | undefined>
  readonly checkProviderAuth: () => Promise<CodexProviderAuthReadiness | undefined>
  readonly deleteMorrowData: (
    request: MorrowDataDeleteRequest
  ) => Promise<MorrowDataDeleteReceipt | undefined>
  readonly openPrivacySettings: (
    request: PrivacySettingsRequest
  ) => Promise<PrivacySettingsReceipt | undefined>
  readonly recordCrashLog: (request: CrashLogRequest) => Promise<CrashLogReceipt | undefined>
}

const isTauriRuntime = (): boolean => "__TAURI_INTERNALS__" in window

const nativeMenuCommandEvents: readonly {
  readonly eventName: string
  readonly command: NativeMenuCommand
}[] = [
  { eventName: "morrow://sync-now", command: "sync-now" },
  { eventName: "morrow://open-settings", command: "open-settings" },
  { eventName: "morrow://open-calendar", command: "open-calendar" },
  { eventName: "morrow://open-reminders", command: "open-reminders" }
] as const

function parseNativeAppShellState(value: unknown): NativeAppShellState {
  return nativeAppShellStateSchema.parse(value)
}

function parseSyncScanResult(value: unknown): SyncScanResult {
  return syncResultCountsSchema.parse(value)
}

function parseTokenCommandReceipt(value: unknown): MorrowTokenCommandReceipt {
  return tokenCommandReceiptSchema.parse(value)
}

function parseTokenReadResponse(value: unknown): MorrowTokenReadResponse {
  return tokenReadResponseSchema.parse(value)
}

export function createNativeShellBridge(): NativeShellBridge {
  return {
    getState: async () => {
      if (!isTauriRuntime()) {
        return undefined
      }
      const state = await invoke<unknown>("get_app_state")
      return parseNativeAppShellState(state)
    },
    setShellState: async (state) => {
      if (!isTauriRuntime()) {
        return undefined
      }
      return invoke<MenuModel>("set_app_shell_state", { shellState: parseNativeAppShellState(state) })
    },
    subscribeAppState: async (onState) => {
      if (!isTauriRuntime()) {
        return undefined
      }
      return listen<unknown>("morrow://app-state", (event) => {
        onState(parseNativeAppShellState(event.payload))
      })
    },
    subscribeMenuCommand: async (onCommand) => {
      if (!isTauriRuntime()) {
        return undefined
      }
      const unsubscribers = await Promise.all(
        nativeMenuCommandEvents.map(({ eventName, command }) =>
          listen(eventName, () => onCommand(command))
        )
      )
      return () => {
        for (const unsubscribe of unsubscribers) {
          unsubscribe()
        }
      }
    },
    reconcileNow: async () => {
      if (!isTauriRuntime()) {
        return undefined
      }
      await invoke("reconcile_now")
    },
    scanSelectedChats: async (request) => {
      if (!isTauriRuntime()) {
        return undefined
      }
      const result = await invoke<unknown>("scan_selected_chats", { request: parseSyncScanRequest(request) })
      return parseSyncScanResult(result)
    },
    discoverMessagesChats: async () => {
      if (!isTauriRuntime()) {
        return undefined
      }
      const report = await invoke<unknown>("discover_messages_chats")
      return parseMessagesDiscoveryReport(report)
    },
    getPermissionStatuses: async () => {
      if (!isTauriRuntime()) {
        return undefined
      }
      const statuses = await invoke<unknown>("get_native_permission_statuses")
      return parseNativePermissionStatuses(statuses)
    },
    storeMorrowToken: async (request) => {
      if (!isTauriRuntime()) {
        return undefined
      }
      const receipt = await invoke<unknown>("store_morrow_token", { request })
      return parseTokenCommandReceipt(receipt)
    },
    readMorrowToken: async (request) => {
      if (!isTauriRuntime()) {
        return undefined
      }
      const response = await invoke<unknown>("read_morrow_token", { request })
      return parseTokenReadResponse(response)
    },
    deleteMorrowToken: async (request) => {
      if (!isTauriRuntime()) {
        return undefined
      }
      const receipt = await invoke<unknown>("delete_morrow_token", { request })
      return parseTokenCommandReceipt(receipt)
    },
    checkProviderAuth: async () => {
      if (!isTauriRuntime()) {
        return undefined
      }
      const readiness = await invoke<unknown>("check_provider_auth")
      return parseCodexProviderAuthReadiness(readiness)
    },
    deleteMorrowData: async (request) => {
      if (!isTauriRuntime()) {
        return undefined
      }
      const receipt = await invoke<unknown>("delete_morrow_data", { request })
      return parseMorrowDataDeleteReceipt(receipt)
    },
    openPrivacySettings: async (request) => {
      if (!isTauriRuntime()) {
        return undefined
      }
      const receipt = await invoke<unknown>("open_privacy_settings", { request })
      return parsePrivacySettingsReceipt(receipt)
    },
    recordCrashLog: async (request) => {
      if (!isTauriRuntime()) {
        return undefined
      }
      const receipt = await invoke<unknown>("record_crash_log", { request })
      return parseCrashLogReceipt(receipt)
    }
  }
}
