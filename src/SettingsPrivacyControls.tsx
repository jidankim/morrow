import { CalendarDays, HardDrive, ListTodo } from "lucide-react"
import { useState } from "react"
import type { DeleteAllState } from "./DeleteAllOutcome"
import type { DeleteAllOptions } from "./domain/privacyControls"
import type { AppConfig } from "./domain/appConfig"
import { FullDiskAccessRecoveryGuide } from "./FullDiskAccessRecoveryGuide"
import { SettingsDeleteAllSection } from "./SettingsDeleteAllSection"
import type { PrivacySettingsPane } from "./nativePrivacyBridge"
import type { RuntimeIdentity } from "./tauriBridge"

export type { DeleteAllState } from "./DeleteAllOutcome"

type PrivacyOpenState =
  | { readonly status: "idle" }
  | { readonly status: "opening"; readonly pane: PrivacySettingsPane }
  | { readonly status: "opened"; readonly pane: PrivacySettingsPane }
  | { readonly status: "failed"; readonly pane: PrivacySettingsPane; readonly message: string }

type SettingsPrivacyControlsProps = {
  readonly config: AppConfig
  readonly deleteAllState: DeleteAllState
  readonly runtimeIdentity?: RuntimeIdentity | undefined
  readonly onChange: (config: AppConfig) => void
  readonly onDeleteAll: (options: DeleteAllOptions) => void
  readonly onOpenPrivacySettings: (pane: PrivacySettingsPane) => Promise<void>
}

const privacyActions: readonly {
  readonly pane: PrivacySettingsPane
  readonly label: string
}[] = [
  { pane: "calendar", label: "Open Calendar access" },
  { pane: "reminders", label: "Open Reminders access" }
]

export function SettingsPrivacyControls({
  config,
  deleteAllState,
  runtimeIdentity,
  onChange,
  onDeleteAll,
  onOpenPrivacySettings
}: SettingsPrivacyControlsProps): JSX.Element {
  const [privacyOpenState, setPrivacyOpenState] = useState<PrivacyOpenState>({ status: "idle" })

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

  const changeLocalDiagnosticsRetention = (value: string): void => {
    const retentionDays = Number(value)
    if (!Number.isInteger(retentionDays) || retentionDays < 1 || retentionDays > 365) {
      return
    }
    onChange({ ...config, localDiagnosticsRetentionDays: retentionDays })
  }

  return (
    <>
      <section
        className="settings-section"
        aria-labelledby="macos-access-heading"
        data-runtime-kind={runtimeIdentity?.runtimeKind}
      >
        <div>
          <p className="eyebrow">macOS access</p>
          <h3 id="macos-access-heading">Privacy permissions</h3>
        </div>
        <p className="settings-copy">
          Morrow's runtime needs Full Disk Access to read local Messages. Terminal/Codex
          access is only for terminal QA and does not grant Morrow app access.
        </p>
        {runtimeIdentity === undefined ? (
          <p className="settings-note">
            Open Full Disk Access, add or enable Morrow, restart Morrow, then retry chat
            discovery.
          </p>
        ) : null}
        <FullDiskAccessRecoveryGuide
          runtimeIdentity={runtimeIdentity}
          surface="settings"
          onOpenFullDiskAccess={() => openPrivacySettings("fullDiskAccess")}
        />
        <p className="settings-copy">
          Calendar and Reminders access are separate from Messages Full Disk Access.
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
        <PrivacyOpenMessage state={privacyOpenState} />
      </section>
      <section className="settings-section" aria-labelledby="local-diagnostics-heading">
        <div>
          <p className="eyebrow">Local diagnostics</p>
          <h3 id="local-diagnostics-heading">Local diagnostics</h3>
        </div>
        <p className="settings-copy" id="local-diagnostics-description">
          When enabled, Morrow writes diagnostics as private local files on this Mac.
          They are not uploads, and Delete All deletes these files.
        </p>
        <label className="check-row">
          <input
            aria-describedby="local-diagnostics-description"
            checked={config.localDiagnosticsEnabled}
            onChange={(event) =>
              onChange({ ...config, localDiagnosticsEnabled: event.currentTarget.checked })
            }
            type="checkbox"
          />
          <span>Write private local diagnostics files</span>
        </label>
        <label className="field">
          <span>Local diagnostics retention (days)</span>
          <input
            max={365}
            min={1}
            onChange={(event) => changeLocalDiagnosticsRetention(event.currentTarget.value)}
            step={1}
            type="number"
            value={config.localDiagnosticsRetentionDays}
          />
        </label>
        <p className="settings-note">Keep diagnostics for 1 to 365 days. Default is 30 days.</p>
      </section>
      <SettingsDeleteAllSection state={deleteAllState} onDeleteAll={onDeleteAll} />
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
