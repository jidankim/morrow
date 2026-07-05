import type { DecisionEvidenceItem, DecisionTraceStep } from "./domain/decisionEvidence"
import { formatLatestEvalStatus } from "./domain/syncResultCounts"

type StatusDecisionEvidenceProps = {
  readonly items: readonly DecisionEvidenceItem[]
  readonly latestEvalStatus: Parameters<typeof formatLatestEvalStatus>[0]
  readonly loading: boolean
}

export function StatusDecisionEvidence({
  items,
  latestEvalStatus,
  loading
}: StatusDecisionEvidenceProps): JSX.Element {
  return (
    <section className="decision-evidence-section" aria-labelledby="decision-evidence-heading">
      <div className="decision-evidence-header">
        <h3 id="decision-evidence-heading">Decision evidence</h3>
        {loading ? (
          <p
            aria-label="Decision evidence loading"
            className="decision-evidence-loading"
            role="status"
          >
            Loading decision evidence.
          </p>
        ) : null}
      </div>
      {items.length === 0 ? (
        <p className="empty-state">No recent decision evidence.</p>
      ) : (
        <ul className="decision-evidence-list" aria-label="Recent decision evidence">
          {items.map((item, index) => (
            <DecisionEvidenceRow
              item={item}
              key={`${item.subjectType}-${item.createdAt}-${index}`}
              latestEvalStatus={latestEvalStatus}
            />
          ))}
        </ul>
      )}
    </section>
  )
}

type DecisionEvidenceRowProps = {
  readonly item: DecisionEvidenceItem
  readonly latestEvalStatus: Parameters<typeof formatLatestEvalStatus>[0]
}

function DecisionEvidenceRow({
  item,
  latestEvalStatus
}: DecisionEvidenceRowProps): JSX.Element {
  return (
    <li className={`decision-evidence-item ${item.subjectType}`}>
      <div className="decision-evidence-main">
        <span className="decision-evidence-subject">{formatDecisionSubject(item.subjectType)}</span>
        <span className="decision-evidence-route">{formatRouteReason(item)}</span>
        <span className={`decision-evidence-trace ${traceRetentionClass(item.traceRetention)}`}>
          {formatTraceRetention(item.traceRetention)}
        </span>
      </div>
      <div className="decision-evidence-meta">
        {item.confidenceMillis === undefined ? null : <span>Confidence {item.confidenceMillis} ms</span>}
        {item.candidateId === undefined ? null : (
          <span className="decision-evidence-candidate-id" aria-label={`Candidate ID ${item.candidateId}`}>
            <span>ID</span>
            <code>{item.candidateId}</code>
          </span>
        )}
        <span>
          {item.labelType}: {item.labelValue} · Eval {formatLatestEvalStatus(latestEvalStatus).toLowerCase()}
        </span>
      </div>
      {item.providerDiagnostic === undefined ? null : (
        <p className="decision-evidence-provider-diagnostic">{item.providerDiagnostic}</p>
      )}
      {item.sourceExcerpt === undefined ? null : (
        <p className="decision-evidence-source">
          <span className="decision-evidence-source-label">Message</span>
          <span className="decision-evidence-source-text">{item.sourceExcerpt}</span>
        </p>
      )}
      <p className="decision-evidence-sequence">{formatDecisionSequence(item.traceSequence)}</p>
    </li>
  )
}

function formatDecisionSubject(subjectType: DecisionEvidenceItem["subjectType"]): string {
  switch (subjectType) {
    case "candidate":
      return "Candidate"
    case "quietLog":
      return "Quiet"
    default:
      return assertNever(subjectType)
  }
}

function formatRouteReason(item: DecisionEvidenceItem): string {
  const route = item.route ?? "Route unavailable"
  if (item.reasonCode === undefined) {
    return route
  }
  return `${route} / ${item.reasonCode}`
}

function formatTraceRetention(traceRetention: DecisionEvidenceItem["traceRetention"]): string {
  switch (traceRetention) {
    case "retained":
      return "Trace retained"
    case "notRetained":
    case "diagnosticsMissing":
    case "traceMissing":
      return "Trace not retained"
    default:
      return assertNever(traceRetention)
  }
}

function traceRetentionClass(traceRetention: DecisionEvidenceItem["traceRetention"]): string {
  switch (traceRetention) {
    case "retained":
      return "retained"
    case "notRetained":
    case "diagnosticsMissing":
    case "traceMissing":
      return "not-retained"
    default:
      return assertNever(traceRetention)
  }
}

function formatDecisionSequence(traceSequence: readonly DecisionTraceStep[]): string {
  if (traceSequence.length === 0) {
    return "Sequence unavailable."
  }
  return traceSequence.map(formatDecisionStep).join(" -> ")
}

function formatDecisionStep(step: DecisionTraceStep): string {
  return [step.component, step.operation, step.decision, step.outcome]
    .filter((value) => value !== undefined)
    .join(" ")
}

function assertNever(value: never): never {
  throw new Error(`Unhandled decision evidence variant: ${String(value)}`)
}
