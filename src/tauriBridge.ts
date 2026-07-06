import { invoke } from "@tauri-apps/api/core"
import type { DecisionEvidenceLoadRequest, DecisionEvidenceReport } from "./domain/decisionEvidence"
import type { MenuModel, NativeAppShellState } from "./domain/appShell"
import {
  type MessagesChatPreviewReport,
  type MessagesChatPreviewRequest,
  type MessagesDiscoveryReport,
  type SyncScanRequest
} from "./messagesDiscoveryBridge"
import {
  discoverMessagesChatsInTauri,
  loadMessagesChatPreviewsInTauri,
  scanSelectedChatsInTauri,
  type SyncScanResult
} from "./messagesTauriCommands"
import {
  getNativeAppShellState,
  setNativeAppShellState,
  subscribeNativeAppShellState,
  subscribeNativeMenuCommand,
  type NativeMenuCommand
} from "./nativeAppShellBridge"
import { parseNativePermissionStatuses, type NativePermissionStatus } from "./nativePermissionBridge"
import { parseRuntimeIdentity, type RuntimeIdentity } from "./nativeRuntimeBridge"
import { loadDecisionEvidenceInTauri } from "./nativeDecisionEvidenceBridge"
import {
  parseCrashLogReceipt,
  parseMorrowDataDeleteReceipt,
  parsePrivacySettingsReceipt,
  type CrashLogReceipt,
  type CrashLogRequest,
  type MorrowDataDeleteReceipt,
  type MorrowDataDeleteRequest,
  type PrivacySettingsReceipt,
  type PrivacySettingsRequest
} from "./nativePrivacyBridge"
import type { ProviderUsageLoadRequest, ProviderUsageReport, ProviderUsageWindowKey } from "./domain/providerUsage"
import { loadProviderUsageInTauri } from "./nativeProviderUsageBridge"
import type { CodexProviderAuthReadiness } from "./providerAuthBridge"
import type { CodexCliInstallReceipt, CodexLoginLaunchReceipt } from "./providerSetupBridge"
import {
  checkProviderAuthInTauri,
  installCodexCliInTauri,
  startCodexLoginInTauri
} from "./nativeProviderSetupBridge"
import {
  getSyncSchedulerStateInTauri,
  setSyncSchedulerStateInTauri,
  type SyncSchedulerState
} from "./syncSchedulerTauriBridge"
import {
  parseTokenCommandReceipt,
  parseTokenReadResponse,
  type MorrowTokenCommandReceipt,
  type MorrowTokenLookupRequest,
  type MorrowTokenReadResponse,
  type MorrowTokenWriteRequest
} from "./nativeTokenBridge"

export type { NativePermissionStatus } from "./nativePermissionBridge"
export type { RuntimeIdentity } from "./nativeRuntimeBridge"
export { parseMessagesDiscoveryReport } from "./messagesDiscoveryBridge"
export type { NativeMenuCommand } from "./nativeAppShellBridge"
export type {
  MessagesChatPreviewReport,
  MessagesChatPreviewRequest,
  MessagesDiscoveryReport,
  SyncScanRequest
} from "./messagesDiscoveryBridge"
export type { SyncScanResult } from "./messagesTauriCommands"
export type { DecisionEvidenceLoadRequest, DecisionEvidenceReport } from "./domain/decisionEvidence"
export type {
  ProviderUsageLoadRequest,
  ProviderUsageReport,
  ProviderUsageWindowKey
} from "./domain/providerUsage"
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
export type {
  CodexCliInstallReceipt,
  CodexLoginLaunchReceipt,
  CodexLoginLaunchStatus,
  CodexSetupActionStatus
} from "./providerSetupBridge"
export type { SyncSchedulerState } from "./syncSchedulerTauriBridge"
export {
  MORROW_KEYCHAIN_SERVICE,
  MORROW_PROVIDER_TOKEN_KIND,
  MORROW_TOKEN_KIND
} from "./nativeTokenBridge"
export type {
  MorrowTokenCommandReceipt,
  MorrowTokenKind,
  MorrowTokenLookupRequest,
  MorrowTokenReadResponse,
  MorrowTokenWriteRequest
} from "./nativeTokenBridge"

