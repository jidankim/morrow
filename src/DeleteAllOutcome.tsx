import type { MorrowDataDeleteReceipt } from "./nativePrivacyBridge"

export type DeleteAllState =
  | { readonly status: "idle" }
  | { readonly status: "deleting" }
  | { readonly status: "succeeded"; readonly receipt: MorrowDataDeleteReceipt }
  | { readonly status: "failed"; readonly message: string }

export function DeleteAllOutcome({
  state
}: {
  readonly state: DeleteAllState
}): JSX.Element | null {
  switch (state.status) {
    case "idle":
      return null
    case "deleting":
      return (
        <div className="result-surface info" role="status">
          <h4>Deleting Morrow data</h4>
          <p>Local Morrow data is being reset.</p>
        </div>
      )
    case "succeeded":
      return <DeleteAllSuccess receipt={state.receipt} />
    case "failed":
      return (
        <div className="result-surface error" role="alert">
          <h4>Delete did not complete</h4>
          <p>{state.message}</p>
        </div>
      )
    default:
      return assertNever(state)
  }
}

function DeleteAllSuccess({
  receipt
}: {
  readonly receipt: MorrowDataDeleteReceipt
}): JSX.Element {
  return (
    <div className="result-surface success" role="status">
      <h4>Morrow data reset</h4>
      <ul>
        <li>
          {receipt.databaseDeleted
            ? "Local Morrow database was reset."
            : "Local Morrow database was already clear."}
        </li>
        <li>Approved Calendar and Reminders items were preserved.</li>
        <li>{providerCredentialText(receipt)}</li>
        <li>
          {proposedCleanupText(receipt)}
        </li>
        <li>
          {cleanupActionText(
            receipt.cleanupPlan.emptyProposalContainers,
            "Empty Morrow Proposed containers"
          )}
        </li>
      </ul>
    </div>
  )
}

function providerCredentialText(receipt: MorrowDataDeleteReceipt): string {
  if (!receipt.providerCredentialsDeleteRequested) {
    return "Morrow-owned provider credentials were left unchanged. Codex CLI login was left unchanged."
  }
  if (receipt.providerCredentialsDeleteFailed) {
    return "Morrow-owned provider credentials could not be deleted by macOS. Codex CLI login was left unchanged."
  }
  if (receipt.providerCredentialsDeleted) {
    return "Morrow-owned provider credentials were deleted. Codex CLI login was left unchanged."
  }
  return "No Morrow-owned provider credentials were found. Codex CLI login was left unchanged."
}

function proposedCleanupText(receipt: MorrowDataDeleteReceipt): string {
  switch (receipt.cleanupPlan.proposedItems) {
    case "completed":
      return `Deleted ${receipt.cleanupPlan.proposedCalendarItemsDeleted} proposed Calendar item(s) and ${receipt.cleanupPlan.proposedReminderItemsDeleted} proposed Reminder item(s).`
    case "adapterDeferred":
      return "Proposed Calendar and Reminders cleanup is deferred to the native adapters."
    case "skippedByUser":
      return "Proposed Calendar and Reminders cleanup was skipped."
    default:
      return assertNever(receipt.cleanupPlan.proposedItems)
  }
}

function cleanupActionText(
  action: MorrowDataDeleteReceipt["cleanupPlan"]["emptyProposalContainers"],
  target: string
): string {
  switch (action) {
    case "adapterDeferred":
      return `${target} cleanup is deferred to the native adapters.`
    case "completed":
      return `${target} cleanup completed.`
    case "skippedByUser":
      return `${target} cleanup was skipped.`
    default:
      return assertNever(action)
  }
}

function assertNever(value: never): never {
  throw new Error(`Unhandled delete-all outcome variant: ${String(value)}`)
}
