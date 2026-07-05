import { beforeEach, describe, expect, it, vi } from "vitest"

const tauriMock = vi.hoisted(() => ({
  invoke: vi.fn(async (): Promise<unknown> => ({
    pendingProposalCount: 4,
    createdCandidateIds: ["candidate-alpha", "candidate-beta"]
  }))
}))

vi.mock("@tauri-apps/api/core", () => ({
  invoke: tauriMock.invoke
}))

describe("messagesTauriCommands scan result parsing", () => {
  beforeEach(() => {
    tauriMock.invoke.mockClear()
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: {}
    })
  })

  it("preserves created candidate ids from native scan results", async () => {
    // Given
    const { scanSelectedChatsInTauri } = await import("./messagesTauriCommands")
    const scanRequest = {
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
      backfillPromptChatIds: [],
      sourceExcerptsEnabled: false,
      feedbackTextSnapshotsEnabled: true,
      localDiagnosticsEnabled: true,
      localDiagnosticsRetentionDays: 45,
      capPolicy: { mode: "refillForPending", maxVisible: 10, pendingCount: 0 }
    } as const

    // When
    const result = await scanSelectedChatsInTauri(scanRequest)

    // Then
    expect(result).toEqual(
      expect.objectContaining({
        createdCandidateIds: ["candidate-alpha", "candidate-beta"]
      })
    )
  })

  it("rejects malformed decision evidence responses at the native bridge boundary", async () => {
    // Given
    const { createNativeShellBridge } = await import("./tauriBridge")
    tauriMock.invoke.mockResolvedValueOnce({
      items: [
        {
          subjectType: "candidate",
          candidateId: "candidate-alpha",
          candidateState: "draft",
          candidateKind: "calendar_event",
          route: "provider",
          reasonCode: "accepted",
          confidenceMillis: 830,
          labelType: "candidate",
          labelValue: "created",
          sourceExcerptPolicy: "disabled",
          privacyTier: "safe",
          hasDiagnosticsHashes: true,
          createdAt: 1_783_000_010,
          traceRetention: "retained",
          traceSequence: [],
          rawText: "must not cross the schema",
          nativeIdentifier: "/Users/person/Library/Messages/chat.db"
        }
      ],
      skippedTraceLineCount: 0,
      latestEvalStatus: "passed"
    })

    // When / Then
    await expect(
      createNativeShellBridge().loadDecisionEvidence({
        createdCandidateIds: ["candidate-alpha"],
        limit: 20
      })
    ).rejects.toThrow()
  })

  it("passes created candidate ids through the native decision evidence command", async () => {
    // Given
    const { createNativeShellBridge } = await import("./tauriBridge")
    tauriMock.invoke.mockResolvedValueOnce({
      items: [],
      skippedTraceLineCount: 0,
      latestEvalStatus: "passed"
    })

    // When
    const report = await createNativeShellBridge().loadDecisionEvidence({
      createdCandidateIds: ["candidate-alpha"],
      limit: 20
    })

    // Then
    expect(report).toEqual({
      items: [],
      skippedTraceLineCount: 0,
      latestEvalStatus: "passed"
    })
    expect(tauriMock.invoke).toHaveBeenCalledWith("load_decision_evidence", {
      request: {
        createdCandidateIds: ["candidate-alpha"],
        limit: 20
      }
    })
  })
})
