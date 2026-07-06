import { describe, expect, it } from "vitest"

describe("provider setup bridge parsers", () => {
  it("parses Codex CLI install and login launch receipts", async () => {
    // Given
    const { parseCodexCliInstallReceipt, parseCodexLoginLaunchReceipt } = await import(
      "./providerSetupBridge"
    )
    const installReceipt = {
      status: "alreadyInstalled",
      commandSurface: "codex standalone installer",
      commandOutputRedacted: true,
      diagnostic: "Codex CLI is already installed."
    } as const
    const loginReceipt = {
      status: "missingCli",
      commandSurface: "codex login",
      commandOutputRedacted: true,
      diagnostic: "Codex CLI was not found on PATH."
    } as const

    // When
    const parsedInstallReceipt = parseCodexCliInstallReceipt(installReceipt)
    const parsedLoginReceipt = parseCodexLoginLaunchReceipt(loginReceipt)

    // Then
    expect(parsedInstallReceipt).toEqual(installReceipt)
    expect(parsedLoginReceipt).toEqual(loginReceipt)
  })

  it("rejects setup payloads that are not redacted", async () => {
    // Given
    const { parseCodexCliInstallReceipt, parseCodexLoginLaunchReceipt } = await import(
      "./providerSetupBridge"
    )
    const malformedPayload = {
      status: "installed",
      commandSurface: "codex standalone installer",
      commandOutputRedacted: false,
      diagnostic: "Codex CLI installed successfully."
    } as const

    // When / Then
    expect(() => parseCodexCliInstallReceipt(malformedPayload)).toThrow()
    expect(() => parseCodexLoginLaunchReceipt(malformedPayload)).toThrow()
  })
})
