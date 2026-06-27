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
      selectedChatIds: ["chat-alpha"],
      referenceTimezone: "Asia/Seoul",
      backfillPromptChatIds: ["chat-alpha"],
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
})
