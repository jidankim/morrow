import { describe, expect, it } from "vitest"
import {
  chatDiscoveryFromReport,
  parseMessagesChatPreviewReport,
  parseMessagesDiscoveryReport,
  parseSyncScanRequest
} from "../messagesDiscoveryBridge"

describe("app shell preview privacy boundaries", () => {
  it("maps native display labels to UI labels while rejecting body preview fields", () => {
    const nativeChat = {
      chatId: "messages-chat-11111111111111111111111111111111",
      displayLabel: "Team planning",
      participantCount: 1,
      participantIds: ["messages-participant-11111111111111111111111111111111"],
      latestActivityTimestamp: 1_783_000_000
    } as const
    const report = parseMessagesDiscoveryReport({ status: "ready", chats: [nativeChat] })
    const baseScanRequest = {
      selectedChatIds: [nativeChat.chatId],
      selectedChats: [
        {
          id: nativeChat.chatId,
          label: nativeChat.displayLabel,
          participantCount: nativeChat.participantCount,
          participantIds: nativeChat.participantIds,
          latestActivityTimestamp: nativeChat.latestActivityTimestamp
        }
      ],
      referenceTimezone: "Asia/Seoul",
      referenceUnixSeconds: 1_783_000_200,
      backfillPromptChatIds: [nativeChat.chatId],
      sourceExcerptsEnabled: false,
      feedbackTextSnapshotsEnabled: false,
      capPolicy: { mode: "refillForPending", maxVisible: 10, pendingCount: 0 }
    } as const

    expect(chatDiscoveryFromReport(report).chats[0]).toEqual({
      id: nativeChat.chatId,
      label: "Team planning",
      participantCount: 1,
      participantIds: nativeChat.participantIds,
      latestActivityTimestamp: 1_783_000_000
    })
    expect(() =>
      parseMessagesDiscoveryReport({
        status: "ready",
        chats: [{ ...nativeChat, latestMessageBody: "private clinic visit" }]
      })
    ).toThrow()
    expect(() =>
      parseMessagesDiscoveryReport({
        status: "ready",
        chats: [{ ...nativeChat, latestMessagePreview: "private clinic visit" }]
      })
    ).toThrow()
    expect(() =>
      parseMessagesDiscoveryReport({
        status: "ready",
        chats: [{ ...nativeChat, messagePreview: "private clinic visit" }]
      })
    ).toThrow()
    expect(() =>
      parseMessagesDiscoveryReport({
        status: "ready",
        chats: [{ ...nativeChat, body: "private clinic visit" }]
      })
    ).toThrow()
    expect(
      parseMessagesChatPreviewReport({
        chats: [{ chatId: nativeChat.chatId, preview: "private clinic visit" }]
      })
    ).toEqual({
      chats: [{ chatId: nativeChat.chatId, preview: "private clinic visit" }]
    })
    expect(() =>
      parseSyncScanRequest({ ...baseScanRequest, latestMessagePreview: "private clinic visit" })
    ).toThrow()
    expect(() =>
      parseSyncScanRequest({
        ...baseScanRequest,
        selectedChats: [
          {
            ...baseScanRequest.selectedChats[0],
            latestMessageBody: "private clinic visit"
          }
        ]
      })
    ).toThrow()
  })
})
