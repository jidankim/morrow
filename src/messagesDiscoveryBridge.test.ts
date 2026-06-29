import { afterEach, describe, expect, it, vi } from "vitest"
import { syncScanRequestFromState } from "./messagesDiscoveryBridge"

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
        firstProposalGuidanceEnabled: true,
        telemetryEnabled: false,
        crashLogExcerptsEnabled: false
      },
      pendingProposalCount: 0,
      selectedChats: [selectedChat]
    })

    // Then
    expect(request.referenceUnixSeconds).toBe(1_783_000_200)
  })
})
