import type { KeyboardEvent } from "react"
import { BreakdownTable, RecentOutcomesTable, SummaryMetrics } from "./ProviderUsageDashboardTables"
import type { ProviderUsageWindowKey } from "./domain/providerUsage"
import type { ProviderUsageState } from "./useProviderUsageController"

type ProviderUsageDashboardProps = {
  readonly state: ProviderUsageState
  readonly selectedWindowKey: ProviderUsageWindowKey
  readonly onWindowChange: (windowKey: ProviderUsageWindowKey) => void
}

export function ProviderUsageDashboard({
  state,
  selectedWindowKey,
  onWindowChange
}: ProviderUsageDashboardProps): JSX.Element {
  return (
    <div className="panel provider-usage-panel">
      <div className="panel-header">
        <div>
          <p className="eyebrow">Provider activity</p>
          <h2>Usage</h2>
        </div>
        <WindowControls selectedWindowKey={selectedWindowKey} onWindowChange={onWindowChange} />
      </div>
      <ProviderUsageBody state={state} />
    </div>
  )
}

type WindowControlsProps = {
  readonly selectedWindowKey: ProviderUsageWindowKey
  readonly onWindowChange: (windowKey: ProviderUsageWindowKey) => void
}

const WINDOW_OPTIONS = [
  { key: "7d", label: "7 days" },
  { key: "30d", label: "30 days" },
  { key: "90d", label: "90 days" },
  { key: "all", label: "All time" }
] as const satisfies readonly {
  readonly key: ProviderUsageWindowKey
  readonly label: string
}[]

function WindowControls({
  selectedWindowKey,
  onWindowChange
}: WindowControlsProps): JSX.Element {
  return (
    <div className="usage-window-controls" role="group" aria-label="Time window">
      {WINDOW_OPTIONS.map((option, index) => (
        <button
          className={option.key === selectedWindowKey ? "active" : ""}
          type="button"
          aria-pressed={option.key === selectedWindowKey}
          key={option.key}
          onClick={() => onWindowChange(option.key)}
          onKeyDown={(event) => handleWindowKeyDown(event, index, onWindowChange)}
        >
          {option.label}
        </button>
      ))}
    </div>
  )
}

function handleWindowKeyDown(
  event: KeyboardEvent<HTMLButtonElement>,
  index: number,
  onWindowChange: (windowKey: ProviderUsageWindowKey) => void
): void {
  switch (event.key) {
    case "ArrowRight":
      event.preventDefault()
      onWindowChange(windowKeyAtIndex((index + 1) % WINDOW_OPTIONS.length))
      return
    case "ArrowLeft":
      event.preventDefault()
      onWindowChange(windowKeyAtIndex((index + WINDOW_OPTIONS.length - 1) % WINDOW_OPTIONS.length))
      return
    case "Home":
      event.preventDefault()
      onWindowChange(windowKeyAtIndex(0))
      return
    case "End":
      event.preventDefault()
      onWindowChange(windowKeyAtIndex(WINDOW_OPTIONS.length - 1))
      return
    default:
      return
  }
}

function windowKeyAtIndex(index: number): ProviderUsageWindowKey {
  const option = WINDOW_OPTIONS[index]
  if (option === undefined) {
    return "30d"
  }
  return option.key
}

function ProviderUsageBody({ state }: { readonly state: ProviderUsageState }): JSX.Element {
  switch (state.status) {
    case "idle":
    case "loading":
      return (
        <p aria-label="Provider usage loading" className="usage-state" role="status">
          Loading provider usage.
        </p>
      )
    case "empty":
      return (
        <div className="usage-state">
          <p className="empty-state">No provider usage recorded yet.</p>
          <p className="usage-state-detail">
            Usage appears after local provider route outcomes are recorded.
          </p>
        </div>
      )
    case "loaded":
      return <LoadedProviderUsage report={state.report} />
    case "failed":
      return (
        <p className="usage-state error" role="alert">
          {state.message}
        </p>
      )
    default:
      return assertNeverProviderUsageState(state)
  }
}

function LoadedProviderUsage({
  report
}: {
  readonly report: Extract<ProviderUsageState, { readonly status: "loaded" }>["report"]
}): JSX.Element {
  const recentOutcomes = report.recentOutcomes.slice(0, 25)
  return (
    <>
      <SummaryMetrics totals={report.totals} windowKey={report.window.key} />
      <section className="usage-section" aria-labelledby="provider-breakdown-heading">
        <div className="usage-section-header">
          <h3 id="provider-breakdown-heading">Breakdown</h3>
          <p>Grouped by provider, model, and prompt version.</p>
        </div>
        {report.providers.length === 0 ? (
          <p className="empty-state">No provider breakdown rows.</p>
        ) : (
          <BreakdownTable providers={report.providers} />
        )}
      </section>
      <section className="usage-section" aria-labelledby="recent-outcomes-heading">
        <div className="usage-section-header">
          <h3 id="recent-outcomes-heading">Recent outcomes</h3>
          <p>Showing 25 most recent outcomes.</p>
        </div>
        {recentOutcomes.length === 0 ? (
          <p className="empty-state">No recent outcomes for this window.</p>
        ) : (
          <RecentOutcomesTable recentOutcomes={recentOutcomes} />
        )}
      </section>
    </>
  )
}

function assertNeverProviderUsageState(state: never): never {
  throw new Error(`Unsupported provider usage state: ${state}`)
}
