import { beforeEach, describe, expect, it, vi } from "vitest"
import { createListIntakeProfile } from "./domain/appConfig"

const tauriMock = vi.hoisted(() => ({
  invoke: vi.fn(async (): Promise<unknown> => ({ pendingProposalCount: 3 }))
}))

vi.mock("@tauri-apps/api/core", () => ({
  invoke: tauriMock.invoke
}))

describe("messagesTauriCommands boundary parsing", () => {
  beforeEach(() => {
    tauriMock.invoke.mockClear()
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: {}
    })
  })

  it("parses messages command requests and native responses at the Tauri edge", async () => {
    // Given
    const {
      discoverMessagesChatsInTauri,
      loadMessagesChatPreviewsInTauri,
      scanSelectedChatsInTauri
    } = await import("./messagesTauriCommands")
    const selectedChat = {
      id: "messages-chat-11111111111111111111111111111111",
      label: "Team planning",
      participantCount: 1,
      participantIds: ["messages-participant-11111111111111111111111111111111"],
      latestActivityTimestamp: 1_783_000_000
    } as const
    const listIntakeProfile = createListIntakeProfile({
      enabled: true,
      profileId: "list-intake-fishcount",
      name: "Fish count",
      profileVersion: "list-intake-v2",
      kind: "quantityList",
      extractionMode: "providerConstrained",
      providerPromptVersion: "list-intake-v1",
      positiveExamples: ["2 anchovies, 3 salmon"],
      negativeExamples: [],
      categoryRules: [],
      aggregation: { window: "localDay", timezoneSource: "referenceTimezone" },
      chatScope: { mode: "allSelectedChats" },
      grouping: { chat: true, sender: "off" },
      captureFromScheduledMessages: false,
      outputPolicy: "aggregateOnly",
      quantityListBounds: {
        minItems: 1,
        maxItems: 20,
        minQuantity: 1,
        maxQuantity: 999,
        maxItemNameVisibleChars: 80,
        maxUnitVisibleChars: 24,
        uncategorizedCategoryId: "uncategorized"
      },
      thresholds: { autoAggregateThresholdMillis: 850, reviewThresholdMillis: 550 }
    })
    const scanRequest = {
      selectedChatIds: [selectedChat.id],
      selectedChats: [selectedChat],
      referenceTimezone: "Asia/Seoul",
      referenceUnixSeconds: 1_783_000_200,
      backfillPromptChatIds: [selectedChat.id],
      sourceExcerptsEnabled: false,
      feedbackTextSnapshotsEnabled: true,
      localDiagnosticsEnabled: true,
      localDiagnosticsRetentionDays: 45,
      listIntakeProfiles: [listIntakeProfile],
      capPolicy: { mode: "refillForPending", maxVisible: 10, pendingCount: 0 }
    } as const
    const discoveryReport = {
      status: "ready",
      chats: [
        {
          chatId: selectedChat.id,
          displayLabel: selectedChat.label,
          participantCount: selectedChat.participantCount,
          participantIds: selectedChat.participantIds,
          latestActivityTimestamp: selectedChat.latestActivityTimestamp
        }
      ]
    } as const
    const previewRequest = { chatIds: [selectedChat.id] } as const
    const previewReport = { chats: [{ chatId: selectedChat.id, preview: "" }] } as const
    tauriMock.invoke
      .mockResolvedValueOnce({
        pendingProposalCount: 4,
        createdCandidateIds: ["candidate-alpha", "candidate-beta"]
      })
      .mockResolvedValueOnce(discoveryReport)
      .mockResolvedValueOnce(previewReport)

    // When
    const scanResult = await scanSelectedChatsInTauri(scanRequest)
    const discoveredChats = await discoverMessagesChatsInTauri()
    const previewRows = await loadMessagesChatPreviewsInTauri(previewRequest)

    // Then
    expect(scanResult).toEqual({
      pendingProposalCount: 4,
      createdCandidateCount: 0,
      quietLogCount: 0,
      createdExternalProposalCount: 0,
      failedExternalProposalCount: 0,
      feedbackLabelCount: 0,
      featureSnapshotCount: 0,
      latestEvalStatus: "never_run",
      createdCandidateIds: ["candidate-alpha", "candidate-beta"]
    })
    expect(discoveredChats).toEqual(discoveryReport)
    expect(previewRows).toEqual(previewReport)
    expect(tauriMock.invoke).toHaveBeenCalledTimes(3)
    expect(tauriMock.invoke).toHaveBeenNthCalledWith(1, "scan_selected_chats", {
      request: scanRequest
    })
    expect(tauriMock.invoke).toHaveBeenNthCalledWith(2, "discover_messages_chats")
    expect(tauriMock.invoke).toHaveBeenNthCalledWith(3, "load_messages_chat_previews", {
      request: previewRequest
    })
  })

  it("rejects malformed native messages command responses at the Tauri edge", async () => {
    // Given
    const {
      discoverMessagesChatsInTauri,
      loadMessagesChatPreviewsInTauri,
      scanSelectedChatsInTauri
    } = await import("./messagesTauriCommands")
    const selectedChat = {
      id: "messages-chat-11111111111111111111111111111111",
      label: "Team planning",
      participantCount: 1,
      participantIds: ["messages-participant-11111111111111111111111111111111"],
      latestActivityTimestamp: 1_783_000_000
    } as const
    const scanRequest = {
      selectedChatIds: [selectedChat.id],
      selectedChats: [selectedChat],
      referenceTimezone: "Asia/Seoul",
      referenceUnixSeconds: 1_783_000_200,
      backfillPromptChatIds: [selectedChat.id],
      sourceExcerptsEnabled: false,
      feedbackTextSnapshotsEnabled: true,
      localDiagnosticsEnabled: false,
      localDiagnosticsRetentionDays: 30,
      listIntakeProfiles: [],
      capPolicy: { mode: "refillForPending", maxVisible: 10, pendingCount: 0 }
    } as const
    const previewRequest = { chatIds: [selectedChat.id] } as const
    tauriMock.invoke
      .mockResolvedValueOnce({ pendingProposalCount: -1 })
      .mockResolvedValueOnce({ status: "permission_denied", chats: [] })
      .mockResolvedValueOnce({ chats: [{ chatId: selectedChat.id, preview: "   " }] })

    // When / Then
    await expect(scanSelectedChatsInTauri(scanRequest)).rejects.toThrow()
    await expect(discoverMessagesChatsInTauri()).rejects.toThrow()
    await expect(loadMessagesChatPreviewsInTauri(previewRequest)).rejects.toThrow()
    expect(tauriMock.invoke).toHaveBeenCalledTimes(3)
  })
})
