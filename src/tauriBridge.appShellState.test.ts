import { beforeEach, describe, expect, it, vi } from "vitest"

const tauriMock = vi.hoisted(() => ({
  invoke: vi.fn(),
  listen: vi.fn()
}))

vi.mock("@tauri-apps/api/core", () => ({
  invoke: tauriMock.invoke
}))

vi.mock("@tauri-apps/api/event", () => ({
  listen: tauriMock.listen
}))

describe("createNativeShellBridge app shell state", () => {
  beforeEach(() => {
    tauriMock.invoke.mockReset()
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: {}
    })
  })

  it("treats native null app shell error messages as absent", async () => {
    // Given
    tauriMock.invoke.mockResolvedValueOnce({
      mode: "scanning",
      errorMessage: null,
      onboardingComplete: true,
      pendingProposalCount: 2
    })
    const { createNativeShellBridge } = await import("./tauriBridge")
    const bridge = createNativeShellBridge()

    // When
    const state = await bridge.getState()

    // Then
    expect(state).toMatchObject({
      mode: "scanning",
      onboardingComplete: true,
      pendingProposalCount: 2
    })
    expect(state?.errorMessage).toBeUndefined()
  })
})
