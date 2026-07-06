import { vi } from "vitest"
import { APP_SHELL_STATE_KEY, createDefaultAppShellState } from "./domain/appShell"
import type { ProviderUsageLoadRequest, ProviderUsageReport } from "./domain/providerUsage"

type NativeStateForTest = {
  readonly mode: "scanning" | "paused" | "error"
  readonly errorMessage?: string
  readonly onboardingComplete: boolean
  readonly pendingProposalCount: number
  readonly syncNowRunning: boolean
  readonly automaticSyncEnabled: boolean
  readonly automaticSyncStatusLabel: "Off" | "On" | "Cooling Down" | "Needs Action"
  readonly automaticSyncDetail: string
}

type NativeMenuCommandForTest =
  | "sync-now"
  | "open-settings"
  | "open-calendar"
  | "open-reminders"
  | "toggle-automatic-sync"

type NativeSyncSchedulerStateForTest = {
  readonly enabled: boolean
  readonly interval_seconds: number
  readonly status: "disabled" | "scheduled" | "running" | "cooldown" | "blocked"
  readonly last_started_at?: number | undefined
  readonly last_finished_at?: number | undefined
  readonly next_run_at?: number | undefined
  readonly next_eligible_at?: number | undefined
  readonly last_result?: "success" | "retryable_failure" | "blocked" | "manual_disabled" | undefined
  readonly retry_attempt: number
  readonly last_reason?: string | undefined
  readonly updated_at: number
}

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
  let schedulerState: NativeSyncSchedulerStateForTest | undefined = {
    enabled: false,
    interval_seconds: 1_800,
    status: "disabled",
    retry_attempt: 0,
    updated_at: 1_783_000_000
  }
  let providerUsageReport: ProviderUsageReport | undefined
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
      return {
        pendingProposalCount: 12,
        createdCandidateCount: 0,
        quietLogCount: 0,
        createdExternalProposalCount: 0,
        failedExternalProposalCount: 0,
        feedbackLabelCount: 0,
        featureSnapshotCount: 0,
        latestEvalStatus: "never_run",
        createdCandidateIds: []
      }
    }),
    loadDecisionEvidence: vi.fn(async () => ({
      items: [],
      skippedTraceLineCount: 0,
      latestEvalStatus: "never_run"
    })),
    loadProviderUsage: vi.fn(async (_request?: ProviderUsageLoadRequest) => providerUsageReport),
    getSyncSchedulerState: vi.fn(async () => schedulerState),
    setSyncSchedulerState: vi.fn(async (state: NativeSyncSchedulerStateForTest) => {
      schedulerState = state
      return state
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
    getSchedulerState: (): NativeSyncSchedulerStateForTest | undefined => schedulerState,
    setSchedulerState: (state: NativeSyncSchedulerStateForTest | undefined): void => {
      schedulerState = state
    },
    setProviderUsageReport: (report: ProviderUsageReport | undefined): void => {
      providerUsageReport = report
    },
    getProviderUsageReport: (): ProviderUsageReport | undefined => providerUsageReport,
    resetSyncCalls: (): void => {
      syncCalls.length = 0
    },
    resetSchedulerState: (): void => {
      schedulerState = {
        enabled: false,
        interval_seconds: 1_800,
        status: "disabled",
        retry_attempt: 0,
        updated_at: 1_783_000_000
      }
      providerUsageReport = undefined
    }
  }
})

export const bridgeMock = hoistedBridgeMock

export const resetAppShellBridgeTestHarness = (): void => {
  vi.useRealTimers()
  window.localStorage.clear()
  window.location.hash = ""
  bridgeMock.getState.mockClear()
  bridgeMock.setShellState.mockClear()
  bridgeMock.subscribeAppState.mockClear()
  bridgeMock.subscribeMenuCommand.mockClear()
  bridgeMock.reconcileNow.mockClear()
  bridgeMock.scanSelectedChats.mockClear()
  bridgeMock.loadDecisionEvidence.mockClear()
  bridgeMock.loadProviderUsage.mockClear()
  bridgeMock.loadProviderUsage.mockImplementation(
    async (_request?: ProviderUsageLoadRequest) => bridgeMock.getProviderUsageReport()
  )
  bridgeMock.getSyncSchedulerState.mockClear()
  bridgeMock.setSyncSchedulerState.mockClear()
  bridgeMock.resetSchedulerState()
  bridgeMock.readMorrowToken.mockClear()
  bridgeMock.discoverMessagesChats.mockClear()
  bridgeMock.discoverMessagesChats.mockResolvedValue(nativeReadyReport)
  bridgeMock.openPrivacySettings.mockClear()
  bridgeMock.resetSyncCalls()
}

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
    loadDecisionEvidence: bridgeMock.loadDecisionEvidence,
    loadProviderUsage: bridgeMock.loadProviderUsage,
    getSyncSchedulerState: bridgeMock.getSyncSchedulerState,
    setSyncSchedulerState: bridgeMock.setSyncSchedulerState,
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
