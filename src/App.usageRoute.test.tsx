import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react"
import { beforeEach, describe, expect, it } from "vitest"
import {
  bridgeMock,
  resetAppShellBridgeTestHarness,
  seedReadyState
} from "./AppShellBridgeTestHarness"
import { App } from "./App"
import type { ProviderUsageReport } from "./domain/providerUsage"

const currentProviderUsageReport = {
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
      models: []
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
} as const satisfies ProviderUsageReport

const emptyProviderUsageReport = {
  ...currentProviderUsageReport,
  totals: {
    totalOutcomes: 0,
    candidateCount: 0,
    quietCount: 0,
    quietRate: 0,
    candidateRate: 0
  },
  providers: [],
  recentOutcomes: []
} as const satisfies ProviderUsageReport

const sevenDayProviderUsageReport = {
  ...currentProviderUsageReport,
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
} as const satisfies ProviderUsageReport

const openRoute = (hash: string): void => {
  act(() => {
    window.location.hash = hash
    window.dispatchEvent(new HashChangeEvent("hashchange"))
  })
}

describe("App Usage route", () => {
  beforeEach(() => {
    resetAppShellBridgeTestHarness()
    seedReadyState()
    bridgeMock.getRuntimeIdentity.mockClear()
  })

  it("selects Usage route from #usage and marks navigation active", async () => {
    // Given
    bridgeMock.setProviderUsageReport(currentProviderUsageReport)
    window.location.hash = "#usage"

    // When
    render(<App />)

    // Then
    expect(await screen.findByRole("heading", { name: "Usage" })).toBeInTheDocument()
    expect(screen.getByRole("link", { name: "Usage" })).toHaveAttribute("aria-current", "page")
    expect(screen.getByRole("link", { name: "Usage" })).toHaveAttribute("href", "#usage")
    expect(screen.getByRole("link", { name: "Status" })).not.toHaveAttribute("aria-current")
  })

  it("calls the bridge with the default provider usage window when Usage route is active", async () => {
    // Given
    bridgeMock.setProviderUsageReport(currentProviderUsageReport)
    window.location.hash = "#usage"

    // When
    render(<App />)

    // Then
    await waitFor(() =>
      expect(bridgeMock.loadProviderUsage).toHaveBeenCalledWith({ windowKey: "30d" })
    )
    const summary = await screen.findByLabelText("Provider usage summary")
    expect(within(summary).getByText("3")).toBeInTheDocument()
    expect(screen.getAllByText("provider://alpha#<script>")).toHaveLength(2)
    expect(screen.getByText("unknown")).toBeInTheDocument()
    expect(screen.queryByText("route://candidate;<img src=x>")).not.toBeInTheDocument()
  })

  it("reloads provider usage when the Usage window control changes", async () => {
    // Given
    bridgeMock.loadProviderUsage.mockImplementation(async (request) => {
      if (request?.windowKey === "7d") {
        return sevenDayProviderUsageReport
      }
      return currentProviderUsageReport
    })
    window.location.hash = "#usage"
    render(<App />)
    await waitFor(() =>
      expect(bridgeMock.loadProviderUsage).toHaveBeenCalledWith({ windowKey: "30d" })
    )

    // When
    fireEvent.click(screen.getByRole("button", { name: "7 days" }))

    // Then
    await waitFor(() =>
      expect(bridgeMock.loadProviderUsage).toHaveBeenCalledWith({ windowKey: "7d" })
    )
    expect(screen.getByRole("button", { name: "7 days" })).toHaveAttribute(
      "aria-pressed",
      "true"
    )
    expect(await screen.findByText("1")).toBeInTheDocument()
  })

  it("falls back to Status for malformed hashes without loading provider usage", async () => {
    // Given
    window.location.hash = "#usage<script>"

    // When
    render(<App />)

    // Then
    expect(await screen.findByRole("heading", { name: "App shell" })).toBeInTheDocument()
    expect(screen.getByRole("link", { name: "Status" })).toHaveAttribute("aria-current", "page")
    expect(bridgeMock.loadProviderUsage).not.toHaveBeenCalled()
  })

  it("reloads provider usage predictably when hash changes back to Usage", async () => {
    // Given
    bridgeMock.setProviderUsageReport(currentProviderUsageReport)
    render(<App />)
    expect(await screen.findByRole("heading", { name: "App shell" })).toBeInTheDocument()

    // When
    openRoute("#usage")

    // Then
    await waitFor(() => expect(bridgeMock.loadProviderUsage).toHaveBeenCalledTimes(1))
    const summary = await screen.findByLabelText("Provider usage summary")
    expect(within(summary).getByText("3")).toBeInTheDocument()

    // When
    openRoute("#status")
    bridgeMock.setProviderUsageReport(emptyProviderUsageReport)
    openRoute("#usage")

    // Then
    await waitFor(() => expect(bridgeMock.loadProviderUsage).toHaveBeenCalledTimes(2))
    expect(await screen.findByText("No provider usage recorded yet.")).toBeInTheDocument()
  })

  it("renders the Usage error contract when provider usage loading fails", async () => {
    // Given
    bridgeMock.loadProviderUsage.mockRejectedValueOnce(new Error("storage unavailable"))
    window.location.hash = "#usage"

    // When
    render(<App />)

    // Then
    expect(await screen.findByText("Provider usage could not be loaded.")).toBeInTheDocument()
  })
})
