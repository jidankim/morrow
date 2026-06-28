import { act, fireEvent, render, screen, waitFor } from "@testing-library/react"
import { beforeEach, describe, expect, it, vi } from "vitest"
import { App } from "./App"
import { APP_SHELL_STATE_KEY, createDefaultAppShellState } from "./domain/appShell"

type CrashLogRequestForTest = {
  readonly message: string
}

const bridgeMock = vi.hoisted(() => ({
  getState: vi.fn(async () => undefined),
  setShellState: vi.fn(async () => undefined),
  subscribeAppState: vi.fn(async () => vi.fn()),
  subscribeMenuCommand: vi.fn(async () => vi.fn()),
  reconcileNow: vi.fn(async () => undefined),
  scanSelectedChats: vi.fn(async () => ({ pendingProposalCount: 12 })),
  storeMorrowToken: vi.fn(async () => ({ storageSurface: "keychainBridge", stored: true, deleted: false })),
  readMorrowToken: vi.fn(async () => ({
    storageSurface: "keychainBridge",
    present: true,
    token: "redacted-provider-token-for-ui"
  })),
  deleteMorrowToken: vi.fn(async () => ({ storageSurface: "keychainBridge", stored: false, deleted: true })),
  discoverMessagesChats: vi.fn(async () => ({
    status: "ready",
    chats: [
      {
        chatId: "messages-chat-11111111111111111111111111111111",
        displayLabel: "Chat alpha",
        participantCount: 2,
        participantIds: ["messages-participant-11111111111111111111111111111111", "messages-participant-22222222222222222222222222222222"],
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

vi.mock("./tauriBridge", () => ({
  MORROW_KEYCHAIN_SERVICE: "com.morrow.desktop.token",
  MORROW_PROVIDER_TOKEN_KIND: "morrow-openai-provider-api-key",
  createNativeShellBridge: () => bridgeMock
}))

const seedReadyState = (): void => {
  const initial = createDefaultAppShellState()
  const chat = {
    id: "messages-chat-11111111111111111111111111111111",
    label: "Chat alpha",
    participantCount: 2,
    participantIds: ["messages-participant-11111111111111111111111111111111", "messages-participant-22222222222222222222222222222222"],
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

describe("App privacy controls", () => {
  beforeEach(() => {
    window.localStorage.clear()
    window.location.hash = ""
    bridgeMock.scanSelectedChats.mockClear()
    bridgeMock.storeMorrowToken.mockClear()
    bridgeMock.readMorrowToken.mockClear()
    bridgeMock.deleteMorrowToken.mockClear()
    bridgeMock.discoverMessagesChats.mockClear()
    bridgeMock.openPrivacySettings.mockClear()
    bridgeMock.deleteMorrowData.mockClear()
    bridgeMock.recordCrashLog.mockClear()
  })

  it("passes hidden source-excerpt setting through the scan request", async () => {
    seedReadyState()
    render(<App />)

    act(() => {
      window.location.hash = "#settings"
      window.dispatchEvent(new HashChangeEvent("hashchange"))
    })
    fireEvent.click(screen.getByLabelText("Show short source excerpts in future proposal notes"))
    act(() => {
      window.location.hash = "#status"
      window.dispatchEvent(new HashChangeEvent("hashchange"))
    })
    await waitFor(() => expect(screen.getByRole("button", { name: "Sync Now" })).toBeEnabled())
    await waitFor(() => expect(screen.getByText("Messages chat")).toBeInTheDocument())
    expect(document.body).not.toHaveTextContent("messages-chat-33333333333333333333333333333333")
    expect(document.body).not.toHaveTextContent("messages-participant-33333333333333333333333333333333")
    expect(document.body).not.toHaveTextContent("private clinic visit")
    fireEvent.click(screen.getByRole("button", { name: "Sync Now" }))

    await waitFor(() => expect(bridgeMock.scanSelectedChats).toHaveBeenCalledOnce())
    expect(bridgeMock.scanSelectedChats).toHaveBeenCalledWith({
      selectedChatIds: ["messages-chat-11111111111111111111111111111111"],
      selectedChats: [
        {
          id: "messages-chat-11111111111111111111111111111111",
          label: "Chat alpha",
          participantCount: 2,
          participantIds: ["messages-participant-11111111111111111111111111111111", "messages-participant-22222222222222222222222222222222"],
          latestActivityTimestamp: 1_783_000_000
        }
      ],
      referenceTimezone: "Asia/Seoul",
      backfillPromptChatIds: ["messages-chat-11111111111111111111111111111111"],
      sourceExcerptsEnabled: false,
      capPolicy: {
        mode: "refillForPending",
        maxVisible: 10,
        pendingCount: 0
      }
    })
  })

  it("opens macOS privacy panes for Messages Calendar and Reminders access", async () => {
    render(<App />)

    act(() => {
      window.location.hash = "#settings"
      window.dispatchEvent(new HashChangeEvent("hashchange"))
    })

    expect(screen.getByText("For QA from Terminal, add Terminal to Full Disk Access too.")).toBeInTheDocument()

    fireEvent.click(screen.getByRole("button", { name: "Open Full Disk Access" }))
    fireEvent.click(screen.getByRole("button", { name: "Open Calendar access" }))
    fireEvent.click(screen.getByRole("button", { name: "Open Reminders access" }))

    await waitFor(() => expect(bridgeMock.openPrivacySettings).toHaveBeenCalledTimes(3))
    expect(bridgeMock.openPrivacySettings).toHaveBeenNthCalledWith(1, { pane: "fullDiskAccess" })
    expect(bridgeMock.openPrivacySettings).toHaveBeenNthCalledWith(2, { pane: "calendar" })
    expect(bridgeMock.openPrivacySettings).toHaveBeenNthCalledWith(3, { pane: "reminders" })
  })

})
