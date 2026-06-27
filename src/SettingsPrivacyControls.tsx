import { CalendarDays, HardDrive, ListTodo, Trash2 } from "lucide-react"
import { useState } from "react"
import { DeleteAllOutcome, type DeleteAllState } from "./DeleteAllOutcome"
import {
  DELETE_ALL_CONFIRMATION_TEXT,
  createDefaultDeleteAllOptions,
  parseDeleteAllConfirmation,
  type DeleteAllOptions
} from "./domain/privacyControls"
import type { PrivacySettingsPane } from "./nativePrivacyBridge"

export type { DeleteAllState } from "./DeleteAllOutcome"

type PrivacyOpenState =
  | { readonly status: "idle" }
  | { readonly status: "opening"; readonly pane: PrivacySettingsPane }
  | { readonly status: "opened"; readonly pane: PrivacySettingsPane }
  | { readonly status: "failed"; readonly pane: PrivacySettingsPane; readonly message: string }

type SettingsPrivacyControlsProps = {
  readonly deleteAllState: DeleteAllState
  readonly onDeleteAll: (options: DeleteAllOptions) => void
  readonly onOpenPrivacySettings: (pane: PrivacySettingsPane) => Promise<void>
}

const privacyActions: readonly {
  readonly pane: PrivacySettingsPane
  readonly label: string
}[] = [
  { pane: "fullDiskAccess", label: "Open Full Disk Access" },
  { pane: "calendar", label: "Open Calendar access" },
  { pane: "reminders", label: "Open Reminders access" }
]

export function SettingsPrivacyControls({
  deleteAllState,
  onDeleteAll,
  onOpenPrivacySettings
}: SettingsPrivacyControlsProps): JSX.Element {
  const [confirmation, setConfirmation] = useState("")
  const [deleteOptions, setDeleteOptions] = useState(createDefaultDeleteAllOptions)
  const [privacyOpenState, setPrivacyOpenState] = useState<PrivacyOpenState>({ status: "idle" })
  const canDelete = parseDeleteAllConfirmation(confirmation).ok
  const deleteInFlight = deleteAllState.status === "deleting"

  const submitDeleteAll = (): void => {
    if (!canDelete || deleteInFlight) {
      return
    }
    onDeleteAll(deleteOptions)
    setConfirmation("")
    setDeleteOptions(createDefaultDeleteAllOptions())
  }

  const openPrivacySettings = async (pane: PrivacySettingsPane): Promise<void> => {
    setPrivacyOpenState({ status: "opening", pane })
    try {
      await onOpenPrivacySettings(pane)
      setPrivacyOpenState({ status: "opened", pane })
    } catch (error: unknown) {
      const message =
        error instanceof Error ? error.message : "macOS Settings could not be opened."
      setPrivacyOpenState({ status: "failed", pane, message })
    }
  }

  return (
    <>
      <section className="settings-section" aria-labelledby="macos-access-heading">
        <div>
          <p className="eyebrow">macOS access</p>
          <h3 id="macos-access-heading">Privacy permissions</h3>
        </div>
        <p className="settings-copy">
          Grant Morrow Full Disk Access for Messages. Calendar and Reminders access are separate.
        </p>
        <div className="permission-action-row">
          {privacyActions.map((action) => (
            <button
              className="button secondary"
              disabled={
                privacyOpenState.status === "opening" && privacyOpenState.pane === action.pane
              }
              key={action.pane}
              onClick={() => void openPrivacySettings(action.pane)}
              type="button"
            >
              {privacyIcon(action.pane)}
              {action.label}
            </button>
          ))}
        </div>
        <p className="settings-note">
          For QA from Terminal, add Terminal to Full Disk Access too.
        </p>
        <PrivacyOpenMessage state={privacyOpenState} />
      </section>
      <section className="danger-section" aria-labelledby="delete-all-heading">
        <div>
          <p className="eyebrow">Privacy reset</p>
          <h3 id="delete-all-heading">Delete all Morrow data</h3>
        </div>
        <DeleteOptionControls options={deleteOptions} onChange={setDeleteOptions} />
        <label className="field">
          <span>Type {DELETE_ALL_CONFIRMATION_TEXT} to confirm</span>
          <input
            autoComplete="off"
            value={confirmation}
            onChange={(event) => setConfirmation(event.currentTarget.value)}
          />
        </label>
        <DeleteAllOutcome state={deleteAllState} />
        <button
          className="button danger"
          disabled={!canDelete || deleteInFlight}
          onClick={submitDeleteAll}
          type="button"
        >
          <Trash2 aria-hidden="true" size={16} />
          {deleteInFlight ? "Deleting Morrow data" : "Delete Morrow data"}
        </button>
      </section>
    </>
  )
}

function DeleteOptionControls({
  options,
  onChange
}: {
  readonly options: DeleteAllOptions
  readonly onChange: (options: DeleteAllOptions) => void
}): JSX.Element {
  return (
    <>
      <label className="check-row">
        <input
          checked={options.cleanupProposedItems}
          onChange={(event) =>
            onChange({ ...options, cleanupProposedItems: event.currentTarget.checked })
          }
          type="checkbox"
        />
        <span>Delete proposed Morrow items</span>
      </label>
      <label className="check-row">
        <input
          checked={options.deleteEmptyProposalContainers}
          onChange={(event) =>
            onChange({ ...options, deleteEmptyProposalContainers: event.currentTarget.checked })
          }
          type="checkbox"
        />
        <span>Delete empty Morrow Proposed containers</span>
      </label>
      <label className="check-row">
        <input
          checked={options.revokeProviderOAuth}
          onChange={(event) =>
            onChange({ ...options, revokeProviderOAuth: event.currentTarget.checked })
          }
          type="checkbox"
        />
        <span>Revoke Morrow OAuth grant</span>
      </label>
    </>
  )
}

function PrivacyOpenMessage({ state }: { readonly state: PrivacyOpenState }): JSX.Element | null {
  switch (state.status) {
    case "idle":
      return null
    case "opening":
      return (
        <p className="inline-status" role="status">
          Opening {privacyPaneName(state.pane)}...
        </p>
      )
    case "opened":
      return (
        <p className="inline-status success" role="status">
          {privacyPaneName(state.pane)} opened.
        </p>
      )
    case "failed":
      return (
        <p className="inline-status error" role="alert">
          {state.message}
        </p>
      )
    default:
      return assertNever(state)
  }
}

function privacyIcon(pane: PrivacySettingsPane): JSX.Element {
  switch (pane) {
    case "fullDiskAccess":
      return <HardDrive aria-hidden="true" size={16} />
    case "calendar":
      return <CalendarDays aria-hidden="true" size={16} />
    case "reminders":
      return <ListTodo aria-hidden="true" size={16} />
    default:
      return assertNever(pane)
  }
}

function privacyPaneName(pane: PrivacySettingsPane): string {
  switch (pane) {
    case "fullDiskAccess":
      return "Full Disk Access"
    case "calendar":
      return "Calendar access"
    case "reminders":
      return "Reminders access"
    default:
      return assertNever(pane)
  }
}

function assertNever(value: never): never {
  throw new Error(`Unhandled settings variant: ${String(value)}`)
}
