import type { AppConfig } from "./domain/appShell"
import type { DeleteAllOptions } from "./domain/privacyControls"
import {
  SettingsPrivacyControls,
  type DeleteAllState,
  type ProviderCredentialState
} from "./SettingsPrivacyControls"
import type { PrivacySettingsPane } from "./tauriBridge"

export type { DeleteAllState, ProviderCredentialState } from "./SettingsPrivacyControls"

type SettingsViewProps = {
  readonly config: AppConfig
  readonly deleteAllState: DeleteAllState
  readonly providerCredentialState: ProviderCredentialState
  readonly onChange: (config: AppConfig) => void
  readonly onCheckProviderCredential: () => Promise<void>
  readonly onDeleteAll: (options: DeleteAllOptions) => void
  readonly onOpenPrivacySettings: (pane: PrivacySettingsPane) => Promise<void>
}

export function SettingsView({
  config,
  deleteAllState,
  providerCredentialState,
  onChange,
  onCheckProviderCredential,
  onDeleteAll,
  onOpenPrivacySettings
}: SettingsViewProps): JSX.Element {
  return (
    <div className="panel">
      <div className="panel-header">
        <div>
          <p className="eyebrow">App-local configuration</p>
          <h2>Settings</h2>
        </div>
      </div>
      <label className="field">
        <span>Reference timezone</span>
        <select
          value={config.referenceTimezone}
          onChange={(event) => onChange({ ...config, referenceTimezone: event.currentTarget.value })}
        >
          <option value="Asia/Seoul">Asia/Seoul</option>
          <option value="America/New_York">America/New_York</option>
          <option value="Europe/London">Europe/London</option>
          <option value="UTC">UTC</option>
        </select>
      </label>
      <label className="field">
        <span>Calendar source</span>
        <select
          value={config.calendarSource}
          onChange={() => onChange({ ...config, calendarSource: "apple-calendar" })}
        >
          <option value="apple-calendar">Apple Calendar</option>
        </select>
      </label>
      <label className="check-row">
        <input
          checked={config.launchAtLogin}
          onChange={(event) => onChange({ ...config, launchAtLogin: event.currentTarget.checked })}
          type="checkbox"
        />
        <span>Open Morrow at login</span>
      </label>
      <label className="check-row">
        <input
          checked={config.sourceExcerptsEnabled}
          onChange={(event) =>
            onChange({ ...config, sourceExcerptsEnabled: event.currentTarget.checked })
          }
          type="checkbox"
        />
        <span>Show short source excerpts in future proposal notes</span>
      </label>
      <label className="check-row">
        <input
          checked={config.firstProposalGuidanceEnabled}
          onChange={(event) =>
            onChange({ ...config, firstProposalGuidanceEnabled: event.currentTarget.checked })
          }
          type="checkbox"
        />
        <span>Show first-proposal guidance</span>
      </label>
      <SettingsPrivacyControls
        deleteAllState={deleteAllState}
        providerCredentialState={providerCredentialState}
        onCheckProviderCredential={onCheckProviderCredential}
        onDeleteAll={onDeleteAll}
        onOpenPrivacySettings={onOpenPrivacySettings}
      />
    </div>
  )
}
