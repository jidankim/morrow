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

const scanRequest = {
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
  backfillPromptChatIds: ["messages-chat-11111111111111111111111111111111"],
  sourceExcerptsEnabled: false,
  capPolicy: { mode: "refillForPending", maxVisible: 10, pendingCount: 7 }
} as const

describe("createNativeShellBridge sync result counts", () => {
  beforeEach(() => {
    tauriMock.invoke.mockClear()
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: {}
    })
  })

  it("parses native sync results with candidate, quiet log, and external proposal counts", async () => {
    // Given
    tauriMock.invoke.mockResolvedValueOnce({
      pendingProposalCount: 5,
      createdCandidateCount: 4,
      quietLogCount: 2,
      createdExternalProposalCount: 3,
      failedExternalProposalCount: 1
    })
    const { createNativeShellBridge } = await import("./tauriBridge")

    // When
    const result = await createNativeShellBridge().scanSelectedChats(scanRequest)

    // Then
    expect(result).toEqual({
      pendingProposalCount: 5,
      createdCandidateCount: 4,
      quietLogCount: 2,
      createdExternalProposalCount: 3,
      failedExternalProposalCount: 1
    })
  })

  it("defaults missing new sync result counts for legacy native payloads", async () => {
    // Given
    const { createNativeShellBridge } = await import("./tauriBridge")

    // When
    const result = await createNativeShellBridge().scanSelectedChats(scanRequest)

    // Then
    expect(result).toEqual({
      pendingProposalCount: 3,
      createdCandidateCount: 0,
      quietLogCount: 0,
      createdExternalProposalCount: 0,
      failedExternalProposalCount: 0
    })
  })

  it("rejects malformed sync result count shapes before returning evidence", async () => {
    // Given
    tauriMock.invoke.mockResolvedValueOnce({
      pendingProposalCount: 1,
      createdCandidateCount: -1,
      quietLogCount: 0,
      createdExternalProposalCount: 0,
      failedExternalProposalCount: 0
    })
    const { createNativeShellBridge } = await import("./tauriBridge")

    // When / Then
    await expect(createNativeShellBridge().scanSelectedChats(scanRequest)).rejects.toThrow()
  })
})
