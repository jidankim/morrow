import { useState } from "react"
import type {
  ListIntakeDecisionRequest,
  ListIntakeItem,
  ListIntakeProposal
} from "./domain/listIntakeReview"

type ProposalDraft = {
  readonly itemName: string
  readonly quantity: string
  readonly unit: string
  readonly categoryId: string
}

type ListIntakeProposalCardProps = {
  readonly proposal: ListIntakeProposal
  readonly onApproveEdited: (request: ListIntakeDecisionRequest) => Promise<void>
  readonly onReject: (request: ListIntakeDecisionRequest) => Promise<void>
}

export function ListIntakeProposalCard({
  proposal,
  onApproveEdited,
  onReject
}: ListIntakeProposalCardProps): JSX.Element {
  const [drafts, setDrafts] = useState<readonly ProposalDraft[]>(
    proposal.items.map((item) => ({
      categoryId: item.categoryId,
      itemName: item.itemName,
      quantity: item.quantity.toString(),
      unit: item.unit ?? ""
    }))
  )
  const [editError, setEditError] = useState<string | undefined>(undefined)

  const updateDraft = (index: number, patch: ProposalDraft): void => {
    setDrafts(drafts.map((draft, draftIndex) => (draftIndex === index ? patch : draft)))
    setEditError(undefined)
  }

  const approve = (): void => {
    const parsedItems = parseDrafts(drafts, proposal)
    if (parsedItems.status === "invalid") {
      setEditError(parsedItems.message)
      return
    }
    setEditError(undefined)
    void onApproveEdited({
      decision: "approveEdited",
      proposalId: proposal.proposalId,
      items: [...parsedItems.items]
    })
  }

  return (
    <article className="proposal-summary list-intake-proposal">
      <div className="aggregate-group-heading">
        <div>
          <p className="eyebrow">Needs review</p>
          <h4>{proposal.profileName}</h4>
        </div>
        <span className="status-pill setup-needed">list_intake</span>
      </div>
      <dl className="aggregate-meta">
        <div>
          <dt>Chat</dt>
          <dd>{proposal.chatLabel}</dd>
        </div>
        <div>
          <dt>Sender</dt>
          <dd>{proposal.senderLabel}</dd>
        </div>
        <div>
          <dt>Rows</dt>
          <dd>{proposal.items.length}</dd>
        </div>
      </dl>
      {drafts.map((draft, index) => (
        <div className="proposal-edit-grid" key={`${proposal.proposalId}-${index.toString()}`}>
          <label className="field">
            <span>Item name</span>
            <input
              value={draft.itemName}
              onChange={(event) => updateDraft(index, { ...draft, itemName: event.currentTarget.value })}
            />
          </label>
          <label className="field">
            <span>Quantity</span>
            <input
              inputMode="numeric"
              value={draft.quantity}
              onChange={(event) => updateDraft(index, { ...draft, quantity: event.currentTarget.value })}
            />
          </label>
          <label className="field">
            <span>Unit</span>
            <input
              value={draft.unit}
              onChange={(event) => updateDraft(index, { ...draft, unit: event.currentTarget.value })}
            />
          </label>
          <label className="field">
            <span>Category</span>
            <select
              value={draft.categoryId}
              onChange={(event) => updateDraft(index, { ...draft, categoryId: event.currentTarget.value })}
            >
              {proposal.categories.map((category) => (
                <option key={category.categoryId} value={category.categoryId}>
                  {category.label}
                </option>
              ))}
            </select>
          </label>
        </div>
      ))}
      {editError !== undefined ? <p className="inline-status error" role="alert">{editError}</p> : null}
      <div className="list-intake-toolbar">
        <button className="button primary" onClick={approve} type="button">
          Approve list-intake proposal
        </button>
        <button
          className="button secondary"
          onClick={() => {
            setEditError(undefined)
            void onReject({ decision: "reject", proposalId: proposal.proposalId })
          }}
          type="button"
        >
          Reject list-intake proposal
        </button>
      </div>
    </article>
  )
}

type DraftParseResult =
  | { readonly status: "valid"; readonly items: readonly ListIntakeItem[] }
  | { readonly status: "invalid"; readonly message: string }

function parseDrafts(
  drafts: readonly ProposalDraft[],
  proposal: ListIntakeProposal
): DraftParseResult {
  const items: ListIntakeItem[] = []
  for (const draft of drafts) {
    const parsedItem = parseDraft(draft, proposal)
    if (parsedItem.status === "invalid") {
      return parsedItem
    }
    items.push(parsedItem.item)
  }
  return { status: "valid", items }
}

function parseDraft(draft: ProposalDraft, proposal: ListIntakeProposal):
  | { readonly status: "valid"; readonly item: ListIntakeItem }
  | { readonly status: "invalid"; readonly message: string } {
  const itemName = draft.itemName.trim()
  const unit = draft.unit.trim()
  const quantity = Number.parseInt(draft.quantity, 10)
  const category = proposal.categories.find((option) => option.categoryId === draft.categoryId)
  if (itemName.length === 0 || itemName.length > 80) {
    return { status: "invalid", message: "proposal-edit-error: Item name is required." }
  }
  if (!Number.isInteger(quantity) || quantity < 1 || quantity > 999) {
    return { status: "invalid", message: "proposal-edit-error: Quantity must be 1 through 999." }
  }
  if (unit.length > 24) {
    return { status: "invalid", message: "proposal-edit-error: Unit must be 24 characters or fewer." }
  }
  if (category === undefined) {
    return { status: "invalid", message: "proposal-edit-error: Category is not available." }
  }
  if (unit.length === 0) {
    return {
      status: "valid",
      item: { categoryId: category.categoryId, categoryLabel: category.label, itemName, quantity }
    }
  }
  return {
    status: "valid",
    item: { categoryId: category.categoryId, categoryLabel: category.label, itemName, quantity, unit }
  }
}
