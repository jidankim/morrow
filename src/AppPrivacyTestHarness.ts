import { act } from "@testing-library/react"
import { vi } from "vitest"
import { APP_SHELL_STATE_KEY, createDefaultAppShellState } from "./domain/appShell"

type CrashLogRequestForTest = {
  readonly message: string
}

const hoistedMocks = vi.hoisted(() => ({
  getState: vi.fn(async () => undefined),
  setShellState: vi.fn(async () => undefined),
  subscribeAppState: vi.fn(async () => vi.fn()),
  subscribeMenuCommand: vi.fn(async () => vi.fn()),
  reconcileNow: vi.fn(async () => undefined),
  scanSelectedChats: vi.fn(async () => ({ pendingProposalCount: 12 })),
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
  discoverMessagesChats: vi.fn(async () => ({
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
      },
      {
        chatId: "messages-chat-33333333333333333333333333333333",
        displayLabel: "Messages chat",
        participantCount: 1,
        participantIds: ["messages-participant-33333333333333333333333333333333"],
        latestActivityTimestamp: 1_783_000_100,
        latestMessageBody: "private clinic visit"
      }
    ]
  })),
  openPrivacySettings: vi.fn(async () => ({ pane: "fullDiskAccess", opened: true })),
  deleteMorrowData: vi.fn(async (): Promise<import("./tauriBridge").MorrowDataDeleteReceipt> => ({
    storageSurface: "morrowStore",
    databaseDeleted: true,
    approvedExternalItemsDeleted: false,
    diagnosticsArtifactsDeleted: true,
    providerOAuthDeleteRequested: true,
    providerOAuthDeleted: true,
    providerOAuthDeleteFailed: false,
    providerCredentialDeletes: [
      {
        tokenKind: "morrow-owned-token",
        deleteRequested: true,
        deleted: true,
        failed: false
      },
      {
        tokenKind: "morrow-openai-provider-api-key",
        deleteRequested: true,
        deleted: true,
        failed: false
      }
    ],
    cleanupPlan: {
      proposedItems: "completed",
      emptyProposalContainers: "skippedByUser",
      proposedCalendarItemsDeleted: 1,
      proposedReminderItemsDeleted: 1
    }
  })),
  recordCrashLog: vi.fn(async (_request: CrashLogRequestForTest) => ({ stored: true }))
}))

export const bridgeMock = hoistedMocks

vi.mock("./tauriBridge", () => ({
  MORROW_KEYCHAIN_SERVICE: "com.morrow.desktop.token",
  MORROW_TOKEN_KIND: "morrow-owned-token",
  MORROW_PROVIDER_TOKEN_KIND: "morrow-openai-provider-api-key",
  createNativeShellBridge: () => bridgeMock
}))

export const seedReadyState = (): void => {
  const initial = createDefaultAppShellState()
  const chat = {
    id: "messages-chat-11111111111111111111111111111111",
    label: "Chat alpha",
    participantCount: 2,
    participantIds: [
      "messages-participant-11111111111111111111111111111111",
      "messages-participant-22222222222222222222222222222222"
    ],
    latestActivityTimestamp: 1_783_000_000
  } as const
  window.localStorage.setItem(
    APP_SHELL_STATE_KEY,
    JSON.stringify({
      ...initial,
      config: { ...initial.config, permissionsGranted: true },
      discovery: { status: "ready", chats: [chat] },
      selectedChats: [{ ...chat, backfillPromptEnabled: true }]
    })
  )
}

export const openAppRoute = (hash: "#settings" | "#status"): void => {
  act(() => {
    window.location.hash = hash
    window.dispatchEvent(new HashChangeEvent("hashchange"))
  })
}
