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
  discoverMessagesChats: vi.fn(async () => ({
    status: "ready",
    chats: [
      {
        chatId: "messages-chat-11111111111111111111111111111111",
        displayLabel: "Chat alpha",
        participantCount: 2,
        participantIds: ["messages-participant-11111111111111111111111111111111", "messages-participant-22222222222222222222222222222222"],
        latestActivityTimestamp: 1_783_000_000
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

  it("requires type-to-confirm before delete-all calls native Morrow data deletion", async () => {
    render(<App />)

    act(() => {
      window.location.hash = "#settings"
      window.dispatchEvent(new HashChangeEvent("hashchange"))
    })

    const deleteButton = screen.getByRole("button", { name: "Delete Morrow data" })
    expect(screen.getByLabelText("Delete proposed Morrow items")).toBeChecked()
    expect(screen.getByLabelText("Delete empty Morrow Proposed containers")).not.toBeChecked()
    expect(deleteButton).toBeDisabled()

    fireEvent.change(screen.getByLabelText("Type DELETE MORROW DATA to confirm"), {
      target: { value: "delete morrow data" }
    })
    expect(deleteButton).toBeDisabled()

    fireEvent.change(screen.getByLabelText("Type DELETE MORROW DATA to confirm"), {
      target: { value: "DELETE MORROW DATA" }
    })
    fireEvent.click(deleteButton)

    await waitFor(() => expect(bridgeMock.deleteMorrowData).toHaveBeenCalledOnce())
    expect(bridgeMock.deleteMorrowData).toHaveBeenCalledWith({
      confirmation: "DELETE MORROW DATA",
      cleanupProposedItems: true,
      deleteEmptyProposalContainers: false,
      revokeProviderOAuth: true
    })
    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("Morrow data reset"))
    expect(screen.getByText("Approved Calendar and Reminders items were preserved.")).toBeInTheDocument()
    expect(screen.getByText("Deleted 1 proposed Calendar item(s) and 1 proposed Reminder item(s).")).toBeInTheDocument()
  })

  it("reports non-fatal OAuth revoke failures after delete-all succeeds", async () => {
    bridgeMock.deleteMorrowData.mockResolvedValueOnce({
      storageSurface: "morrowStore",
      databaseDeleted: false,
      approvedExternalItemsDeleted: false,
      providerOAuthDeleteRequested: true,
      providerOAuthDeleted: false,
      providerOAuthDeleteFailed: true,
      providerOAuthDeleteError: "Morrow Keychain delete failed with OSStatus -60008",
      cleanupPlan: {
        proposedItems: "completed",
        emptyProposalContainers: "skippedByUser",
        proposedCalendarItemsDeleted: 0,
        proposedReminderItemsDeleted: 0
      }
    })
    render(<App />)

    act(() => {
      window.location.hash = "#settings"
      window.dispatchEvent(new HashChangeEvent("hashchange"))
    })
    fireEvent.change(await screen.findByLabelText("Type DELETE MORROW DATA to confirm"), {
      target: { value: "DELETE MORROW DATA" }
    })
    fireEvent.click(screen.getByRole("button", { name: "Delete Morrow data" }))

    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("Morrow data reset"))
    expect(screen.getByText("Morrow OAuth grant could not be revoked by macOS.")).toBeInTheDocument()
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

  it("scrubs crash log text before reporting delete-all failures", async () => {
    bridgeMock.deleteMorrowData.mockRejectedValueOnce(
      new Error('delete failed excerpt="private clinic visit" prompt="raw prompt"')
    )
    render(<App />)

    act(() => {
      window.location.hash = "#settings"
      window.dispatchEvent(new HashChangeEvent("hashchange"))
    })
    fireEvent.change(await screen.findByLabelText("Type DELETE MORROW DATA to confirm"), {
      target: { value: "DELETE MORROW DATA" }
    })
    fireEvent.click(screen.getByRole("button", { name: "Delete Morrow data" }))

    await waitFor(() => expect(bridgeMock.recordCrashLog).toHaveBeenCalledOnce())
    const crashRequest = bridgeMock.recordCrashLog.mock.calls[0]?.[0]
    expect(crashRequest?.message).toContain("excerpt=[redacted]")
    expect(crashRequest?.message).toContain("prompt=[redacted]")
    expect(crashRequest?.message).not.toContain("private clinic visit")
    expect(crashRequest?.message).not.toContain("raw prompt")
  })

  it("surfaces scrubbed native string rejection from delete-all as a visible failure", async () => {
    bridgeMock.deleteMorrowData.mockRejectedValueOnce(
      'Calendar access was denied excerpt="private clinic visit"'
    )
    render(<App />)

    act(() => {
      window.location.hash = "#settings"
      window.dispatchEvent(new HashChangeEvent("hashchange"))
    })
    fireEvent.change(await screen.findByLabelText("Type DELETE MORROW DATA to confirm"), {
      target: { value: "DELETE MORROW DATA" }
    })
    fireEvent.click(screen.getByRole("button", { name: "Delete Morrow data" }))

    await waitFor(() => expect(bridgeMock.recordCrashLog).toHaveBeenCalledOnce())
    act(() => {
      window.location.hash = "#status"
      window.dispatchEvent(new HashChangeEvent("hashchange"))
    })
    expect(screen.getByTestId("status-label")).toHaveTextContent("Error")
    expect(screen.getByText("Calendar access was denied excerpt=[redacted]")).toBeInTheDocument()
  })
})