export type NativeShellBridge = {
  readonly getState: () => Promise<NativeAppShellState | undefined>
  readonly setShellState: (state: NativeAppShellState) => Promise<MenuModel | undefined>
  readonly getRuntimeIdentity: () => Promise<RuntimeIdentity | undefined>
  readonly subscribeAppState: (onState: (state: NativeAppShellState) => void) => Promise<(() => void) | undefined>
  readonly subscribeMenuCommand: (onCommand: (command: NativeMenuCommand) => void) => Promise<(() => void) | undefined>
  readonly reconcileNow: () => Promise<void>
  readonly scanSelectedChats: (request: SyncScanRequest) => Promise<SyncScanResult | undefined>
  readonly loadDecisionEvidence: (request: DecisionEvidenceLoadRequest) => Promise<DecisionEvidenceReport | undefined>
  readonly loadProviderUsage: (request?: ProviderUsageLoadRequest) => Promise<ProviderUsageReport | undefined>
  readonly discoverMessagesChats: () => Promise<MessagesDiscoveryReport | undefined>
  readonly loadMessagesChatPreviews: (request: MessagesChatPreviewRequest) => Promise<MessagesChatPreviewReport | undefined>
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
  readonly installCodexCli: () => Promise<CodexCliInstallReceipt | undefined>
  readonly startCodexLogin: () => Promise<CodexLoginLaunchReceipt | undefined>
  readonly getSyncSchedulerState: () => Promise<SyncSchedulerState | undefined>
  readonly setSyncSchedulerState: (
    state: SyncSchedulerState
  ) => Promise<SyncSchedulerState | undefined>
  readonly deleteMorrowData: (
    request: MorrowDataDeleteRequest
  ) => Promise<MorrowDataDeleteReceipt | undefined>
  readonly openPrivacySettings: (
    request: PrivacySettingsRequest
  ) => Promise<PrivacySettingsReceipt | undefined>
  readonly recordCrashLog: (request: CrashLogRequest) => Promise<CrashLogReceipt | undefined>
}

const isTauriRuntime = (): boolean => "__TAURI_INTERNALS__" in window

export function createNativeShellBridge(): NativeShellBridge {
  return {
    getState: async () => {
      if (!isTauriRuntime()) {
        return undefined
      }
      return getNativeAppShellState()
    },
    setShellState: async (state) => {
      if (!isTauriRuntime()) {
        return undefined
      }
      return setNativeAppShellState(state)
    },
    getRuntimeIdentity: async () =>
      isTauriRuntime() ? parseRuntimeIdentity(await invoke<unknown>("get_runtime_identity")) : undefined,
    subscribeAppState: async (onState) => {
      if (!isTauriRuntime()) {
        return undefined
      }
      return subscribeNativeAppShellState(onState)
    },
    subscribeMenuCommand: async (onCommand) => {
      if (!isTauriRuntime()) {
        return undefined
      }
      return subscribeNativeMenuCommand(onCommand)
    },
    reconcileNow: async () => {
      if (!isTauriRuntime()) {
        return undefined
      }
      await invoke("reconcile_now")
    },
    scanSelectedChats: (request) =>
      isTauriRuntime() ? scanSelectedChatsInTauri(request) : Promise.resolve(undefined),
    loadDecisionEvidence: (request) =>
      isTauriRuntime() ? loadDecisionEvidenceInTauri(request) : Promise.resolve(undefined),
    loadProviderUsage: (request) =>
      isTauriRuntime() ? loadProviderUsageInTauri(request) : Promise.resolve(undefined),
    discoverMessagesChats: () =>
      isTauriRuntime() ? discoverMessagesChatsInTauri() : Promise.resolve(undefined),
    loadMessagesChatPreviews: (request) =>
      isTauriRuntime() ? loadMessagesChatPreviewsInTauri(request) : Promise.resolve(undefined),
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
      const tokenReadResult = await invoke<unknown>("read_morrow_token", { request })
      return parseTokenReadResponse(tokenReadResult)
    },
    deleteMorrowToken: async (request) => {
      if (!isTauriRuntime()) {
        return undefined
      }
      const receipt = await invoke<unknown>("delete_morrow_token", { request })
      return parseTokenCommandReceipt(receipt)
    },
    checkProviderAuth: () => (isTauriRuntime() ? checkProviderAuthInTauri() : Promise.resolve(undefined)),
    installCodexCli: () => (isTauriRuntime() ? installCodexCliInTauri() : Promise.resolve(undefined)),
    startCodexLogin: () => (isTauriRuntime() ? startCodexLoginInTauri() : Promise.resolve(undefined)),
    getSyncSchedulerState: () =>
      isTauriRuntime() ? getSyncSchedulerStateInTauri() : Promise.resolve(undefined),
    setSyncSchedulerState: (state) =>
      isTauriRuntime() ? setSyncSchedulerStateInTauri(state) : Promise.resolve(undefined),
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
