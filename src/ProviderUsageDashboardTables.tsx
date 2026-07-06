import type {
  ProviderUsageModelGroup,
  ProviderUsagePromptGroup,
  ProviderUsageProviderGroup,
  ProviderUsageRecentOutcome,
  ProviderUsageTotals,
  ProviderUsageWindowKey
} from "./domain/providerUsage"
import { publicProviderUsageRouteLabel } from "./domain/providerUsage"
import { formatCount, formatOptionalPercent, formatRate, windowLabel } from "./providerUsageDashboardFormat"

export function SummaryMetrics({
  totals,
  windowKey
}: {
  readonly totals: ProviderUsageTotals
  readonly windowKey: ProviderUsageWindowKey
}): JSX.Element {
  return (
    <dl className="usage-summary" aria-label="Provider usage summary">
      <div>
        <dt>Total outcomes</dt>
        <dd>{formatCount(totals.totalOutcomes)}</dd>
      </div>
      <div>
        <dt>Candidates</dt>
        <dd>
          {formatCount(totals.candidateCount)} / {formatRate(totals.candidateRate)}
        </dd>
      </div>
      <div>
        <dt>Quiet</dt>
        <dd>
          {formatCount(totals.quietCount)} / {formatRate(totals.quietRate)}
        </dd>
      </div>
      <div>
        <dt>Window</dt>
        <dd>{windowLabel(windowKey)}</dd>
      </div>
    </dl>
  )
}

export function BreakdownTable({
  providers
}: {
  readonly providers: readonly ProviderUsageProviderGroup[]
}): JSX.Element {
  return (
    <div className="usage-table-wrap">
      <table className="usage-table" aria-label="Provider breakdown">
        <thead>
          <tr>
            <th scope="col">Scope</th>
            <th scope="col">Total</th>
            <th scope="col">Candidate</th>
            <th scope="col">Quiet</th>
            <th scope="col">Avg confidence</th>
          </tr>
        </thead>
        <tbody>
          {providers.flatMap((provider) => breakdownRows(provider))}
        </tbody>
      </table>
    </div>
  )
}

export function RecentOutcomesTable({
  recentOutcomes
}: {
  readonly recentOutcomes: readonly ProviderUsageRecentOutcome[]
}): JSX.Element {
  return (
    <div className="usage-table-wrap">
      <table className="usage-table" aria-label="Recent outcomes">
        <thead>
          <tr>
            <th scope="col">Provider</th>
            <th scope="col">Model</th>
            <th scope="col">Prompt</th>
            <th scope="col">Outcome</th>
            <th scope="col">Route</th>
            <th scope="col">Confidence</th>
            <th scope="col">Created</th>
          </tr>
        </thead>
        <tbody>
          {recentOutcomes.map((outcome, index) => (
            <tr key={`${outcome.createdAtUnixSeconds}-${index}`}>
              <th scope="row">{outcome.providerId}</th>
              <td>{outcome.modelId}</td>
              <td>{outcome.promptVersion}</td>
              <td>{outcome.outcomeKind}</td>
              <td>{publicProviderUsageRouteLabel(outcome.routeLabel)}</td>
              <td>{formatOptionalPercent(outcome.confidence)}</td>
              <td>Unix {formatCount(outcome.createdAtUnixSeconds)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}

function breakdownRows(provider: ProviderUsageProviderGroup): readonly JSX.Element[] {
  return [
    <BreakdownRow
      group={provider}
      key={`provider-${provider.providerId}`}
      label={provider.providerId}
      scope="Provider"
    />,
    ...provider.models.flatMap((model) => modelRows(provider.providerId, model))
  ]
}

function modelRows(
  providerId: string,
  model: ProviderUsageModelGroup
): readonly JSX.Element[] {
  return [
    <BreakdownRow
      group={model}
      key={`model-${providerId}-${model.modelId}`}
      label={model.modelId}
      scope="Model"
    />,
    ...model.promptVersions.map((prompt) => (
      <BreakdownRow
        group={prompt}
        key={`prompt-${providerId}-${model.modelId}-${prompt.promptVersion}`}
        label={prompt.promptVersion}
        scope="Prompt"
      />
    ))
  ]
}

function BreakdownRow({
  group,
  label,
  scope
}: {
  readonly group: ProviderUsageProviderGroup | ProviderUsageModelGroup | ProviderUsagePromptGroup
  readonly label: string
  readonly scope: "Provider" | "Model" | "Prompt"
}): JSX.Element {
  return (
    <tr className={`usage-breakdown-row ${scope.toLowerCase()}`}>
      <th scope="row">
        <span className="usage-scope">{scope}</span>
        <span className="usage-label">{label}</span>
      </th>
      <td>{formatCount(group.totalOutcomes)}</td>
      <td>{formatCount(group.candidateCount)} / {formatRate(group.candidateRate)}</td>
      <td>{formatCount(group.quietCount)} / {formatRate(group.quietRate)}</td>
      <td>{formatOptionalPercent(group.averageConfidence)}</td>
    </tr>
  )
}
