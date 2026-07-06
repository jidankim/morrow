import { describe, expect, it } from "vitest"
import { bridgeMock, resetAppShellBridgeTestHarness } from "./AppShellBridgeTestHarness"

const currentProviderUsageReport = {
  generatedAtUnixSeconds: 1_783_000_600,
  window: {
    key: "7d",
    startUnixSeconds: 1_782_395_800,
    endUnixSeconds: 1_783_000_600
  },
  totals: {
    totalOutcomes: 1,
    candidateCount: 1,
    quietCount: 0,
    quietRate: 0,
    candidateRate: 1
  },
  providers: [],
  recentOutcomes: []
} as const

describe("AppShellBridgeTestHarness provider usage mock", () => {
  it("exposes provider usage loading through the mocked bridge", async () => {
    // Given
    resetAppShellBridgeTestHarness()
    bridgeMock.setProviderUsageReport(currentProviderUsageReport)
    const { createNativeShellBridge } = await import("./tauriBridge")
    const bridge = createNativeShellBridge()

    // When
    const report = await bridge.loadProviderUsage({ windowKey: "7d" })

    // Then
    expect(report).toEqual(currentProviderUsageReport)
    expect(bridgeMock.loadProviderUsage).toHaveBeenCalledWith({ windowKey: "7d" })
  })

  it("clears provider usage fixture state on reset", async () => {
    // Given
    bridgeMock.setProviderUsageReport(currentProviderUsageReport)
    resetAppShellBridgeTestHarness()
    const { createNativeShellBridge } = await import("./tauriBridge")
    const bridge = createNativeShellBridge()

    // When
    const report = await bridge.loadProviderUsage()

    // Then
    expect(report).toBeUndefined()
    expect(bridgeMock.loadProviderUsage).toHaveBeenCalledOnce()
  })
})
