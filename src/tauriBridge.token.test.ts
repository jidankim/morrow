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

  it("invokes provider auth readiness without token storage", async () => {
    const { createNativeShellBridge } = await import("./tauriBridge")
    const bridge = createNativeShellBridge()
    tauriMock.invoke.mockResolvedValueOnce({
      status: "loggedInUsingChatGpt",
      ready: true,
      commandSurface: "codex login status",
      commandOutputRedacted: true,
      diagnostic: "Codex CLI ChatGPT session is ready."
    })

    const readiness = await bridge.checkProviderAuth()

    expect(readiness).toEqual({
      status: "loggedInUsingChatGpt",
      ready: true,
      commandSurface: "codex login status",
      commandOutputRedacted: true,
      diagnostic: "Codex CLI ChatGPT session is ready."
    })
    expect(tauriMock.invoke).toHaveBeenCalledOnce()
    expect(tauriMock.invoke).toHaveBeenCalledWith("check_provider_auth")
  })

  it("rejects malformed provider auth readiness from the bridge", async () => {
    const { createNativeShellBridge } = await import("./tauriBridge")
    const bridge = createNativeShellBridge()
    tauriMock.invoke
      .mockResolvedValueOnce({
        status: "rawProviderOutput",
        ready: true,
        commandSurface: "codex login status",
        commandOutputRedacted: false,
        diagnostic: "untrusted"
      })
      .mockResolvedValueOnce({
        ready: false,
        commandSurface: "codex login status",
        commandOutputRedacted: true,
        diagnostic: "missing status"
      })

    await expect(bridge.checkProviderAuth()).rejects.toThrow()
    await expect(bridge.checkProviderAuth()).rejects.toThrow()
  })
})
