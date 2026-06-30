import { beforeEach, describe, expect, it, vi } from "vitest"

const tauriMock = vi.hoisted(() => ({
  invoke: vi.fn(async (): Promise<unknown> => ({ pendingProposalCount: 3 })),
  listen: vi.fn()
}))

vi.mock("@tauri-apps/api/core", () => ({
  invoke: tauriMock.invoke
}))

vi.mock("@tauri-apps/api/event", () => ({
  listen: tauriMock.listen
}))

describe("createNativeShellBridge scan command", () => {
  beforeEach(() => {
    tauriMock.invoke.mockClear()
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: {}
    })
  })

  it("invokes native scan_selected_chats with source excerpt and cap policy", async () => {
    // Given
    const { createNativeShellBridge } = await import("./tauriBridge")
    const bridge = createNativeShellBridge()
    const request = {
      selectedChatIds: ["messages-chat-11111111111111111111111111111111"],
      selectedChats: [
        {
          id: "messages-chat-11111111111111111111111111111111",
          label: "Team planning",
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
      sourceExcerptsEnabled: false,
      feedbackTextSnapshotsEnabled: false,
      capPolicy: {
        mode: "refillForPending",
        maxVisible: 10,
        pendingCount: 7
      }
    } as const

    // When
    const result = await bridge.scanSelectedChats(request)

    // Then
    expect(result).toEqual({
      pendingProposalCount: 3,
      createdCandidateCount: 0,
      quietLogCount: 0,
      createdExternalProposalCount: 0,
      failedExternalProposalCount: 0,
      feedbackLabelCount: 0,
      featureSnapshotCount: 0,
      latestEvalStatus: "never_run"
    })
    expect(tauriMock.invoke).toHaveBeenCalledWith("scan_selected_chats", { request })
  })

  it("rejects raw handle-shaped scan chat ids before invoking native scan", async () => {
    // Given
    const { createNativeShellBridge } = await import("./tauriBridge")
    const bridge = createNativeShellBridge()
    const selectedChat = {
      id: "messages-chat-33333333333333333333333333333333",
      label: "Team planning",
      participantCount: 2,
      participantIds: [
        "messages-participant-11111111111111111111111111111111",
        "messages-participant-22222222222222222222222222222222"
      ],
      latestActivityTimestamp: 1_783_000_000
    } as const
    const baseRequest = {
      selectedChatIds: ["messages-chat-33333333333333333333333333333333"],
      selectedChats: [selectedChat],
      referenceTimezone: "Asia/Seoul",
      referenceUnixSeconds: 1_783_000_200,
      backfillPromptChatIds: ["messages-chat-33333333333333333333333333333333"],
      sourceExcerptsEnabled: false,
      feedbackTextSnapshotsEnabled: false,
      capPolicy: {
        mode: "refillForPending",
        maxVisible: 10,
        pendingCount: 7
      }
    } as const
    const cases = [
      {
        name: "selectedChatIds",
        request: { ...baseRequest, selectedChatIds: ["+15555550103"] }
      },
      {
        name: "selectedChats id",
        request: {
          ...baseRequest,
          selectedChats: [{ ...selectedChat, id: "person@example.com" }]
        }
      },
      {
        name: "backfillPromptChatIds",
        request: { ...baseRequest, backfillPromptChatIds: ["iMessage;-;+15555550103"] }
      }
    ] as const

    for (const testCase of cases) {
      // When / Then
      tauriMock.invoke.mockClear()
      await expect(bridge.scanSelectedChats(testCase.request), testCase.name).rejects.toThrow()
      expect(tauriMock.invoke, testCase.name).not.toHaveBeenCalled()
    }
  })

  it("invokes native reconcile_now before scan orchestration uses it", async () => {
    // Given
    const { createNativeShellBridge } = await import("./tauriBridge")
    const bridge = createNativeShellBridge()

    // When
    await bridge.reconcileNow()

    // Then
    expect(tauriMock.invoke).toHaveBeenCalledWith("reconcile_now")
  })

  it("invokes native load_messages_chat_previews with opaque chat ids", async () => {
    // Given
    const report = {
      chats: [
        {
          chatId: "messages-chat-11111111111111111111111111111111",
          preview: ""
        }
      ]
    } as const
    tauriMock.invoke.mockResolvedValueOnce(report)
    const { createNativeShellBridge } = await import("./tauriBridge")
    const bridge = createNativeShellBridge()
    const request = {
      chatIds: ["messages-chat-11111111111111111111111111111111"]
    } as const

    // When
    const result = await bridge.loadMessagesChatPreviews(request)

    // Then
    expect(result).toEqual(report)
    expect(tauriMock.invoke).toHaveBeenCalledWith("load_messages_chat_previews", { request })
  })

  it("rejects malformed preview requests before invoking native previews", async () => {
    // Given
    const { createNativeShellBridge } = await import("./tauriBridge")
    const bridge = createNativeShellBridge()
    const request = {
      chatIds: ["+15555550103"]
    } as const

    // When / Then
    await expect(bridge.loadMessagesChatPreviews(request)).rejects.toThrow()
    expect(tauriMock.invoke).not.toHaveBeenCalled()
  })

  it("surfaces rejected native preview promises without partial preview data", async () => {
    // Given
    const previewError = new Error("preview load failed")
    tauriMock.invoke.mockRejectedValueOnce(previewError)
    const { createNativeShellBridge } = await import("./tauriBridge")
    const bridge = createNativeShellBridge()
    const request = {
      chatIds: ["messages-chat-11111111111111111111111111111111"]
    } as const
    let previewReport: Awaited<ReturnType<typeof bridge.loadMessagesChatPreviews>> | undefined

    // When / Then
    await expect(async () => {
      previewReport = await bridge.loadMessagesChatPreviews(request)
    }).rejects.toThrow(previewError)
    expect(previewReport).toBeUndefined()
  })

  it("invokes native open_privacy_settings with a typed pane request", async () => {
    // Given
    tauriMock.invoke.mockResolvedValueOnce({ pane: "calendar", opened: true })
    const { createNativeShellBridge } = await import("./tauriBridge")
    const bridge = createNativeShellBridge()
    const request = { pane: "calendar" } as const

    // When
    const result = await bridge.openPrivacySettings(request)

    // Then
    expect(result).toEqual({ pane: "calendar", opened: true })
    expect(tauriMock.invoke).toHaveBeenCalledWith("open_privacy_settings", { request })
  })

})
