import { afterEach, describe, expect, it, vi } from "vitest"
import {
  chatDiscoveryFromReport,
  parseMessagesChatPreviewReport,
  parseMessagesChatPreviewRequest,
  parseMessagesDiscoveryReport,
  parseSyncScanRequest,
  syncScanRequestFromState
} from "./messagesDiscoveryBridge"

const selectedChat = {
  id: "messages-chat-11111111111111111111111111111111",
  label: "Team planning",
  participantCount: 1,
  participantIds: ["messages-participant-11111111111111111111111111111111"],
  latestActivityTimestamp: 1_783_000_000,
  backfillPromptEnabled: true
} as const

describe("syncScanRequestFromState", () => {
  afterEach(() => {
    vi.useRealTimers()
  })

  it("captures the current reference timestamp for native scan windows", () => {
    // Given
    vi.useFakeTimers()
    vi.setSystemTime(new Date("2026-07-02T13:50:00Z"))

    // When
    const request = syncScanRequestFromState({
      config: {
        referenceTimezone: "Asia/Seoul",
        calendarSource: "apple-calendar",
        permissionsGranted: true,
        launchAtLogin: false,
        sourceExcerptsEnabled: false,
        feedbackTextSnapshotsEnabled: true,
        firstProposalGuidanceEnabled: true,
        telemetryEnabled: false,
        crashLogExcerptsEnabled: false,
        localDiagnosticsEnabled: false,
        localDiagnosticsRetentionDays: 30
      },
      pendingProposalCount: 0,
      selectedChats: [selectedChat]
    })

    // Then
    expect(request.referenceUnixSeconds).toBe(1_783_000_200)
    expect(request.feedbackTextSnapshotsEnabled).toBe(true)
  })

  it("rejects missing or malformed feedback text snapshot consent", () => {
    // Given
    const baseRequest = {
      selectedChatIds: ["messages-chat-11111111111111111111111111111111"],
      selectedChats: [
        {
          id: "messages-chat-11111111111111111111111111111111",
          label: "Team planning",
          participantCount: 1,
          participantIds: ["messages-participant-11111111111111111111111111111111"],
          latestActivityTimestamp: 1_783_000_000
        }
      ],
      referenceTimezone: "Asia/Seoul",
      referenceUnixSeconds: 1_783_000_200,
      backfillPromptChatIds: ["messages-chat-11111111111111111111111111111111"],
      sourceExcerptsEnabled: true,
      feedbackTextSnapshotsEnabled: true,
      capPolicy: { mode: "refillForPending", maxVisible: 10, pendingCount: 0 }
    } as const

    // When / Then
    expect(parseSyncScanRequest(baseRequest).feedbackTextSnapshotsEnabled).toBe(true)
    expect(() =>
      parseSyncScanRequest({
        ...baseRequest,
        feedbackTextSnapshotsEnabled: -1
      })
    ).toThrow()
    const { feedbackTextSnapshotsEnabled: _missing, ...missingConsent } = baseRequest
    expect(() => parseSyncScanRequest(missingConsent)).toThrow()
  })

  it("rejects preview fields from native scan requests", () => {
    // Given
    const baseRequest = {
      selectedChatIds: ["messages-chat-11111111111111111111111111111111"],
      selectedChats: [
        {
          id: "messages-chat-11111111111111111111111111111111",
          label: "Team planning",
          participantCount: 1,
          participantIds: ["messages-participant-11111111111111111111111111111111"],
          latestActivityTimestamp: 1_783_000_000
        }
      ],
      referenceTimezone: "Asia/Seoul",
      referenceUnixSeconds: 1_783_000_200,
      backfillPromptChatIds: ["messages-chat-11111111111111111111111111111111"],
      sourceExcerptsEnabled: true,
      feedbackTextSnapshotsEnabled: true,
      capPolicy: { mode: "refillForPending", maxVisible: 10, pendingCount: 0 }
    } as const

    // When / Then
    expect(() =>
      parseSyncScanRequest({ ...baseRequest, previews: { [selectedChat.id]: "private clinic visit" } })
    ).toThrow()
    expect(() =>
      parseSyncScanRequest({
        ...baseRequest,
        selectedChats: [{ ...baseRequest.selectedChats[0], messagePreview: "private clinic visit" }]
      })
    ).toThrow()
  })
})

