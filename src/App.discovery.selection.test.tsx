import { fireEvent, screen, waitFor } from "@testing-library/react"
import { beforeEach, describe, expect, it } from "vitest"
import {
  bridgeMock,
  changedNativeReadyReport,
  discoveredChat,
  fixtureReferenceTimezone,
  nativeChatFromFixture,
  nativeReadyReport,
  privatePreviewText,
  rediscoveredChat,
  renderDiscoveryApp,
  resetDiscoveryAppTestState,
  seedSelectedChat
} from "./App.discoveryHarness"
import { registerDiscoveryGuideTests } from "./App.discoveryGuideTests"
import { APP_SHELL_STATE_KEY, createDefaultAppShellState } from "./domain/appShell"

describe("App Messages chat discovery selection and sync", () => {
  beforeEach(resetDiscoveryAppTestState)

  registerDiscoveryGuideTests({
    renderApp: renderDiscoveryApp,
    bridgeMock
  })

  it("blocks Sync Now until required setup and at least one chat are selected", async () => {
    renderDiscoveryApp()

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
    renderDiscoveryApp()

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
    window.localStorage.setItem(
      APP_SHELL_STATE_KEY,
      JSON.stringify({ ...initial, config: { ...initial.config, referenceTimezone: fixtureReferenceTimezone } })
    )
    bridgeMock.discoverMessagesChats.mockResolvedValueOnce({
      status: "ready",
      chats: [
        nativeChatFromFixture({
          id: "messages-chat-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
          label: "Messages chat",
          participantCount: 1,
          participantIds: ["messages-participant-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"],
          latestActivityTimestamp: 1_783_000_000
        }),
        nativeChatFromFixture({
          id: "messages-chat-bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
          label: "Messages chat",
          participantCount: 2,
          participantIds: [
            "messages-participant-bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "messages-participant-cccccccccccccccccccccccccccccccc"
          ],
          latestActivityTimestamp: 1_783_000_000
        })
      ]
    })

    renderDiscoveryApp()

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
    const rawPrivateFixtureValues = [
      "messages-chat-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "messages-chat-bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "messages-participant-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "messages-participant-bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "messages-participant-cccccccccccccccccccccccccccccccc",
      "1783000000"
    ] as const
    for (const privateFixtureValue of rawPrivateFixtureValues) {
      expect(document.body).not.toHaveTextContent(privateFixtureValue)
    }
  })

  it("enables then disables Sync Now as a real discovered chat is selected and deselected", async () => {
    renderDiscoveryApp()

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
    renderDiscoveryApp()

    expect(await screen.findByRole("checkbox", { name: /Updated alpha/ })).toBeChecked()
    const syncButton = screen.getByRole("button", { name: "Sync Now" })
    await waitFor(() => expect(syncButton).toBeEnabled())
    fireEvent.click(syncButton)

    await waitFor(() => expect(bridgeMock.scanSelectedChats).toHaveBeenCalledOnce())
    expect(screen.getByTestId("feedback-label-count")).toHaveTextContent("9")
    expect(screen.getByTestId("feature-snapshot-count")).toHaveTextContent("4")
    expect(screen.getByTestId("latest-eval-status")).toHaveTextContent("Passed")
    expect(document.body).not.toHaveTextContent(privatePreviewText)
    expect(bridgeMock.scanSelectedChats).toHaveBeenCalledWith({
      selectedChatIds: [discoveredChat.id],
      selectedChats: [rediscoveredChat],
      referenceTimezone: fixtureReferenceTimezone,
      referenceUnixSeconds: expect.any(Number),
      backfillPromptChatIds: [discoveredChat.id],
      sourceExcerptsEnabled: true,
      feedbackTextSnapshotsEnabled: false,
      localDiagnosticsEnabled: false,
      localDiagnosticsRetentionDays: 30,
      listIntakeProfiles: [],
      capPolicy: { mode: "refillForPending", maxVisible: 10, pendingCount: 0 }
    })
  })

  it("keeps a stale persisted selected chat disabled until rediscovered", async () => {
    seedSelectedChat()
    bridgeMock.discoverMessagesChats
      .mockResolvedValueOnce({ status: "empty", chats: [] })
      .mockResolvedValueOnce(nativeReadyReport)
    renderDiscoveryApp()

    expect(await screen.findByText("Messages discovery finished, but found no eligible chats.")).toBeInTheDocument()
    expect(screen.getAllByText("Refresh chat discovery before scanning selected chats.").length).toBeGreaterThan(1)
    expect(screen.queryByRole("checkbox", { name: /Chat alpha/ })).not.toBeInTheDocument()
    expect(screen.getByRole("button", { name: "Sync Now" })).toBeDisabled()
    fireEvent.click(screen.getByRole("button", { name: "Retry chat discovery" }))

    expect(await screen.findByRole("checkbox", { name: /Chat alpha/ })).toBeChecked()
    await waitFor(() => expect(screen.getByRole("button", { name: "Sync Now" })).toBeEnabled())
  })
})
