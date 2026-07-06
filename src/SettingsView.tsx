import type { AppConfig } from "./domain/appShell"
import type { DeleteAllOptions } from "./domain/privacyControls"
import type { SyncSchedulerIntervalSeconds, SyncSchedulerState } from "./domain/syncScheduler"
import { referenceTimeZoneOptions } from "./domain/timeZone"
import {
  SettingsPrivacyControls,
  type DeleteAllState
} from "./SettingsPrivacyControls"
import {
  SettingsProviderCredentialSection,
  type ProviderCredentialState
} from "./SettingsProviderCredentialSection"
import { SyncSchedulerControls } from "./SyncSchedulerControls"
import type { PrivacySettingsPane, RuntimeIdentity } from "./tauriBridge"

export type { DeleteAllState } from "./SettingsPrivacyControls"
export type { ProviderCredentialState } from "./SettingsProviderCredentialSection"

type SettingsViewProps = {
  readonly config: AppConfig
  readonly deleteAllState: DeleteAllState
  readonly providerCredentialState: ProviderCredentialState
  readonly runtimeIdentity?: RuntimeIdentity | undefined
  readonly syncScheduler: SyncSchedulerState
  readonly syncSchedulerNowUnixSeconds: number
  readonly onChangeAutomaticSyncInterval: (intervalSeconds: SyncSchedulerIntervalSeconds) => void
  readonly onChange: (config: AppConfig) => void
  readonly onCheckProviderCredential: () => Promise<void>
  readonly onDeleteAll: (options: DeleteAllOptions) => void
  readonly onOpenPrivacySettings: (pane: PrivacySettingsPane) => Promise<void>
  readonly onToggleAutomaticSync: () => void
}

export function SettingsView({
  config,
  deleteAllState,
  providerCredentialState,
  runtimeIdentity,
  syncScheduler,
  syncSchedulerNowUnixSeconds,
  onChangeAutomaticSyncInterval,
  onChange,
  onCheckProviderCredential,
  onDeleteAll,
  onOpenPrivacySettings,
  onToggleAutomaticSync
}: SettingsViewProps): JSX.Element {
  const timeZoneOptions = referenceTimeZoneOptions(Intl.DateTimeFormat().resolvedOptions().timeZone)
  const listReminderDescription = "list-reminder-profile-description"
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
          {timeZoneOptions.map((timeZoneOption) => (
            <option key={timeZoneOption.value} value={timeZoneOption.value}>
              {timeZoneOption.label}
            </option>
          ))}
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
      <section className="settings-section" aria-labelledby="list-reminder-profile-heading">
        <div>
          <p className="eyebrow">Reminder profile</p>
          <h3 id="list-reminder-profile-heading">Daily list reminders</h3>
        </div>
        <p className="settings-copy" id={listReminderDescription}>
          Treat bare quantity lists as one local reminder due the next day at 23:59.
        </p>
        <label className="check-row">
          <input
            aria-describedby={listReminderDescription}
            checked={config.listReminderProfile.enabled}
            onChange={(event) =>
              onChange({
                ...config,
                listReminderProfile: event.currentTarget.checked
                  ? {
                      ...config.listReminderProfile,
                      enabled: true,
                      routingMode: "profileBareQuantityLists",
                      defaultDueMode: "nextLocalDayAtDefaultTime",
                      defaultDueTime: "23:59",
                      recurrenceMode: "none",
                      itemOutputMode: "singleReminderTitle"
                    }
                  : {
                      ...config.listReminderProfile,
                      enabled: false,
                      routingMode: "explicitOnly",
                      defaultDueMode: "explicitOnly",
                      defaultDueTime: "23:59",
                      recurrenceMode: "none",
                      itemOutputMode: "singleReminderTitle"
                    }
              })
            }
            type="checkbox"
          />
          <span>Enable daily list reminders</span>
        </label>
      </section>
      <SyncSchedulerControls
        nowUnixSeconds={syncSchedulerNowUnixSeconds}
        scheduler={syncScheduler}
        surface="settings"
        onIntervalChange={onChangeAutomaticSyncInterval}
        onToggle={onToggleAutomaticSync}
      />
      <SettingsProviderCredentialSection
        state={providerCredentialState}
        onCheckProviderCredential={onCheckProviderCredential}
      />
      <SettingsPrivacyControls
        config={config}
        deleteAllState={deleteAllState}
        runtimeIdentity={runtimeIdentity}
        onChange={onChange}
        onDeleteAll={onDeleteAll}
        onOpenPrivacySettings={onOpenPrivacySettings}
      />
    </div>
  )
}
