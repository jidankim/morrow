import type { ListIntakeDecisionRequest, ListIntakeReviewReport } from "./domain/listIntakeReview"
import { ListIntakeProposalCard } from "./ListIntakeProposalCard"
import type { ListIntakeReviewState } from "./useListIntakeReviewController"

type ListIntakeReviewViewProps = {
  readonly state: ListIntakeReviewState
  readonly onApproveEdited: (request: ListIntakeDecisionRequest) => Promise<void>
  readonly onReject: (request: ListIntakeDecisionRequest) => Promise<void>
  readonly onReload: () => Promise<void>
}

export function ListIntakeReviewView({
  state,
  onApproveEdited,
  onReject,
  onReload
}: ListIntakeReviewViewProps): JSX.Element {
  return (
    <div className="panel list-intake-review-panel">
      <div className="panel-header">
        <div>
          <p className="eyebrow">Captured rows</p>
          <h2>List intake</h2>
        </div>
        <button className="button secondary" onClick={() => void onReload()} type="button">
          Refresh
        </button>
      </div>
      <ListIntakeReviewBody
        state={state}
        onApproveEdited={onApproveEdited}
        onReject={onReject}
      />
    </div>
  )
}

function ListIntakeReviewBody({
  state,
  onApproveEdited,
  onReject
}: Omit<ListIntakeReviewViewProps, "onReload">): JSX.Element {
  switch (state.status) {
    case "idle":
    case "loading":
      return <p className="usage-state" role="status">Loading list intake review.</p>
    case "empty":
      return (
        <div className="usage-state">
          <p className="empty-state">No list-intake rows or proposals yet.</p>
          <p className="usage-state-detail">Rows appear after enabled profiles capture quantity lists.</p>
        </div>
      )
    case "loaded":
      return (
        <LoadedListIntakeReview
          report={state.report}
          notice={state.notice}
          onApproveEdited={onApproveEdited}
          onReject={onReject}
        />
      )
    case "failed":
      return <p className="usage-state error" role="alert">{state.message}</p>
    default:
      return assertNeverListIntakeReviewState(state)
  }
}

function LoadedListIntakeReview({
  report,
  notice,
  onApproveEdited,
  onReject
}: {
  readonly report: ListIntakeReviewReport
  readonly notice?: string | undefined
  readonly onApproveEdited: (request: ListIntakeDecisionRequest) => Promise<void>
  readonly onReject: (request: ListIntakeDecisionRequest) => Promise<void>
}): JSX.Element {
  return (
    <>
      {notice !== undefined ? <p className="inline-status" role="status">{notice}</p> : null}
      <section className="usage-section" aria-labelledby="recent-aggregates-heading">
        <div className="usage-section-header">
          <h3 id="recent-aggregates-heading">Recent captured aggregates</h3>
          <p>Grouped by profile, local day, chat, sender label, and category.</p>
        </div>
        {report.aggregates.length === 0 ? (
          <p className="empty-state">No aggregate rows yet.</p>
        ) : (
          <div className="list-intake-review-stack">
            {report.aggregates.map((aggregate, index) => (
              <article className="aggregate-group" key={`${aggregate.localDate}-${index.toString()}`}>
                <div className="aggregate-group-heading">
                  <div>
                    <p className="eyebrow">{aggregate.profileName}</p>
                    <h4>{aggregate.localDate}</h4>
                  </div>
                  <span className="status-pill scanning">{aggregate.outputPolicy}</span>
                </div>
                <dl className="aggregate-meta">
                  <div>
                    <dt>Chat</dt>
                    <dd>{aggregate.chatLabel}</dd>
                  </div>
                  <div>
                    <dt>Sender</dt>
                    <dd>{aggregate.senderLabel}</dd>
                  </div>
                  <div>
                    <dt>Category</dt>
                    <dd>{aggregate.categoryLabel}</dd>
                  </div>
                </dl>
                <ul className="aggregate-items" aria-label="Aggregate item rows">
                  {aggregate.items.map((item) => (
                    <li key={`${item.itemName}-${item.categoryId}`}>
                      {item.quantity} {item.itemName}{item.unit === undefined ? "" : ` ${item.unit}`}
                    </li>
                  ))}
                </ul>
              </article>
            ))}
          </div>
        )}
      </section>
      <section className="usage-section" aria-labelledby="proposal-review-heading">
        <div className="usage-section-header">
          <h3 id="proposal-review-heading">Proposed items queue</h3>
          <p>Reviewable list_intake rows stay editable before approval.</p>
        </div>
        {report.proposals.length === 0 ? (
          <p className="empty-state">No list-intake proposals need review.</p>
        ) : (
          <div className="proposal-queue">
            {report.proposals.map((proposal) => (
              <ListIntakeProposalCard
                key={proposal.proposalId}
                proposal={proposal}
                onApproveEdited={onApproveEdited}
                onReject={onReject}
              />
            ))}
          </div>
        )}
      </section>
    </>
  )
}

function assertNeverListIntakeReviewState(state: never): never {
  throw new Error(`Unsupported list intake review state: ${state}`)
}