describe("opt-in chat preview bridge parsing", () => {
  it("parses discovery command reports and rejects malformed command shapes", () => {
    // Given
    const report = {
      status: "ready",
      chats: [{ chatId: selectedChat.id, displayLabel: selectedChat.label, participantCount: 1, participantIds: selectedChat.participantIds, latestActivityTimestamp: selectedChat.latestActivityTimestamp }]
    } as const

    // When
    const parsed = parseMessagesDiscoveryReport(report)

    // Then
    expect(chatDiscoveryFromReport(parsed)).toEqual({
      status: "ready",
      chats: [{ id: selectedChat.id, label: selectedChat.label, participantCount: 1, participantIds: selectedChat.participantIds, latestActivityTimestamp: selectedChat.latestActivityTimestamp }]
    })
    expect(() => parseMessagesDiscoveryReport({ status: "permission_denied", chats: [] })).toThrow()
    expect(() => parseMessagesDiscoveryReport({ status: "ready", chats: [] })).toThrow()
  })

  it("parses opt-in chat preview reports separately from discovery", () => {
    // Given
    const request = {
      chatIds: [
        "messages-chat-11111111111111111111111111111111",
        "messages-chat-22222222222222222222222222222222"
      ]
    } as const
    const report = {
      chats: [
        {
          chatId: "messages-chat-11111111111111111111111111111111",
          preview: "private clinic visit"
        },
        {
          chatId: "messages-chat-22222222222222222222222222222222",
          preview: "team sync moved to 3"
        }
      ]
    } as const

    // When
    const parsedRequest = parseMessagesChatPreviewRequest(request)
    const parsedReport = parseMessagesChatPreviewReport(report)

    // Then
    expect(parsedRequest).toEqual(request)
    expect(parsedReport).toEqual(report)
    expect(() =>
      parseMessagesChatPreviewReport({
        previews: [
          {
            chatId: "messages-chat-11111111111111111111111111111111",
            preview: "private clinic visit"
          }
        ]
      })
    ).toThrow()
    expect(() =>
      parseMessagesDiscoveryReport({
        status: "ready",
        chats: [
          {
            chatId: "messages-chat-11111111111111111111111111111111",
            displayLabel: "Team planning",
            participantCount: 1,
            participantIds: ["messages-participant-11111111111111111111111111111111"],
            latestActivityTimestamp: 1_783_000_000,
            latestMessagePreview: "private clinic visit"
          }
        ]
      })
    ).toThrow()
  })

  it("rejects whitespace-only opt-in chat preview rows", () => {
    // Given
    const report = {
      chats: [{ chatId: "messages-chat-11111111111111111111111111111111", preview: "   " }]
    } as const

    // When / Then
    expect(() => parseMessagesChatPreviewReport(report)).toThrow()
  })

  it("accepts native empty opt-in chat preview rows", () => {
    // Given
    const report = {
      chats: [{ chatId: "messages-chat-11111111111111111111111111111111", preview: "" }]
    } as const

    // When / Then
    expect(parseMessagesChatPreviewReport(report)).toEqual(report)
  })

  it("accepts native-capped preview rows and rejects malformed preview rows", () => {
    // Given
    const nativeCappedPreview = `${"x".repeat(120)}...`
    const tooLongPreview = "x".repeat(124)
    const malformedReports = [
      {
        name: "raw chat id",
        report: { chats: [{ chatId: "iMessage;-;+15555550103", preview: "private clinic visit" }] }
      },
      {
        name: "raw handle",
        report: { chats: [{ chatId: "+15555550103", preview: "private clinic visit" }] }
      },
      {
        name: "uncapped over native visible limit",
        report: {
          chats: [
            { chatId: "messages-chat-11111111111111111111111111111111", preview: "x".repeat(121) }
          ]
        }
      },
      {
        name: "too long preview",
        report: {
          chats: [
            { chatId: "messages-chat-11111111111111111111111111111111", preview: tooLongPreview }
          ]
        }
      },
      {
        name: "passthrough body",
        report: {
          chats: [
            {
              chatId: "messages-chat-11111111111111111111111111111111",
              preview: "private clinic visit",
              body: "private clinic visit"
            }
          ]
        }
      }
    ] as const

    // When / Then
    expect(
      parseMessagesChatPreviewReport({
        chats: [
          { chatId: "messages-chat-11111111111111111111111111111111", preview: nativeCappedPreview }
        ]
      })
    ).toEqual({
      chats: [
        { chatId: "messages-chat-11111111111111111111111111111111", preview: nativeCappedPreview }
      ]
    })
    expect(() => parseMessagesChatPreviewRequest({ chatIds: ["+15555550103"] })).toThrow()
    expect(() => parseMessagesChatPreviewRequest({ chats: [selectedChat.id] })).toThrow()
    for (const testCase of malformedReports) {
      expect(() => parseMessagesChatPreviewReport(testCase.report), testCase.name).toThrow()
    }
  })
})
