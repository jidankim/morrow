import { fireEvent, render, screen, waitFor, act } from "@testing-library/react"
import { beforeEach, describe, expect, it, vi } from "vitest"
import { App } from "./App"
import { APP_SHELL_STATE_KEY, createDefaultAppShellState } from "./domain/appShell"

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

const discoveredChat = {
  id: "messages-chat-11111111111111111111111111111111",
  label: "Chat alpha",
  participantCount: 2,
  participantIds: ["messages-participant-11111111111111111111111111111111", "messages-participant-22222222222222222222222222222222"],
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
} as const satisfies NativeDiscoveryReportForTest

const rediscoveredChat = {
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

const changedNativeReadyReport = {
  status: "ready",
  chats: [
    {
      chatId: rediscoveredChat.id,
      displayLabel: rediscoveredChat.label,
      participantCount: rediscoveredChat.participantCount,
      participantIds: rediscoveredChat.participantIds,
      latestActivityTimestamp: rediscoveredChat.latestActivityTimestamp
    }
  ]
} as const satisfies NativeDiscoveryReportForTest

const bridgeMock = vi.hoisted(() => ({
  getState: vi.fn(async () => undefined),
  setShellState: vi.fn(async () => undefined),
  subscribeAppState: vi.fn(async () => vi.fn()),
  subscribeMenuCommand: vi.fn(async () => vi.fn()),
  reconcileNow: vi.fn(async () => undefined),
  scanSelectedChats: vi.fn(async () => ({ pendingProposalCount: 12 })),
  discoverMessagesChats: vi.fn(async (): Promise<NativeDiscoveryReportForTest> => nativeReadyReport),
  openPrivacySettings: vi.fn(async () => ({ pane: "fullDiskAccess", opened: true })),
  deleteMorrowData: vi.fn(async () => undefined),
  recordCrashLog: vi.fn(async () => ({ stored: true }))
}))

vi.mock("./tauriBridge", () => ({
  MORROW_KEYCHAIN_SERVICE: "com.morrow.desktop.token",
  MORROW_TOKEN_KIND: "morrow-owned-token",
  createNativeShellBridge: () => bridgeMock
}))

const seedSelectedChat = (): void => {
  const initial = createDefaultAppShellState()
  window.localStorage.setItem(
    APP_SHELL_STATE_KEY,
    JSON.stringify({
      ...initial,
      config: { ...initial.config, permissionsGranted: true },
      discovery: { status: "ready", chats: [discoveredChat] },
      selectedChats: [{ ...discoveredChat, backfillPromptEnabled: true }]
    })
  )
}

describe("App Messages chat discovery onboarding", () => {
  beforeEach(() => {
    window.localStorage.clear()
    window.location.hash = ""
    bridgeMock.setShellState.mockClear()
    bridgeMock.reconcileNow.mockClear()
    bridgeMock.scanSelectedChats.mockClear()
    bridgeMock.discoverMessagesChats.mockClear()
    bridgeMock.discoverMessagesChats.mockResolvedValue(nativeReadyReport)
    bridgeMock.openPrivacySettings.mockClear()
  })

  it("blocks Sync Now until required setup and at least one chat are selected", async () => {
    render(<App />)

    await screen.findByText("Onboarding required")
    expect(await screen.findByRole("checkbox", { name: /Chat alpha/ })).toBeInTheDocument()
    expect(screen.getByText("Select at least one chat before scanning.")).toBeInTheDocument()
    expect(screen.getByTestId("status-label")).toHaveTextContent("Setup needed")
    expect(screen.getByRole("button", { name: "Sync Now" })).toBeDisabled()
    expect(bridgeMock.reconcileNow).not.toHaveBeenCalled()
    expect(bridgeMock.scanSelectedChats).not.toHaveBeenCalled()
  })

  it("shows loading then an empty discovery state without enabling Sync Now", async () => {
    let resolveDiscovery: (report: NativeDiscoveryReportForTest) => void = () => undefined
    bridgeMock.discoverMessagesChats.mockReturnValueOnce(
      new Promise<NativeDiscoveryReportForTest>((resolve) => {
        resolveDiscovery = resolve
      })
    )
    render(<App />)

    expect(await screen.findByText("Loading Messages chats...")).toBeInTheDocument()
    await act(async () => resolveDiscovery({ status: "empty", chats: [] }))

    expect(screen.getByText("No Messages chats found.")).toBeInTheDocument()
    expect(screen.getByRole("button", { name: "Sync Now" })).toBeDisabled()
  })

  it("offers Full Disk Access recovery when discovery is denied", async () => {
    bridgeMock.discoverMessagesChats.mockResolvedValueOnce({ status: "permissionDenied", chats: [] })
    render(<App />)

    expect(await screen.findByText("Messages access denied.")).toBeInTheDocument()
    fireEvent.click(screen.getByRole("button", { name: "Open Full Disk Access" }))

    await waitFor(() =>
      expect(bridgeMock.openPrivacySettings).toHaveBeenCalledWith({ pane: "fullDiskAccess" })
    )
    expect(screen.getByRole("button", { name: "Sync Now" })).toBeDisabled()
  })

  it("retries discovery from unavailable and empty states", async () => {
    bridgeMock.discoverMessagesChats
      .mockResolvedValueOnce({ status: "unavailable", chats: [] })
      .mockResolvedValueOnce({ status: "empty", chats: [] })
      .mockResolvedValueOnce(nativeReadyReport)
    render(<App />)

    expect(await screen.findByText("Messages discovery unavailable.")).toBeInTheDocument()
    fireEvent.click(screen.getByRole("button", { name: "Retry chat discovery" }))
    expect(await screen.findByText("No Messages chats found.")).toBeInTheDocument()
    fireEvent.click(screen.getByRole("button", { name: "Retry chat discovery" }))

    expect(await screen.findByRole("checkbox", { name: /Chat alpha/ })).toBeInTheDocument()
    expect(bridgeMock.discoverMessagesChats).toHaveBeenCalledTimes(3)
  })

  it("enables then disables Sync Now as a real discovered chat is selected and deselected", async () => {
    render(<App />)

    const chatCheckbox = await screen.findByRole("checkbox", { name: /Chat alpha/ })
    fireEvent.click(screen.getByLabelText("Required permissions complete"))
    fireEvent.click(chatCheckbox)
    await waitFor(() => expect(screen.getByRole("button", { name: "Sync Now" })).toBeEnabled())

    fireEvent.click(chatCheckbox)

    await waitFor(() => expect(screen.getByRole("button", { name: "Sync Now" })).toBeDisabled())
  })

  it("uses rediscovered same-id chat metadata in scan requests", async () => {
    seedSelectedChat()
    bridgeMock.discoverMessagesChats.mockResolvedValueOnce(changedNativeReadyReport)
    render(<App />)

    expect(await screen.findByRole("checkbox", { name: /Updated alpha/ })).toBeChecked()
    const syncButton = screen.getByRole("button", { name: "Sync Now" })
    await waitFor(() => expect(syncButton).toBeEnabled())
    fireEvent.click(syncButton)

    await waitFor(() => expect(bridgeMock.scanSelectedChats).toHaveBeenCalledOnce())
    expect(bridgeMock.scanSelectedChats).toHaveBeenCalledWith({
      selectedChatIds: ["messages-chat-11111111111111111111111111111111"],
      selectedChats: [rediscoveredChat],
      referenceTimezone: "Asia/Seoul",
      backfillPromptChatIds: ["messages-chat-11111111111111111111111111111111"],
      sourceExcerptsEnabled: true,
      capPolicy: { mode: "refillForPending", maxVisible: 10, pendingCount: 0 }
    })
  })

  it("keeps a stale persisted selected chat disabled until rediscovered", async () => {
    seedSelectedChat()
    bridgeMock.discoverMessagesChats
      .mockResolvedValueOnce({ status: "empty", chats: [] })
      .mockResolvedValueOnce(nativeReadyReport)
    render(<App />)

    expect(await screen.findByText("No Messages chats found.")).toBeInTheDocument()
    expect(screen.getByRole("button", { name: "Sync Now" })).toBeDisabled()
    fireEvent.click(screen.getByRole("button", { name: "Retry chat discovery" }))

    expect(await screen.findByRole("checkbox", { name: /Chat alpha/ })).toBeChecked()
    await waitFor(() => expect(screen.getByRole("button", { name: "Sync Now" })).toBeEnabled())
  })
})
