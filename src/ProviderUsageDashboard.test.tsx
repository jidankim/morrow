import { fireEvent, render, screen, within } from "@testing-library/react"
import { describe, expect, it, vi } from "vitest"
import { ProviderUsageDashboard } from "./ProviderUsageDashboard"
import type { ProviderUsageReport, ProviderUsageWindowKey } from "./domain/providerUsage"
import type { ProviderUsageState } from "./useProviderUsageController"

const reportWithRows = (
  recentOutcomes: ProviderUsageReport["recentOutcomes"] = recentRows(3)
) =>
  ({
    generatedAtUnixSeconds: 1_783_000_600,
    window: {
      key: "30d",
      startUnixSeconds: 1_780_408_600,
      endUnixSeconds: 1_783_000_600
    },
    totals: {
      totalOutcomes: 12,
      candidateCount: 7,
      quietCount: 5,
      quietRate: 5 / 12,
      candidateRate: 7 / 12
    },
    providers: [
      {
        providerId: "provider-alpha",
        totalOutcomes: 12,
        candidateCount: 7,
        quietCount: 5,
        quietRate: 5 / 12,
        candidateRate: 7 / 12,
        averageConfidence: 0.823,
        models: [
          {
            modelId: "model-primary",
            totalOutcomes: 9,
            candidateCount: 6,
            quietCount: 3,
            quietRate: 1 / 3,
            candidateRate: 2 / 3,
            averageConfidence: 0.91,
            promptVersions: [
              {
                promptVersion: "route-v3",
                totalOutcomes: 9,
                candidateCount: 6,
                quietCount: 3,
                quietRate: 1 / 3,
                candidateRate: 2 / 3,
                averageConfidence: 0.905
              }
            ]
          }
        ]
      }
    ],
    recentOutcomes
  }) satisfies ProviderUsageReport

const hostileReport = {
  ...reportWithRows(),
  providers: [
    {
      providerId: "provider <script>alert(1)</script>",
      totalOutcomes: 1,
      candidateCount: 1,
      quietCount: 0,
      quietRate: -1,
      candidateRate: 8,
      averageConfidence: 2,
      models: [
        {
          modelId: "model </button><img src=x onerror=alert(1)>",
          totalOutcomes: 1,
          candidateCount: 1,
          quietCount: 0,
          quietRate: 0,
          candidateRate: 1,
          averageConfidence: -1,
          promptVersions: [
            {
              promptVersion: "prompt </td><script>alert(2)</script>",
              totalOutcomes: 1,
              candidateCount: 1,
              quietCount: 0,
              quietRate: 0,
              candidateRate: 1,
              averageConfidence: undefined
            }
          ]
        }
      ]
    }
  ],
  recentOutcomes: [
    {
      providerId: "provider <script>alert(1)</script>",
      modelId: "model </button><img src=x onerror=alert(1)>",
      promptVersion: "prompt </td><script>alert(2)</script>",
      outcomeKind: "candidate",
      routeLabel: "route <svg onload=alert(3)>",
      confidence: 2,
      createdAtUnixSeconds: 1_783_000_500
    }
  ]
} satisfies ProviderUsageReport

const privateFieldNames = [
  ["quiet", "_reason"].join(""),
  ["provider", "_json"].join(""),
  ["candidate", "_title"].join("")
] as const

function recentRows(count: number): ProviderUsageReport["recentOutcomes"] {
  return Array.from({ length: count }, (_, index) => ({
    providerId: `provider-${index}`,
    modelId: `model-${index}`,
    promptVersion: `route-v${index}`,
    outcomeKind: index % 2 === 0 ? "candidate" : "quiet",
    routeLabel: index % 2 === 0 ? "candidate_route" : "quiet_route",
    confidence: 0.5,
    createdAtUnixSeconds: 1_783_000_500 - index
  }))
}

function renderDashboard(
  state: ProviderUsageState,
  selectedWindowKey: ProviderUsageWindowKey = "30d"
) {
  const onWindowChange = vi.fn()
  const view = render(
    <ProviderUsageDashboard
      selectedWindowKey={selectedWindowKey}
      state={state}
      onWindowChange={onWindowChange}
    />
  )
  return { ...view, onWindowChange }
}

