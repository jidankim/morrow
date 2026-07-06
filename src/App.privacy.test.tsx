import { fireEvent, render, screen, waitFor } from "@testing-library/react"
import { beforeEach, describe, expect, it } from "vitest"
import {
  bridgeMock,
  emptyPreviewChatId,
  openAppRoute,
  previewPrivacySentinel,
  seedPreviewPrivacyReadyState,
  seedReadyState
} from "./AppPrivacyTestHarness"
import { registerSettingsFullDiskAccessGuideTests } from "./AppPrivacySettingsGuideTestCases"
import { App } from "./App"
import { APP_SHELL_STATE_KEY } from "./domain/appShell"
import { parseSyncScanRequest } from "./messagesDiscoveryBridge"

describe("App privacy controls", () => {
  beforeEach(() => {
    window.localStorage.clear()
    window.location.hash = ""
    bridgeMock.scanSelectedChats.mockClear()
    bridgeMock.readMorrowToken.mockClear()
    bridgeMock.discoverMessagesChats.mockClear()
    bridgeMock.loadMessagesChatPreviews.mockClear()
    bridgeMock.openPrivacySettings.mockClear()
    bridgeMock.deleteMorrowData.mockClear()
    bridgeMock.recordCrashLog.mockClear()
  })

  registerSettingsFullDiskAccessGuideTests()

  it("keeps revealed preview text out of persisted app state and scan requests", async () => {
    seedPreviewPrivacyReadyState()
    render(<App />)

    await waitFor(() => expect(screen.getByRole("button", { name: "Sync Now" })).toBeEnabled())
    expect(document.body).not.toHaveTextContent(previewPrivacySentinel)
    fireEvent.click(screen.getByRole("button", { name: "Reveal previews locally" }))

    expect(await screen.findByText(previewPrivacySentinel)).toHaveAttribute(
      "data-visual-qa-text",
      "chat-row-preview"
    )
    expect(screen.queryByText("No preview available")).not.toBeInTheDocument()
    fireEvent.click(screen.getByRole("checkbox", { name: /Messages chat/ }))
    fireEvent.click(screen.getByRole("button", { name: "Sync Now" }))

    await waitFor(() => expect(bridgeMock.scanSelectedChats).toHaveBeenCalledOnce())
    const scanRequest = bridgeMock.scanSelectedChats.mock.calls[0]?.[0]
    const serializedPrivacySurfaces = JSON.stringify({
      persistedState: window.localStorage.getItem(APP_SHELL_STATE_KEY),
      scanRequest,
      setShellStateCalls: bridgeMock.setShellState.mock.calls,
      crashLogRequests: bridgeMock.recordCrashLog.mock.calls,
      providerReadinessRequests: bridgeMock.checkProviderAuth.mock.calls,
      tokenReadRequests: bridgeMock.readMorrowToken.mock.calls,
      previewRequests: bridgeMock.loadMessagesChatPreviews.mock.calls
    })

    expect(serializedPrivacySurfaces).not.toContain(previewPrivacySentinel)
    expect(serializedPrivacySurfaces).not.toContain("latestMessageBody")
    expect(serializedPrivacySurfaces).not.toContain("latestMessagePreview")
    expect(JSON.stringify(scanRequest?.selectedChats)).not.toContain("preview")
    expect(scanRequest?.selectedChatIds).toEqual(["messages-chat-11111111111111111111111111111111", emptyPreviewChatId])
  })

  it("passes hidden source-excerpt setting through the scan request", async () => {
    seedReadyState()
    render(<App />)

    openAppRoute("#settings")
    fireEvent.click(screen.getByLabelText("Show short source excerpts in future proposal notes"))
    openAppRoute("#status")
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
          participantIds: [
            "messages-participant-11111111111111111111111111111111",
            "messages-participant-22222222222222222222222222222222"
          ],
          latestActivityTimestamp: 1_783_000_000
        }
      ],
      referenceTimezone: "Asia/Seoul",
      referenceUnixSeconds: expect.any(Number),
      backfillPromptChatIds: ["messages-chat-11111111111111111111111111111111"],
      sourceExcerptsEnabled: false,
      feedbackTextSnapshotsEnabled: false,
      localDiagnosticsEnabled: false,
      localDiagnosticsRetentionDays: 30,
      listReminderProfile: {
        enabled: false,
        profileId: "list-reminders",
        profileVersion: "list-reminders-v1",
        routingMode: "explicitOnly",
        defaultDueMode: "explicitOnly",
        defaultDueTime: "23:59",
        recurrenceMode: "none",
        itemOutputMode: "singleReminderTitle"
      },
      capPolicy: {
        mode: "refillForPending",
        maxVisible: 10,
        pendingCount: 0
      }
    })
  })

  it("requires type-to-confirm before delete-all calls native Morrow data deletion", async () => {
    render(<App />)

    openAppRoute("#settings")

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
    expect(await screen.findByRole("heading", { name: "Morrow data reset" })).toBeInTheDocument()
    expect(screen.getByText("Approved Calendar and Reminders items were preserved.")).toBeInTheDocument()
    expect(screen.getByText("Local diagnostics artifacts were deleted.")).toBeInTheDocument()
    expect(screen.getByText("Deleted 1 proposed Calendar item(s) and 1 proposed Reminder item(s).")).toBeInTheDocument()
  })

  it("surfaces local diagnostics artifacts as private local files without remote telemetry controls", async () => {
    render(<App />)

    openAppRoute("#settings")

    expect(screen.getByLabelText("Write private local diagnostics files")).not.toBeChecked()
    expect(screen.getByLabelText("Local diagnostics retention (days)")).toHaveValue(30)
    expect(screen.getByText(/private local files on this Mac/i)).toBeInTheDocument()
    expect(screen.getByText(/not uploads/i)).toBeInTheDocument()
    expect(screen.getByText(/Delete All deletes these files/i)).toBeInTheDocument()
    expect(screen.queryByLabelText(/telemetry/i)).not.toBeInTheDocument()
    await waitFor(() => expect(bridgeMock.discoverMessagesChats).toHaveBeenCalledOnce())
  })

  it("reports non-fatal OAuth revoke failures after delete-all succeeds", async () => {
    bridgeMock.deleteMorrowData.mockResolvedValueOnce({
      storageSurface: "morrowStore",
      databaseDeleted: false,
      approvedExternalItemsDeleted: false,
      diagnosticsArtifactsDeleted: false,
      providerOAuthDeleteRequested: true,
      providerOAuthDeleted: false,
      providerOAuthDeleteFailed: true,
      providerOAuthDeleteError: "Morrow Keychain delete failed with OSStatus -60008",
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
    render(<App />)

    openAppRoute("#settings")

    fireEvent.change(await screen.findByLabelText("Type DELETE MORROW DATA to confirm"), {
      target: { value: "DELETE MORROW DATA" }
    })
    fireEvent.click(screen.getByRole("button", { name: "Delete Morrow data" }))

    expect(await screen.findByRole("heading", { name: "Morrow data reset" })).toBeInTheDocument()
    expect(
      screen.getByText(
        "Morrow-owned provider credentials could not be deleted by macOS. Codex CLI login was left unchanged."
      )
    ).toBeInTheDocument()
  })

  it("scrubs crash log text before reporting delete-all failures", async () => {
    bridgeMock.deleteMorrowData.mockRejectedValueOnce(
      new Error('delete failed excerpt="synthetic diagnostic excerpt" prompt="raw prompt"')
    )
    render(<App />)

    openAppRoute("#settings")
    fireEvent.change(await screen.findByLabelText("Type DELETE MORROW DATA to confirm"), {
      target: { value: "DELETE MORROW DATA" }
    })
    fireEvent.click(screen.getByRole("button", { name: "Delete Morrow data" }))

    await waitFor(() => expect(bridgeMock.recordCrashLog).toHaveBeenCalledOnce())
    const crashRequest = bridgeMock.recordCrashLog.mock.calls[0]?.[0]
    expect(crashRequest?.message).toContain("excerpt=[redacted]")
    expect(crashRequest?.message).toContain("prompt=[redacted]")
    expect(crashRequest?.message).not.toContain("synthetic diagnostic excerpt")
    expect(crashRequest?.message).not.toContain("raw prompt")
  })

  it("surfaces scrubbed native string rejection from delete-all as a visible failure", async () => {
    bridgeMock.deleteMorrowData.mockRejectedValueOnce(
      'Calendar access was denied excerpt="synthetic diagnostic excerpt"'
    )
    render(<App />)

    openAppRoute("#settings")
    fireEvent.change(await screen.findByLabelText("Type DELETE MORROW DATA to confirm"), {
      target: { value: "DELETE MORROW DATA" }
    })
    fireEvent.click(screen.getByRole("button", { name: "Delete Morrow data" }))

    await waitFor(() => expect(bridgeMock.recordCrashLog).toHaveBeenCalledOnce())
    openAppRoute("#status")
    expect(screen.getByTestId("status-label")).toHaveTextContent("Error")
    expect(screen.getByText("Calendar access was denied excerpt=[redacted]")).toBeInTheDocument()
  })

  it("rejects malformed feedback snapshot consent in scan request shapes", () => {
    const scanRequest = {
      selectedChatIds: ["messages-chat-11111111111111111111111111111111"],
      selectedChats: [
        {
          id: "messages-chat-11111111111111111111111111111111",
          label: "Chat alpha",
          participantCount: 2,
          participantIds: [
            "messages-participant-11111111111111111111111111111111",
            "messages-participant-22222222222222222222222222222222"
          ],
          latestActivityTimestamp: 1_783_000_000
        }
      ],
      referenceTimezone: "Asia/Seoul",
      referenceUnixSeconds: 1_783_000_200,
      backfillPromptChatIds: ["messages-chat-11111111111111111111111111111111"],
      sourceExcerptsEnabled: true,
      feedbackTextSnapshotsEnabled: true,
      localDiagnosticsEnabled: false,
      localDiagnosticsRetentionDays: 30,
      listReminderProfile: {
        enabled: false,
        profileId: "list-reminders",
        profileVersion: "list-reminders-v1",
        routingMode: "explicitOnly",
        defaultDueMode: "explicitOnly",
        defaultDueTime: "23:59",
        recurrenceMode: "none",
        itemOutputMode: "singleReminderTitle"
      },
      capPolicy: { mode: "refillForPending", maxVisible: 10, pendingCount: 0 }
    } as const

    expect(parseSyncScanRequest(scanRequest).feedbackTextSnapshotsEnabled).toBe(true)
    expect(() =>
      parseSyncScanRequest({
        ...scanRequest,
        feedbackTextSnapshotsEnabled: -1
      })
    ).toThrow()
    const { feedbackTextSnapshotsEnabled: _missing, ...missingConsent } = scanRequest
    expect(() => parseSyncScanRequest(missingConsent)).toThrow()
  })
})
