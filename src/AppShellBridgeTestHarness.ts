import { vi } from "vitest"
import { APP_SHELL_STATE_KEY, createDefaultAppShellState } from "./domain/appShell"

type NativeStateForTest = {
  readonly mode: "scanning" | "paused" | "error"
  readonly errorMessage?: string
  readonly onboardingComplete: boolean
  readonly pendingProposalCount: number
}

type NativeMenuCommandForTest = "sync-now" | "open-settings" | "open-calendar" | "open-reminders"

type NativeDiscoveryReportForTest =
  | {
      readonly status: "ready"
      readonly chats: readonly {
        readonly chatId: string
        readonly displayLabel: string
        readonly participantCount: number
        readonly participantIds: readonly string[]
        readonly latestActivityTimestamp: number
      }[]
    }
  | { readonly status: "empty" | "permissionDenied" | "unavailable"; readonly chats: readonly [] }

export const discoveredChat = {
  id: "messages-chat-11111111111111111111111111111111",
  label: "Chat alpha",
  participantCount: 2,
  participantIds: [
    "messages-participant-11111111111111111111111111111111",
    "messages-participant-22222222222222222222222222222222"
  ],
  latestActivityTimestamp: 1_783_000_000
} as const

export const nativeReadyReport = {
  status: "ready",
  chats: [
    {
      chatId: "messages-chat-11111111111111111111111111111111",
      displayLabel: "Chat alpha",
      participantCount: 2,
      participantIds: [
        "messages-participant-11111111111111111111111111111111",
        "messages-participant-22222222222222222222222222222222"
      ],
      latestActivityTimestamp: 1_783_000_000
    }
  ]
} as const satisfies NativeDiscoveryReportForTest

const hoistedBridgeMock = vi.hoisted(() => {
  let nativeStateListener: ((state: NativeStateForTest) => void) | undefined
  let nativeMenuCommandListener: ((command: NativeMenuCommandForTest) => void) | undefined
  const syncCalls: string[] = []
  return {
    getState: vi.fn(async () => undefined),
    setShellState: vi.fn(async () => undefined),
    getRuntimeIdentity: vi.fn(async () => undefined),
    subscribeAppState: vi.fn(async (listener: (state: NativeStateForTest) => void) => {
      nativeStateListener = listener
      return vi.fn()
    }),
    subscribeMenuCommand: vi.fn(async (listener: (command: NativeMenuCommandForTest) => void) => {
      nativeMenuCommandListener = listener
      return vi.fn()
    }),
    reconcileNow: vi.fn(async () => {
      syncCalls.push("reconcile")
    }),
    scanSelectedChats: vi.fn(async () => {
      syncCalls.push("scan")
      return { pendingProposalCount: 12 }
    }),
    checkProviderAuth: vi.fn(async () => ({
      status: "loggedInUsingChatGpt",
      ready: true,
      commandSurface: "codex login status",
      commandOutputRedacted: true,
      diagnostic: "Codex CLI ChatGPT session is ready."
    })),
    storeMorrowToken: vi.fn(async () => ({
      storageSurface: "keychainBridge",
      stored: true,
      deleted: false
    })),
    readMorrowToken: vi.fn(async () => ({ storageSurface: "keychainBridge", present: true })),
    deleteMorrowToken: vi.fn(async () => ({
      storageSurface: "keychainBridge",
      stored: false,
      deleted: true
    })),
    discoverMessagesChats: vi.fn(async (): Promise<NativeDiscoveryReportForTest> => {
      return nativeReadyReport
    }),
    openPrivacySettings: vi.fn(async () => ({ pane: "fullDiskAccess", opened: true })),
    deleteMorrowData: vi.fn(async () => undefined),
    recordCrashLog: vi.fn(async () => ({ stored: true })),
    emitNativeState: (state: NativeStateForTest): void => {
      nativeStateListener?.(state)
    },
    emitMenuCommand: (command: NativeMenuCommandForTest): void => {
      nativeMenuCommandListener?.(command)
    },
    getSyncCalls: (): readonly string[] => syncCalls,
    resetSyncCalls: (): void => {
      syncCalls.length = 0
    }
  }
})

export const bridgeMock = hoistedBridgeMock

vi.mock("./tauriBridge", () => ({
  MORROW_KEYCHAIN_SERVICE: "com.morrow.desktop.token",
  MORROW_TOKEN_KIND: "morrow-owned-token",
  MORROW_PROVIDER_TOKEN_KIND: "morrow-openai-provider-api-key",
  createNativeShellBridge: () => ({
    getState: bridgeMock.getState,
    setShellState: bridgeMock.setShellState,
    getRuntimeIdentity: bridgeMock.getRuntimeIdentity,
    subscribeAppState: bridgeMock.subscribeAppState,
    subscribeMenuCommand: bridgeMock.subscribeMenuCommand,
    reconcileNow: bridgeMock.reconcileNow,
    scanSelectedChats: bridgeMock.scanSelectedChats,
    checkProviderAuth: bridgeMock.checkProviderAuth,
    storeMorrowToken: bridgeMock.storeMorrowToken,
    readMorrowToken: bridgeMock.readMorrowToken,
    deleteMorrowToken: bridgeMock.deleteMorrowToken,
    discoverMessagesChats: bridgeMock.discoverMessagesChats,
    openPrivacySettings: bridgeMock.openPrivacySettings,
    deleteMorrowData: bridgeMock.deleteMorrowData,
    recordCrashLog: bridgeMock.recordCrashLog
  })
}))

export const seedReadyState = (): void => {
  const initial = createDefaultAppShellState()
  window.localStorage.setItem(
    APP_SHELL_STATE_KEY,
    JSON.stringify({
      ...initial,
      config: { ...initial.config, permissionsGranted: false },
      discovery: { status: "ready", chats: [discoveredChat] },
      selectedChats: [{ ...discoveredChat, backfillPromptEnabled: true }]
    })
  )
}