describe("ProviderUsageDashboard", () => {
  it("renders loaded summaries when a usage report is available", () => {
    // Given
    const state = { status: "loaded", report: reportWithRows() } satisfies ProviderUsageState

    // When
    renderDashboard(state)

    // Then
    const summary = screen.getByLabelText("Provider usage summary")
    expect(screen.getByRole("heading", { name: "Usage" })).toBeInTheDocument()
    expect(within(summary).getByText("12")).toBeInTheDocument()
    expect(within(summary).getByText("7 / 58.3%")).toBeInTheDocument()
    expect(within(summary).getByText("5 / 41.7%")).toBeInTheDocument()
    expect(within(summary).getByText("30 days")).toBeInTheDocument()
  })

  it("calls the selected-window handler when a window control is activated", async () => {
    // Given
    const state = { status: "loaded", report: reportWithRows() } satisfies ProviderUsageState
    const { onWindowChange } = renderDashboard(state)

    // When
    fireEvent.click(screen.getByRole("button", { name: "7 days" }))

    // Then
    expect(onWindowChange).toHaveBeenCalledWith("7d")
    expect(screen.getByRole("button", { name: "30 days" })).toHaveAttribute(
      "aria-pressed",
      "true"
    )
  })

  it("renders provider model and prompt breakdown rows", () => {
    // Given
    const state = { status: "loaded", report: reportWithRows() } satisfies ProviderUsageState

    // When
    renderDashboard(state)

    // Then
    const providerRow = screen.getByRole("row", { name: /provider-alpha/i })
    expect(within(providerRow).getByText("82.3%")).toBeInTheDocument()
    expect(screen.getByRole("row", { name: /model-primary/i })).toBeInTheDocument()
    expect(screen.getByRole("row", { name: /route-v3/i })).toBeInTheDocument()
  })

  it("caps recent outcomes at 25 rows", () => {
    // Given
    const state = {
      status: "loaded",
      report: reportWithRows(recentRows(30))
    } satisfies ProviderUsageState

    // When
    renderDashboard(state)

    // Then
    const recentTable = screen.getByRole("table", { name: "Recent outcomes" })
    expect(within(recentTable).getAllByRole("row", { name: /provider-/i })).toHaveLength(25)
    expect(screen.getByText("Showing 25 most recent outcomes.")).toBeInTheDocument()
  })

  it("renders empty loading and sanitized error states", () => {
    // Given
    const loadingState = { status: "loading" } satisfies ProviderUsageState

    // When
    const { rerender } = renderDashboard(loadingState)

    // Then
    expect(screen.getByRole("status", { name: "Provider usage loading" })).toHaveTextContent(
      "Loading provider usage."
    )

    // When
    rerender(
      <ProviderUsageDashboard
        selectedWindowKey="30d"
        state={{ status: "empty" }}
        onWindowChange={vi.fn()}
      />
    )

    // Then
    expect(screen.getByText("No provider usage recorded yet.")).toBeInTheDocument()

    // When
    rerender(
      <ProviderUsageDashboard
        selectedWindowKey="30d"
        state={{ status: "failed", message: "Provider usage could not be loaded." }}
        onWindowChange={vi.fn()}
      />
    )

    // Then
    expect(screen.getByRole("alert")).toHaveTextContent("Provider usage could not be loaded.")
  })

  it("supports keyboard-accessible time-window controls", async () => {
    // Given
    const state = { status: "loaded", report: reportWithRows() } satisfies ProviderUsageState
    const { onWindowChange } = renderDashboard(state)
    const selectedWindow = screen.getByRole("button", { name: "30 days" })

    // When
    selectedWindow.focus()
    fireEvent.keyDown(selectedWindow, { key: "ArrowRight" })

    // Then
    expect(onWindowChange).toHaveBeenCalledWith("90d")
  })

  it("renders hostile labels as inert text and keeps private strings out of the DOM", () => {
    // Given
    const state = { status: "loaded", report: hostileReport } satisfies ProviderUsageState

    // When
    const { container } = renderDashboard(state)

    // Then
    expect(screen.getAllByText("provider <script>alert(1)</script>")).toHaveLength(2)
    expect(container.querySelector("script")).toBeNull()
    expect(container.querySelector("img")).toBeNull()
    for (const privateFieldName of privateFieldNames) {
      expect(container.textContent).not.toContain(privateFieldName)
    }
    expect(screen.getAllByText("100.0%").length).toBeGreaterThan(0)
  })

  it("does not render raw parser time route metadata", () => {
    // Given
    const parserTimeReport = reportWithRows([
      {
        providerId: "provider-parser",
        modelId: "model-parser",
        promptVersion: "prompt-parser",
        outcomeKind: "candidate",
        routeLabel: "parser_time:2026-07-01T09:30:00[Asia/Seoul]",
        confidence: 0.5,
        createdAtUnixSeconds: 1_783_000_500
      }
    ])

    // When
    const { container } = renderDashboard({ status: "loaded", report: parserTimeReport })

    // Then
    expect(container.textContent).toContain("parser_time")
    expect(container.textContent).not.toContain("parser_time:2026-07-01T09:30:00")
    expect(container.textContent).not.toContain("2026-07-01T09:30:00")
    expect(container.textContent).not.toContain("Asia/Seoul")
  })
})
