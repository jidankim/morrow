import { fireEvent, render, screen, waitFor } from "@testing-library/react"
import { beforeEach, describe, expect, it, vi } from "vitest"
import { App } from "./App"
import { APP_SHELL_STATE_KEY, createDefaultAppShellState } from "./domain/appShell"

const discoveredChat = {
  id: "messages-chat-11111111111111111111111111111111",
  label: "Chat alpha",
  participantCount: 2,
  participantIds: [
    "messages-participant-11111111111111111111111111111111",
    "messages-participant-22222222222222222222222222222222"
  ],
  latestActivityTimestamp: 1_783_000_000
} as const

const nativeReadyReport = {
  status: "ready",
  chats: [
    {
      chatId: discoveredChat.id,
      displayLabel: discoveredChat.label,
      participantCount: discoveredChat.participantCount,
      participantIds: discoveredChat.participantIds,
      latestActivityTimestamp: discoveredChat.latestActivityTimestamp
    }
  ]
} as const

const bridgeMock = vi.hoisted(() => {
  const defaultSchedulerState = {
    enabled: false,
    interval_seconds: 1_800,
    status: "disabled",
    retry_attempt: 0,
    updated_at: 1_783_000_000
  } as const
  return {
  getState: vi.fn(async () => undefined),
  setShellState: vi.fn(async () => undefined),
  getRuntimeIdentity: vi.fn(async () => undefined),
  subscribeAppState: vi.fn(async () => vi.fn()),
  subscribeMenuCommand: vi.fn(async () => vi.fn()),
  reconcileNow: vi.fn(async () => undefined),
  scanSelectedChats: vi.fn(async () => ({
    pendingProposalCount: 5,
    createdCandidateCount: 4,
    quietLogCount: 2,
    createdExternalProposalCount: 3,
    failedExternalProposalCount: 1
  })),
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
  readMorrowToken: vi.fn(async () => ({
    storageSurface: "keychainBridge",
    present: true
  })),
  deleteMorrowToken: vi.fn(async () => ({
    storageSurface: "keychainBridge",
    stored: false,
    deleted: true
  })),
  discoverMessagesChats: vi.fn(async () => nativeReadyReport),
  getSyncSchedulerState: vi.fn(async () => defaultSchedulerState),
  setSyncSchedulerState: vi.fn(async () => defaultSchedulerState),
  openPrivacySettings: vi.fn(async () => ({ pane: "fullDiskAccess", opened: true })),
  recordCrashLog: vi.fn(async () => ({ path: "morrow-crash.log", written: true }))
  }
})

vi.mock("./tauriBridge", () => ({
  MORROW_KEYCHAIN_SERVICE: "com.morrow.desktop.token",
  MORROW_TOKEN_KIND: "morrow-owned-token",
  MORROW_PROVIDER_TOKEN_KIND: "morrow-openai-provider-api-key",
  createNativeShellBridge: () => bridgeMock
}))

function seedReadyState(): void {
  const initial = createDefaultAppShellState()
  window.localStorage.setItem(
    APP_SHELL_STATE_KEY,
    JSON.stringify({
      ...initial,
      discovery: { status: "ready", chats: [discoveredChat] },
      selectedChats: [{ ...discoveredChat, backfillPromptEnabled: true }]
    })
  )
}

describe("App Sync Now result counts", () => {
  beforeEach(() => {
    window.localStorage.clear()
    window.location.hash = ""
    bridgeMock.getState.mockClear()
    bridgeMock.setShellState.mockClear()
    bridgeMock.subscribeAppState.mockClear()
    bridgeMock.subscribeMenuCommand.mockClear()
    bridgeMock.reconcileNow.mockClear()
    bridgeMock.scanSelectedChats.mockClear()
    bridgeMock.getSyncSchedulerState.mockClear()
    bridgeMock.setSyncSchedulerState.mockClear()
    bridgeMock.readMorrowToken.mockClear()
    bridgeMock.readMorrowToken.mockResolvedValue({
      storageSurface: "keychainBridge",
      present: true
    })
    bridgeMock.discoverMessagesChats.mockClear()
    bridgeMock.discoverMessagesChats.mockResolvedValue(nativeReadyReport)
  })

  it("shows candidate, quiet log, and external proposal evidence after Sync Now", async () => {
    // Given
    seedReadyState()
    render(<App />)
    const syncButton = screen.getByRole("button", { name: "Sync Now" })
    await waitFor(() => expect(syncButton).toBeEnabled())

    // When
    fireEvent.click(syncButton)

    // Then
    await waitFor(() => expect(screen.getByTestId("pending-count")).toHaveTextContent("5"))
    expect(screen.getByTestId("sync-result-counts")).toHaveTextContent(
      "Candidates 4 · Quiet logs 2 · External proposals 3 created / 1 failed"
    )
  })
})
