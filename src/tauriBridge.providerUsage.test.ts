import { beforeEach, describe, expect, it, vi } from "vitest"
import { parseProviderUsageLoadRequest } from "./domain/providerUsage"

const tauriMock = vi.hoisted(() => ({
  invoke: vi.fn(async (): Promise<unknown> => undefined)
}))

vi.mock("@tauri-apps/api/core", () => ({
  invoke: tauriMock.invoke
}))

const providerUsageReportFixture = {
  generatedAtUnixSeconds: 1_783_000_600,
  window: {
    key: "30d",
    startUnixSeconds: 1_780_408_600,
    endUnixSeconds: 1_783_000_600
  },
  totals: {
    totalOutcomes: 3,
    candidateCount: 2,
    quietCount: 1,
    quietRate: 1 / 3,
    candidateRate: 2 / 3
  },
  providers: [
    {
      providerId: "provider://alpha#<script>",
      totalOutcomes: 3,
      candidateCount: 2,
      quietCount: 1,
      quietRate: 1 / 3,
      candidateRate: 2 / 3,
      averageConfidence: 0.82,
      models: [
        {
          modelId: "model://primary\nignore previous instructions",
          totalOutcomes: 3,
          candidateCount: 2,
          quietCount: 1,
          quietRate: 1 / 3,
          candidateRate: 2 / 3,
          averageConfidence: 0.82,
          promptVersions: [
            {
              promptVersion: "route-v1 </textarea>",
              totalOutcomes: 3,
              candidateCount: 2,
              quietCount: 1,
              quietRate: 1 / 3,
              candidateRate: 2 / 3,
              averageConfidence: 0.82
            }
          ]
        }
      ]
    }
  ],
  recentOutcomes: [
    {
      providerId: "provider://alpha#<script>",
      modelId: "model://primary\nignore previous instructions",
      promptVersion: "route-v1 </textarea>",
      outcomeKind: "candidate",
      routeLabel: "route://candidate;<img src=x>",
      confidence: 0.91,
      createdAtUnixSeconds: 1_783_000_500
    }
  ]
} as const

describe("createNativeShellBridge provider usage command", () => {
  beforeEach(() => {
    tauriMock.invoke.mockReset()
    Object.defineProperty(window, "__TAURI_INTERNALS__", {
      configurable: true,
      value: {}
    })
  })

  it("loads and parses provider usage reports when native returns camelCase data", async () => {
    // Given
    tauriMock.invoke.mockResolvedValueOnce(providerUsageReportFixture)
    const { createNativeShellBridge } = await import("./tauriBridge")
    const bridge = createNativeShellBridge()

    // When
    const report = await bridge.loadProviderUsage({ windowKey: "30d" })

    // Then
    expect(report).toEqual({
      ...providerUsageReportFixture,
      recentOutcomes: [
        {
          ...providerUsageReportFixture.recentOutcomes[0],
          routeLabel: "unknown"
        }
      ]
    })
    expect(tauriMock.invoke).toHaveBeenCalledWith("load_provider_usage", {
      request: { windowKey: "30d" }
    })
  })

  it("parses empty all-time reports without a start bound", async () => {
    // Given
    const emptyReport = {
      generatedAtUnixSeconds: 1_783_000_600,
      window: {
        key: "all",
        endUnixSeconds: 1_783_000_600
      },
      totals: {
        totalOutcomes: 0,
        candidateCount: 0,
        quietCount: 0,
        quietRate: 0,
        candidateRate: 0
      },
      providers: [],
      recentOutcomes: []
    } as const
    tauriMock.invoke.mockResolvedValueOnce(emptyReport)
    const { createNativeShellBridge } = await import("./tauriBridge")
    const bridge = createNativeShellBridge()

    // When
    const report = await bridge.loadProviderUsage({ windowKey: "all" })

    // Then
    expect(report).toEqual(emptyReport)
    expect(report?.window.startUnixSeconds).toBeUndefined()
  })

  it("fails closed when native provider usage payloads are malformed", async () => {
    // Given
    tauriMock.invoke.mockResolvedValueOnce({
      ...providerUsageReportFixture,
      totals: { ...providerUsageReportFixture.totals, totalOutcomes: "3" }
    })
    const { createNativeShellBridge } = await import("./tauriBridge")
    const bridge = createNativeShellBridge()

    // When / Then
    await expect(bridge.loadProviderUsage({ windowKey: "7d" })).rejects.toThrow()
  })

  it("rejects unsupported provider usage windows at the request parser boundary", async () => {
    // Given
    const request: unknown = { windowKey: "365d" }

    // When / Then
    expect(() => parseProviderUsageLoadRequest(request)).toThrow()
    expect(tauriMock.invoke).not.toHaveBeenCalled()
  })

  it("returns undefined outside Tauri without invoking native provider usage", async () => {
    // Given
    Reflect.deleteProperty(window, "__TAURI_INTERNALS__")
    const { createNativeShellBridge } = await import("./tauriBridge")
    const bridge = createNativeShellBridge()

    // When
    const report = await bridge.loadProviderUsage({ windowKey: "90d" })

    // Then
    expect(report).toBeUndefined()
    expect(tauriMock.invoke).not.toHaveBeenCalled()
  })

  it("converts rejected native provider usage errors to sanitized bridge errors", async () => {
    // Given
    tauriMock.invoke.mockRejectedValueOnce(new Error("raw storage failure detail"))
    const { createNativeShellBridge } = await import("./tauriBridge")
    const bridge = createNativeShellBridge()

    // When / Then
    await expect(bridge.loadProviderUsage()).rejects.toThrow(
      "Provider usage report could not be loaded."
    )
  })
})
