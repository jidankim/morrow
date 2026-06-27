import { describe, expect, it } from "vitest"
import { APP_SHELL_STATE_KEY, loadAppShellState } from "./appShell"
import {
  chatDiscoveryFromReport,
  parseMessagesDiscoveryReport,
  parseSyncScanRequest
} from "../messagesDiscoveryBridge"

describe("app shell privacy boundaries", () => {
  it("rejects raw Messages chat guid shaped selected ids in local storage", () => {
    const storage = new Map<string, string>([
      [
        APP_SHELL_STATE_KEY,
        JSON.stringify({
          mode: "scanning",
          config: {
            referenceTimezone: "Asia/Seoul",
            calendarSource: "apple-calendar",
            permissionsGranted: true,
            launchAtLogin: false,
            sourceExcerptsEnabled: true,
            firstProposalGuidanceEnabled: true
          },
          discovery: { status: "unverified", chats: [] },
          selectedChats: [
            {
              id: "iMessage;-;+15555550103",
              label: "Chat alpha",
              participantCount: 1,
              participantIds: ["messages-participant-11111111111111111111111111111111"],
              latestActivityTimestamp: 1_783_000_000,
              backfillPromptEnabled: true
            }
          ],
          pendingProposalCount: 0
        })
      ]
    ])

    expect(() => loadAppShellState(storage)).toThrow()
  })

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
      backfillPromptChatIds: [nativeChat.chatId],
      sourceExcerptsEnabled: false,
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
      parseSyncScanRequest({ ...baseScanRequest, latestMessagePreview: "private clinic visit" })
    ).toThrow()
  })
})
