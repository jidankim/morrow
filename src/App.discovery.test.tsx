import { fireEvent, render, screen, waitFor, act } from "@testing-library/react"
import { beforeEach, describe, expect, it, vi } from "vitest"
import { App } from "./App"
import { ChatDiscoveryControls } from "./ChatDiscoveryControls"
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

type NativeReadyChatForTest = Extract<NativeDiscoveryReportForTest, { readonly status: "ready" }>["chats"][number]
type ChatFixtureForTest = Pick<NativeReadyChatForTest, "participantCount" | "participantIds" | "latestActivityTimestamp"> & {
  readonly id: string
  readonly label: string
}

const discoveredChat = {
  id: "messages-chat-11111111111111111111111111111111",
  label: "Chat alpha",
  participantCount: 2,
  participantIds: ["messages-participant-11111111111111111111111111111111", "messages-participant-22222222222222222222222222222222"],
  latestActivityTimestamp: 1_783_000_000
} as const

const rediscoveredChat = {
  id: "messages-chat-11111111111111111111111111111111",
  label: "Updated alpha",
  participantCount: 3,
  participantIds: ["messages-participant-66666666666666666666666666666666", "messages-participant-77777777777777777777777777777777", "messages-participant-88888888888888888888888888888888"],
  latestActivityTimestamp: 1_783_001_000
} as const

const fixtureReferenceTimezone = "Asia/Seoul"

const nativeChatFromFixture = (chat: ChatFixtureForTest): NativeReadyChatForTest => ({
  chatId: chat.id,
  displayLabel: chat.label,
  participantCount: chat.participantCount,
  participantIds: chat.participantIds,
  latestActivityTimestamp: chat.latestActivityTimestamp
})

const nativeReadyReport = { status: "ready", chats: [nativeChatFromFixture(discoveredChat)] } as const

const changedNativeReadyReport = { status: "ready", chats: [nativeChatFromFixture(rediscoveredChat)] } as const

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
  const ready = { ...initial, config: { ...initial.config, permissionsGranted: true, referenceTimezone: fixtureReferenceTimezone }, discovery: { status: "ready", chats: [discoveredChat] }, selectedChats: [{ ...discoveredChat, backfillPromptEnabled: true }] }
  window.localStorage.setItem(APP_SHELL_STATE_KEY, JSON.stringify(ready))
}

const seedPermissionsGranted = (): void => {
  const initial = createDefaultAppShellState()
  const ready = { ...initial, config: { ...initial.config, permissionsGranted: true } }
  window.localStorage.setItem(APP_SHELL_STATE_KEY, JSON.stringify(ready))
}

