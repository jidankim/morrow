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
        pendingCount: 7
      }
    } as const

    // When
    const result = await bridge.scanSelectedChats(request)

    // Then
    expect(result).toEqual({ pendingProposalCount: 3 })
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
      participantIds: ["messages-participant-11111111111111111111111111111111", "messages-participant-22222222222222222222222222222222"],
      latestActivityTimestamp: 1_783_000_000
    } as const
    const baseRequest = {
      selectedChatIds: ["messages-chat-33333333333333333333333333333333"],
      selectedChats: [selectedChat],
      referenceTimezone: "Asia/Seoul",
      backfillPromptChatIds: ["messages-chat-33333333333333333333333333333333"],
      sourceExcerptsEnabled: false,
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

  it("parses native messages discovery reports with discovered metadata", async () => {
    // Given
    const { parseMessagesDiscoveryReport } = await import("./tauriBridge")
    const report = {
      status: "ready",
      chats: [
        {
          chatId: "messages-chat-44444444444444444444444444444444",
          displayLabel: "Team planning",
          participantCount: 2,
          participantIds: ["messages-participant-11111111111111111111111111111111", "messages-participant-22222222222222222222222222222222"],
          latestActivityTimestamp: 1_783_000_000
        }
      ]
    } as const

    // When
    const parsed = parseMessagesDiscoveryReport(report)

    // Then
    expect(parsed).toEqual(report)
  })

  it("rejects malformed or private-shaped discovery payloads", async () => {
    // Given
    const { parseMessagesDiscoveryReport } = await import("./tauriBridge")
    const privatePayload = {
      status: "ready",
      chats: [
        {
          chatId: "iMessage;-;+15555550103",
          displayLabel: "person@example.com",
          participantCount: 1,
          participantIds: ["+1 (555) 123-4567"],
          latestActivityTimestamp: 1_783_000_000,
          rawHandle: "person@example.com"
        }
      ]
    }

    // When / Then
    expect(() => parseMessagesDiscoveryReport({ status: "blocked", chats: [] })).toThrow()
    expect(() => parseMessagesDiscoveryReport(privatePayload)).toThrow()
    expect(() =>
      parseMessagesDiscoveryReport({
        status: "ready",
        chats: [
          {
            chatGuid: "iMessage;-;+15555550103",
            displayLabel: "Team planning",
            participantCount: 1,
            participantIds: ["messages-participant-11111111111111111111111111111111"],
            latestActivityTimestamp: 1_783_000_000
          }
        ]
      })
    ).toThrow()
    expect(() =>
      parseMessagesDiscoveryReport({
        status: "permissionDenied",
        chats: [
          {
            chatId: "messages-chat-11111111111111111111111111111111",
            displayLabel: "Team planning",
            participantCount: 1,
            participantIds: ["messages-participant-11111111111111111111111111111111"],
            latestActivityTimestamp: 1_783_000_000
          }
        ]
      })
    ).toThrow()
  })

  it("invokes native discover_messages_chats and parses degraded states", async () => {
    // Given
    tauriMock.invoke.mockResolvedValueOnce({ status: "permissionDenied", chats: [] })
    const { createNativeShellBridge } = await import("./tauriBridge")
    const bridge = createNativeShellBridge()

    // When
    const result = await bridge.discoverMessagesChats()

    // Then
    expect(result).toEqual({ status: "permissionDenied", chats: [] })
    expect(tauriMock.invoke).toHaveBeenCalledWith("discover_messages_chats")
  })
})
