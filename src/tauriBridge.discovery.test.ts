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

describe("createNativeShellBridge discovery command", () => {
  beforeEach(() => {
    tauriMock.invoke.mockClear()
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: {}
    })
  })

  it("parses native messages discovery reports with discovered metadata", async () => {
    const { parseMessagesDiscoveryReport } = await import("./tauriBridge")
    const report = {
      status: "ready",
      chats: [
        {
          chatId: "messages-chat-44444444444444444444444444444444",
          displayLabel: "Team planning",
          participantCount: 2,
          participantIds: [
            "messages-participant-11111111111111111111111111111111",
            "messages-participant-22222222222222222222222222222222"
          ],
          latestActivityTimestamp: 1_783_000_000
        }
      ]
    } as const

    const parsed = parseMessagesDiscoveryReport(report)

    expect(parsed).toEqual(report)
  })

  it("rejects raw native handles in discovered chat labels", async () => {
    const { chatDisplayMetadata } = await import("./chatDisplay")
    const { parseMessagesDiscoveryReport } = await import("./tauriBridge")
    const fallbackLabelReport = {
      status: "ready",
      chats: [
        {
          chatId: "messages-chat-55555555555555555555555555555555",
          displayLabel: "Messages chat",
          participantCount: 1,
          participantIds: ["messages-participant-55555555555555555555555555555555"],
          latestActivityTimestamp: 1_783_000_000
        }
      ]
    } as const
    const unsafeDisplayMetadata = chatDisplayMetadata(
      {
        id: "messages-chat-66666666666666666666666666666666",
        label: "person@example.com",
        participantCount: 1,
        participantIds: ["messages-participant-66666666666666666666666666666666"],
        latestActivityTimestamp: 1_783_000_000
      },
      "UTC"
    )
    const unsafeLabels = [
      "person@example.com",
      "+1 (555) 123-4567",
      "iMessage;-;+15555550103"
    ] as const

    expect(parseMessagesDiscoveryReport(fallbackLabelReport)).toEqual(fallbackLabelReport)
    expect(Object.values(unsafeDisplayMetadata).join(" ")).not.toContain("person@example.com")
    for (const displayLabel of unsafeLabels) {
      expect(() =>
        parseMessagesDiscoveryReport({
          status: "ready",
          chats: [
            {
              ...fallbackLabelReport.chats[0],
              displayLabel
            }
          ]
        })
      ).toThrow(/raw handles/)
    }
  })

  it("rejects malformed or private-shaped discovery payloads", async () => {
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
    const validNativeChat = {
      chatId: "messages-chat-77777777777777777777777777777777",
      displayLabel: "Team planning",
      participantCount: 1,
      participantIds: ["messages-participant-77777777777777777777777777777777"],
      latestActivityTimestamp: 1_783_000_000
    } as const

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
        status: "ready",
        chats: [
          {
            chatId: validNativeChat.chatId,
            displayLabel: validNativeChat.displayLabel,
            participantCount: validNativeChat.participantCount,
            participantIds: validNativeChat.participantIds
          }
        ]
      })
    ).toThrow()
    expect(() =>
      parseMessagesDiscoveryReport({
        status: "ready",
        chats: [{ ...validNativeChat, latestActivityTimestamp: undefined }]
      })
    ).toThrow()
    expect(() =>
      parseMessagesDiscoveryReport({
        status: "ready",
        chats: [{ ...validNativeChat, latestActivityTimestamp: -1 }]
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
    tauriMock.invoke.mockResolvedValueOnce({ status: "permissionDenied", chats: [] })
    const { createNativeShellBridge } = await import("./tauriBridge")
    const bridge = createNativeShellBridge()

    const result = await bridge.discoverMessagesChats()

    expect(result).toEqual({ status: "permissionDenied", chats: [] })
    expect(tauriMock.invoke).toHaveBeenCalledWith("discover_messages_chats")
  })
})
