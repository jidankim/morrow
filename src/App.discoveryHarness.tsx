import { render } from "@testing-library/react"
import { vi } from "vitest"
import { App } from "./App"
import {
  APP_SHELL_STATE_KEY,
  createDefaultAppShellState,
  type ChatDiscovery
} from "./domain/appShell"
import type { SyncSchedulerState } from "./domain/syncScheduler"
import type { RuntimeIdentity } from "./tauriBridge"

export type NativeDiscoveryReportForTest =
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

type NativeReadyChatForTest = Extract<NativeDiscoveryReportForTest, { readonly status: "ready" }>["chats"][number]
type ChatFixtureForTest = Pick<NativeReadyChatForTest, "participantCount" | "participantIds" | "latestActivityTimestamp"> & {
  readonly id: string
  readonly label: string
}

export type NativePreviewReportForTest = {
  readonly chats: readonly {
    readonly chatId: string
    readonly preview: string
  }[]
}

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

export const rediscoveredChat = {
  id: "messages-chat-11111111111111111111111111111111",
  label: "Updated alpha",
  participantCount: 3,
  participantIds: [
    "messages-participant-66666666666666666666666666666666",
    "messages-participant-77777777777777777777777777777777",
    "messages-participant-88888888888888888888888888888888"
  ],
  latestActivityTimestamp: 1_783_001_000
} as const

export const fixtureReferenceTimezone = "Asia/Seoul"
export const privatePreviewText = "private clinic visit"

export const nativeChatFromFixture = (chat: ChatFixtureForTest): NativeReadyChatForTest => ({
  chatId: chat.id,
  displayLabel: chat.label,
  participantCount: chat.participantCount,
  participantIds: chat.participantIds,
  latestActivityTimestamp: chat.latestActivityTimestamp
})

export const nativeReadyReport = { status: "ready", chats: [nativeChatFromFixture(discoveredChat)] } as const
export const changedNativeReadyReport = { status: "ready", chats: [nativeChatFromFixture(rediscoveredChat)] } as const

const bridgeMock = vi.hoisted(() => {
  const defaultSchedulerState: SyncSchedulerState = {
    enabled: false,
    interval_seconds: 1_800,
    status: "disabled",
    retry_attempt: 0,
    updated_at: 1_783_000_000
  }
  return {
    getState: vi.fn(async () => undefined),
    setShellState: vi.fn(async () => undefined),
    getRuntimeIdentity: vi.fn<() => Promise<RuntimeIdentity | undefined>>(async () => undefined),
    subscribeAppState: vi.fn(async () => vi.fn()),
    subscribeMenuCommand: vi.fn(async () => vi.fn()),
    reconcileNow: vi.fn(async () => undefined),
    scanSelectedChats: vi.fn(async () => ({
      pendingProposalCount: 12,
      feedbackLabelCount: 9,
      featureSnapshotCount: 4,
      latestEvalStatus: "passed"
    })),
    loadDecisionEvidence: vi.fn(async () => ({
      items: [],
      skippedTraceLineCount: 0,
      latestEvalStatus: "never_run"
    })),
    getSyncSchedulerState: vi.fn(async () => defaultSchedulerState),
    setSyncSchedulerState: vi.fn(async (state: SyncSchedulerState) => state),
    loadMessagesChatPreviews: vi.fn(async (): Promise<NativePreviewReportForTest> => ({
      chats: [{ chatId: discoveredChat.id, preview: privatePreviewText }]
    })),
    checkProviderAuth: vi.fn(async () => ({
      status: "loggedInUsingChatGpt",
      ready: true,
      commandSurface: "codex login status",
      commandOutputRedacted: true,
      diagnostic: "Codex CLI ChatGPT session is ready."
    })),
    storeMorrowToken: vi.fn(async () => ({ storageSurface: "keychainBridge", stored: true, deleted: false })),
    readMorrowToken: vi.fn(async () => ({ storageSurface: "keychainBridge", present: true })),
    deleteMorrowToken: vi.fn(async () => ({ storageSurface: "keychainBridge", stored: false, deleted: true })),
    discoverMessagesChats: vi.fn(async (): Promise<NativeDiscoveryReportForTest> => nativeReadyReport),
    openPrivacySettings: vi.fn(async () => ({ pane: "fullDiskAccess", opened: true })),
    deleteMorrowData: vi.fn(async () => undefined),
    recordCrashLog: vi.fn(async () => ({ stored: true }))
  }
})
export { bridgeMock }

vi.mock("./tauriBridge", () => ({
  MORROW_KEYCHAIN_SERVICE: "com.morrow.desktop.token",
  MORROW_TOKEN_KIND: "morrow-owned-token",
  MORROW_PROVIDER_TOKEN_KIND: "morrow-openai-provider-api-key",
  createNativeShellBridge: () => bridgeMock
}))

export function resetDiscoveryAppTestState(): void {
  window.localStorage.clear()
  window.location.hash = ""
  bridgeMock.setShellState.mockClear()
  bridgeMock.getRuntimeIdentity.mockClear()
  bridgeMock.getRuntimeIdentity.mockResolvedValue(undefined)
  bridgeMock.reconcileNow.mockClear()
  bridgeMock.scanSelectedChats.mockClear()
  bridgeMock.loadDecisionEvidence.mockClear()
  bridgeMock.getSyncSchedulerState.mockClear()
  bridgeMock.setSyncSchedulerState.mockClear()
  bridgeMock.loadMessagesChatPreviews.mockClear()
  bridgeMock.loadMessagesChatPreviews.mockResolvedValue({
    chats: [{ chatId: discoveredChat.id, preview: privatePreviewText }]
  })
  bridgeMock.readMorrowToken.mockClear()
  bridgeMock.discoverMessagesChats.mockClear()
  bridgeMock.discoverMessagesChats.mockResolvedValue(nativeReadyReport)
  bridgeMock.openPrivacySettings.mockClear()
}

export function renderDiscoveryApp(): void {
  render(<App />)
}

export function seedSelectedChat(): void {
  const initial = createDefaultAppShellState()
  const ready = {
    ...initial,
    config: {
      ...initial.config,
      permissionsGranted: true,
      referenceTimezone: fixtureReferenceTimezone
    },
    discovery: { status: "ready", chats: [discoveredChat] } satisfies ChatDiscovery,
    selectedChats: [{ ...discoveredChat, backfillPromptEnabled: true }]
  }
  window.localStorage.setItem(APP_SHELL_STATE_KEY, JSON.stringify(ready))
}
