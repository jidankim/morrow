import { CalendarDays, HardDrive, ListTodo } from "lucide-react"
import { useState } from "react"
import type { PrivacySettingsPane } from "./nativePrivacyBridge"

type PrivacyOpenState =
  | { readonly status: "idle" }
  | { readonly status: "opening"; readonly pane: PrivacySettingsPane }
  | { readonly status: "opened"; readonly pane: PrivacySettingsPane }
  | { readonly status: "failed"; readonly pane: PrivacySettingsPane; readonly message: string }

type SettingsMacOSPrivacySectionProps = {
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

export function SettingsMacOSPrivacySection({
  onOpenPrivacySettings
}: SettingsMacOSPrivacySectionProps): JSX.Element {
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

  return (
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
            disabled={privacyOpenState.status === "opening" && privacyOpenState.pane === action.pane}
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
  throw new Error(`Unhandled privacy settings variant: ${String(value)}`)
}
