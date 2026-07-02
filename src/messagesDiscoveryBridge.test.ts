import { afterEach, describe, expect, it, vi } from "vitest"
import { parseSyncScanRequest, syncScanRequestFromState } from "./messagesDiscoveryBridge"

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

  it("forwards persisted local diagnostics scan settings", () => {
    // Given
    const config = {
      referenceTimezone: "Asia/Seoul",
      calendarSource: "apple-calendar",
      permissionsGranted: true,
      launchAtLogin: false,
      sourceExcerptsEnabled: false,
      feedbackTextSnapshotsEnabled: false,
      firstProposalGuidanceEnabled: true,
      telemetryEnabled: false,
      crashLogExcerptsEnabled: false,
      localDiagnosticsEnabled: true,
      localDiagnosticsRetentionDays: 45
    } as const

    // When
    const request = syncScanRequestFromState({
      config,
      pendingProposalCount: 0,
      selectedChats: [selectedChat]
    })

    // Then
    expect(request.localDiagnosticsEnabled).toBe(true)
    expect(request.localDiagnosticsRetentionDays).toBe(45)
    expect(config.telemetryEnabled).toBe(false)
    expect(config.crashLogExcerptsEnabled).toBe(false)
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
      localDiagnosticsEnabled: false,
      localDiagnosticsRetentionDays: 30,
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
      localDiagnosticsEnabled: false,
      localDiagnosticsRetentionDays: 30,
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
