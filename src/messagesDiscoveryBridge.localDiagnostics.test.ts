import { describe, expect, it } from "vitest"
import { parseSyncScanRequest } from "./messagesDiscoveryBridge"

describe("sync scan local diagnostics parsing", () => {
  it("rejects missing or malformed local diagnostics scan settings", () => {
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
      localDiagnosticsEnabled: true,
      localDiagnosticsRetentionDays: 90,
      capPolicy: { mode: "refillForPending", maxVisible: 10, pendingCount: 0 }
    } as const

    // When / Then
    expect(parseSyncScanRequest(baseRequest).localDiagnosticsRetentionDays).toBe(90)
    expect(() =>
      parseSyncScanRequest({
        ...baseRequest,
        localDiagnosticsEnabled: "yes"
      })
    ).toThrow()
    expect(() =>
      parseSyncScanRequest({
        ...baseRequest,
        localDiagnosticsRetentionDays: 0
      })
    ).toThrow()
    expect(() =>
      parseSyncScanRequest({
        ...baseRequest,
        localDiagnosticsRetentionDays: 366
      })
    ).toThrow()
    const { localDiagnosticsEnabled: _missingEnabled, ...missingEnabled } = baseRequest
    expect(() => parseSyncScanRequest(missingEnabled)).toThrow()
    const { localDiagnosticsRetentionDays: _missingRetention, ...missingRetention } = baseRequest
    expect(() => parseSyncScanRequest(missingRetention)).toThrow()
  })
})
