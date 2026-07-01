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
    tauriMock.listen.mockReset()
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
      pendingProposalCount: 2,
      automaticSyncEnabled: true,
      automaticSyncStatusLabel: "Cooling Down",
      automaticSyncDetail: "Retrying after a transient failure."
    })
    const { createNativeShellBridge } = await import("./tauriBridge")
    const bridge = createNativeShellBridge()

    // When
    const state = await bridge.getState()

    // Then
    expect(state).toMatchObject({
      mode: "scanning",
      onboardingComplete: true,
      pendingProposalCount: 2,
      automaticSyncEnabled: true,
      automaticSyncStatusLabel: "Cooling Down",
      automaticSyncDetail: "Retrying after a transient failure."
    })
    expect(state?.errorMessage).toBeUndefined()
  })

  it("rejects malformed automatic sync status labels from native shell state", async () => {
    // Given
    tauriMock.invoke.mockResolvedValueOnce({
      mode: "scanning",
      onboardingComplete: true,
      pendingProposalCount: 0,
      automaticSyncEnabled: true,
      automaticSyncStatusLabel: "Running",
      automaticSyncDetail: "Scanning now."
    })
    const { createNativeShellBridge } = await import("./tauriBridge")
    const bridge = createNativeShellBridge()

    // When / Then
    await expect(bridge.getState()).rejects.toThrow()
  })

  it("maps native automatic sync toggle events without invoking scan commands", async () => {
    // Given
    type NativeMenuEvent = { readonly payload: unknown }
    const listeners = new Map<string, (event: NativeMenuEvent) => void>()
    tauriMock.listen.mockImplementation(
      async (eventName: string, listener: (event: NativeMenuEvent) => void) => {
        listeners.set(eventName, listener)
        return vi.fn()
      }
    )
    const { createNativeShellBridge } = await import("./tauriBridge")
    const bridge = createNativeShellBridge()
    const commands: string[] = []

    // When
    await bridge.subscribeMenuCommand((command) => {
      commands.push(command)
    })
    listeners.get("morrow://toggle-automatic-sync")?.({ payload: undefined })

    // Then
    expect(commands).toEqual(["toggle-automatic-sync"])
    expect(tauriMock.invoke).not.toHaveBeenCalled()
    expect(Array.from(listeners.keys())).toEqual([
      "morrow://sync-now",
      "morrow://open-settings",
      "morrow://open-calendar",
      "morrow://open-reminders",
      "morrow://toggle-automatic-sync"
    ])
  })
})
