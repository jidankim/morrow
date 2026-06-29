import { act, fireEvent, render, screen, waitFor } from "@testing-library/react"
import { beforeEach, describe, expect, it, vi } from "vitest"
import { App } from "./App"
import type { MorrowDataDeleteReceipt } from "./tauriBridge"

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
  checkProviderAuth: vi.fn(async () => ({
    status: "loggedInUsingChatGpt",
    ready: true,
    commandSurface: "codex login status",
    commandOutputRedacted: true,
    diagnostic: "Codex CLI ChatGPT session is ready."
  })),
  readMorrowToken: vi.fn(async () => ({ storageSurface: "keychainBridge", present: true })),
  deleteMorrowToken: vi.fn(async () => ({ storageSurface: "keychainBridge", stored: false, deleted: true })),
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
      }
    ]
  })),
  openPrivacySettings: vi.fn(async () => ({ pane: "fullDiskAccess", opened: true })),
  deleteMorrowData: vi.fn(async (): Promise<MorrowDataDeleteReceipt> => deleteSuccessReceipt()),
  recordCrashLog: vi.fn(async (_request: CrashLogRequestForTest) => ({ stored: true }))
}))

vi.mock("./tauriBridge", () => ({
  MORROW_KEYCHAIN_SERVICE: "com.morrow.desktop.token",
  MORROW_PROVIDER_TOKEN_KIND: "morrow-openai-provider-api-key",
  createNativeShellBridge: () => bridgeMock
}))

const deleteSuccessReceipt = (): MorrowDataDeleteReceipt => ({
  storageSurface: "morrowStore",
  databaseDeleted: true,
  approvedExternalItemsDeleted: false,
  providerCredentialsDeleteRequested: true,
  providerCredentialsDeleted: true,
  providerCredentialsDeleteFailed: false,
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
})

const deleteCredentialFailureReceipt = (): MorrowDataDeleteReceipt => ({
  ...deleteSuccessReceipt(),
  databaseDeleted: false,
  providerCredentialsDeleted: false,
  providerCredentialsDeleteFailed: true,
  providerCredentialsDeleteError: "Morrow Keychain delete failed with OSStatus -60008",
  providerCredentialDeletes: [
    {
      tokenKind: "morrow-owned-token",
      deleteRequested: true,
      deleted: false,
      failed: true,
      error: "Morrow Keychain delete failed with OSStatus -60008"
    },
    {
      tokenKind: "morrow-openai-provider-api-key",
      deleteRequested: true,
      deleted: false,
      failed: true,
      error: "Morrow Keychain delete failed with OSStatus -60008"
    }
  ],
  cleanupPlan: {
    proposedItems: "completed",
    emptyProposalContainers: "skippedByUser",
    proposedCalendarItemsDeleted: 0,
    proposedReminderItemsDeleted: 0
  }
})

const renderSettings = (): void => {
  render(<App />)
  act(() => {
    window.location.hash = "#settings"
    window.dispatchEvent(new HashChangeEvent("hashchange"))
  })
}

const confirmDeleteAll = async (): Promise<void> => {
  const confirmationInput = await screen.findByLabelText("Type DELETE MORROW DATA to confirm")
  await act(async () => {
    fireEvent.change(confirmationInput, {
      target: { value: "DELETE MORROW DATA" }
    })
    fireEvent.click(screen.getByRole("button", { name: "Delete Morrow data" }))
  })
}

describe("App delete-all privacy controls", () => {
  beforeEach(() => {
    window.localStorage.clear()
    window.location.hash = ""
    bridgeMock.deleteMorrowData.mockClear()
    bridgeMock.recordCrashLog.mockClear()
  })

  it("requires type-to-confirm before delete-all calls native Morrow data deletion", async () => {
    renderSettings()

    const deleteButton = screen.getByRole("button", { name: "Delete Morrow data" })
    expect(screen.getByLabelText("Delete proposed Morrow items")).toBeChecked()
    expect(screen.getByLabelText("Delete empty Morrow Proposed containers")).not.toBeChecked()
    expect(screen.getByLabelText("Delete Morrow-owned provider credentials")).toBeChecked()
    expect(document.body).not.toHaveTextContent("OAuth")
    expect(deleteButton).toBeDisabled()

    await act(async () => {
      fireEvent.change(screen.getByLabelText("Type DELETE MORROW DATA to confirm"), {
        target: { value: "delete morrow data" }
      })
    })
    expect(deleteButton).toBeDisabled()

    await confirmDeleteAll()

    await waitFor(() => expect(bridgeMock.deleteMorrowData).toHaveBeenCalledOnce())
    expect(bridgeMock.deleteMorrowData).toHaveBeenCalledWith({
      confirmation: "DELETE MORROW DATA",
      cleanupProposedItems: true,
      deleteEmptyProposalContainers: false,
      revokeProviderOAuth: true
    })
    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("Morrow data reset"))
    expect(screen.getByText("Approved Calendar and Reminders items were preserved.")).toBeInTheDocument()
    expect(
      screen.getByText("Morrow-owned provider credentials were deleted. Codex CLI login was left unchanged.")
    ).toBeInTheDocument()
    expect(screen.getByText("Deleted 1 proposed Calendar item(s) and 1 proposed Reminder item(s).")).toBeInTheDocument()
  })

  it("reports non-fatal provider credential delete failures without claiming Codex login deletion", async () => {
    bridgeMock.deleteMorrowData.mockResolvedValueOnce(deleteCredentialFailureReceipt())
    renderSettings()

    await confirmDeleteAll()

    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("Morrow data reset"))
    expect(
      screen.getByText(
        "Morrow-owned provider credentials could not be deleted by macOS. Codex CLI login was left unchanged."
      )
    ).toBeInTheDocument()
    expect(document.body).not.toHaveTextContent("global Codex CLI login was deleted")
    expect(document.body).not.toHaveTextContent("OAuth")
  })

  it("scrubs crash log text before reporting delete-all failures", async () => {
    bridgeMock.deleteMorrowData.mockRejectedValueOnce(
      new Error('delete failed excerpt="private clinic visit" prompt="raw prompt"')
    )
    renderSettings()

    await confirmDeleteAll()

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
    renderSettings()

    await confirmDeleteAll()

    await waitFor(() => expect(bridgeMock.recordCrashLog).toHaveBeenCalledOnce())
    act(() => {
      window.location.hash = "#status"
      window.dispatchEvent(new HashChangeEvent("hashchange"))
    })
    expect(screen.getByTestId("status-label")).toHaveTextContent("Error")
    expect(screen.getByText("Calendar access was denied excerpt=[redacted]")).toBeInTheDocument()
  })
})