const idleDiscoveryControlProps = {
  referenceTimezone: "UTC",
  selectedChats: [],
  onOpenFullDiskAccess: () => undefined,
  onRetry: () => undefined,
  onToggleBackfillPrompt: () => undefined,
  onToggleChat: () => undefined
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
    expect(screen.queryByLabelText("Required permissions complete")).not.toBeInTheDocument()
    expect(screen.queryByText("Required permissions complete")).not.toBeInTheDocument()
    expect(screen.getByText("Messages discovery found 1 eligible chat.")).toBeInTheDocument()
    expect(screen.getByText("Sync Now disabled: Select at least one chat before scanning.")).toBeInTheDocument()
    expect(screen.getAllByText("Select at least one chat before scanning.").length).toBeGreaterThan(1)
    expect(screen.getByText("Select a chat to verify it for scanning.")).toBeInTheDocument()
    expect(screen.getByTestId("status-label")).toHaveTextContent("Setup needed")
    expect(screen.getByRole("button", { name: "Sync Now" })).toBeDisabled()
    expect(bridgeMock.reconcileNow).not.toHaveBeenCalled()
    expect(bridgeMock.scanSelectedChats).not.toHaveBeenCalled()
  })

  it("shows Sync Now readiness checklist for a ready selected chat", async () => {
    seedSelectedChat()
    render(<App />)

    await waitFor(() => expect(screen.getByRole("button", { name: "Sync Now" })).toBeEnabled())
    expect(screen.getByText("Sync Now ready: All setup checks are complete.")).toBeInTheDocument()
    expect(screen.queryByLabelText("Required permissions complete")).not.toBeInTheDocument()
    expect(screen.queryByText("Required permissions complete")).not.toBeInTheDocument()
    expect(screen.getByText("Messages discovery found 1 eligible chat.")).toBeInTheDocument()
    expect(screen.getByText("1 chat selected.")).toBeInTheDocument()
    expect(screen.getByText("Selected chats are verified for scanning.")).toBeInTheDocument()
  })

  it("renders Messages source summary with selected count and latest activity", async () => {
    const initial = createDefaultAppShellState()
    window.localStorage.setItem(APP_SHELL_STATE_KEY, JSON.stringify({ ...initial, config: { ...initial.config, referenceTimezone: fixtureReferenceTimezone } }))
    bridgeMock.discoverMessagesChats.mockResolvedValueOnce({
      status: "ready",
      chats: [
        nativeChatFromFixture({ id: "messages-chat-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", label: "Messages chat", participantCount: 1, participantIds: ["messages-participant-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"], latestActivityTimestamp: 1_783_000_000 }),
        nativeChatFromFixture({ id: "messages-chat-bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb", label: "Messages chat", participantCount: 2, participantIds: ["messages-participant-bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb", "messages-participant-cccccccccccccccccccccccccccccccc"], latestActivityTimestamp: 1_783_000_000 })
      ]
    })

    render(<App />)

    const chatCheckboxes = await screen.findAllByRole("checkbox", { name: /Messages chat/ })
    expect(chatCheckboxes).toHaveLength(2)
    expect(screen.getByText("Messages source: 2 eligible chats, 0 selected.")).toBeInTheDocument()
    expect(screen.getByText("1 participant")).toBeInTheDocument()
    expect(screen.getByText("2 participants")).toBeInTheDocument()
    expect(screen.getAllByText("Last active Jul 2, 2026, 10:46 PM")).toHaveLength(2)
    expect(screen.getAllByText("Not selected")).toHaveLength(2)
    const firstChatCheckbox = chatCheckboxes[0]
    if (firstChatCheckbox === undefined) {
      throw new Error("Expected a Messages chat checkbox.")
    }
    fireEvent.click(firstChatCheckbox)

    expect(screen.getByText("Messages source: 2 eligible chats, 1 selected.")).toBeInTheDocument()
    expect(screen.getByText("Selected")).toBeInTheDocument()
    expect(screen.getByRole("checkbox", { name: "Ask before backfilling older messages" })).toBeChecked()
    expect(screen.getByText("Morrow asks before using older messages from this chat.")).toBeInTheDocument()
    expect(document.body).not.toHaveTextContent("iMessage;-;")
    expect(document.body).not.toHaveTextContent("person@example.com")
    expect(document.body).not.toHaveTextContent("+15555550103")
    expect(document.body).not.toHaveTextContent("See you at 7")
  })

  it("renders distinct empty discovery copy", async () => {
    let resolveDiscovery: (report: NativeDiscoveryReportForTest) => void = () => undefined
    bridgeMock.discoverMessagesChats.mockReturnValueOnce(
      new Promise<NativeDiscoveryReportForTest>((resolve) => {
        resolveDiscovery = resolve
      })
    )
    render(<App />)

    expect((await screen.findAllByText("Morrow is checking local Messages access.")).length).toBeGreaterThan(1)
    await act(async () => resolveDiscovery({ status: "empty", chats: [] }))

    expect(screen.getByText("Messages discovery finished, but found no eligible chats.")).toBeInTheDocument()
    expect(screen.getByText("Messages discovery found no eligible chats.")).toBeInTheDocument()
    expect(screen.getByRole("button", { name: "Sync Now" })).toBeDisabled()
  })

  it("shows Full Disk Access recovery when Messages discovery is denied", async () => {
    seedPermissionsGranted()
    bridgeMock.discoverMessagesChats.mockResolvedValueOnce({ status: "permissionDenied", chats: [] })
    render(<App />)

    expect(await screen.findByText("Morrow needs Full Disk Access to read Messages.")).toBeInTheDocument()
    expect(screen.getByText("Sync Now disabled: Grant Full Disk Access, then retry chat discovery.")).toBeInTheDocument()
    expect(screen.getAllByText("Grant Full Disk Access, then retry chat discovery.").length).toBeGreaterThan(1)
    expect(screen.getByRole("button", { name: "Open Full Disk Access" })).toBeInTheDocument()
  })

  it("renders distinct denied discovery recovery copy", async () => {
    bridgeMock.discoverMessagesChats.mockResolvedValueOnce({ status: "permissionDenied", chats: [] })
    render(<App />)

    expect(await screen.findByText("Morrow needs Full Disk Access to read Messages.")).toBeInTheDocument()
    expect(screen.getAllByText("Grant Full Disk Access, then retry chat discovery.").length).toBeGreaterThan(1)
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

    expect(await screen.findByText("Morrow could not read Messages.")).toBeInTheDocument()
    expect(screen.getAllByText("Retry chat discovery or check local Messages access.").length).toBeGreaterThan(1)
    fireEvent.click(screen.getByRole("button", { name: "Retry chat discovery" }))
    expect(await screen.findByText("Messages discovery finished, but found no eligible chats.")).toBeInTheDocument()
    fireEvent.click(screen.getByRole("button", { name: "Retry chat discovery" }))

    expect(await screen.findByRole("checkbox", { name: /Chat alpha/ })).toBeInTheDocument()
    expect(bridgeMock.discoverMessagesChats).toHaveBeenCalledTimes(3)
  })

  it("enables then disables Sync Now as a real discovered chat is selected and deselected", async () => {
    render(<App />)

    const chatCheckbox = await screen.findByRole("checkbox", { name: /Chat alpha/ })
    expect(screen.queryByLabelText("Required permissions complete")).not.toBeInTheDocument()
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
      referenceTimezone: fixtureReferenceTimezone,
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

    expect(await screen.findByText("Messages discovery finished, but found no eligible chats.")).toBeInTheDocument()
    expect(screen.getAllByText("Refresh chat discovery before scanning selected chats.").length).toBeGreaterThan(1)
    expect(screen.queryByRole("checkbox", { name: /Chat alpha/ })).not.toBeInTheDocument()
    expect(screen.getByRole("button", { name: "Sync Now" })).toBeDisabled()
    fireEvent.click(screen.getByRole("button", { name: "Retry chat discovery" }))

    expect(await screen.findByRole("checkbox", { name: /Chat alpha/ })).toBeChecked()
    await waitFor(() => expect(screen.getByRole("button", { name: "Sync Now" })).toBeEnabled())
  })

  it("renders distinct unverified discovery copy", () => {
    render(<ChatDiscoveryControls {...idleDiscoveryControlProps} discovery={{ status: "unverified", chats: [] }} />)

    expect(screen.getByText("Messages discovery has not completed yet.")).toBeInTheDocument()
    expect(screen.getByRole("button", { name: "Retry chat discovery" })).toBeInTheDocument()
  })
})
