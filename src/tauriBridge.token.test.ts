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

describe("createNativeShellBridge token commands", () => {
  beforeEach(() => {
    tauriMock.invoke.mockClear()
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: {}
    })
  })

  it("invokes token commands with the provider token kind", async () => {
    // Given
    const {
      createNativeShellBridge,
      MORROW_KEYCHAIN_SERVICE,
      MORROW_PROVIDER_TOKEN_KIND
    } = await import("./tauriBridge")
    const bridge = createNativeShellBridge()
    const lookup = {
      service: MORROW_KEYCHAIN_SERVICE,
      tokenKind: MORROW_PROVIDER_TOKEN_KIND
    } as const
    tauriMock.invoke
      .mockResolvedValueOnce({ storageSurface: "keychainBridge", stored: true, deleted: false })
      .mockResolvedValueOnce({
        storageSurface: "keychainBridge",
        present: true,
        token: "redacted-provider-token-for-bridge"
      })
      .mockResolvedValueOnce({ storageSurface: "keychainBridge", stored: false, deleted: true })

    // When
    const saved = await bridge.storeMorrowToken({
      ...lookup,
      token: "redacted-provider-token-for-bridge"
    })
    const read = await bridge.readMorrowToken(lookup)
    const deleted = await bridge.deleteMorrowToken(lookup)

    // Then
    expect(saved).toEqual({ storageSurface: "keychainBridge", stored: true, deleted: false })
    expect(read).toEqual({
      storageSurface: "keychainBridge",
      present: true,
      token: "redacted-provider-token-for-bridge"
    })
    expect(deleted).toEqual({ storageSurface: "keychainBridge", stored: false, deleted: true })
    expect(tauriMock.invoke).toHaveBeenNthCalledWith(1, "store_morrow_token", {
      request: { ...lookup, token: "redacted-provider-token-for-bridge" }
    })
    expect(tauriMock.invoke).toHaveBeenNthCalledWith(2, "read_morrow_token", { request: lookup })
    expect(tauriMock.invoke).toHaveBeenNthCalledWith(3, "delete_morrow_token", { request: lookup })
  })
})
