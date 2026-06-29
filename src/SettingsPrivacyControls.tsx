import type { DeleteAllState } from "./DeleteAllOutcome"
import type { DeleteAllOptions } from "./domain/privacyControls"
import type { PrivacySettingsPane } from "./nativePrivacyBridge"
import { SettingsDeleteAllSection } from "./SettingsDeleteAllSection"
import { SettingsMacOSPrivacySection } from "./SettingsMacOSPrivacySection"
import {
  SettingsProviderCredentialSection,
  type ProviderCredentialState
} from "./SettingsProviderCredentialSection"

export type { DeleteAllState } from "./DeleteAllOutcome"
export type { ProviderCredentialState } from "./SettingsProviderCredentialSection"

type SettingsPrivacyControlsProps = {
  readonly deleteAllState: DeleteAllState
  readonly providerCredentialState: ProviderCredentialState
  readonly onDeleteAll: (options: DeleteAllOptions) => void
  readonly onCheckProviderCredential: () => Promise<void>
  readonly onOpenPrivacySettings: (pane: PrivacySettingsPane) => Promise<void>
}

export function SettingsPrivacyControls({
  deleteAllState,
  providerCredentialState,
  onCheckProviderCredential,
  onDeleteAll,
  onOpenPrivacySettings
}: SettingsPrivacyControlsProps): JSX.Element {
  return (
    <>
      <SettingsMacOSPrivacySection onOpenPrivacySettings={onOpenPrivacySettings} />
      <SettingsProviderCredentialSection
        state={providerCredentialState}
        onCheckProviderCredential={onCheckProviderCredential}
      />
      <SettingsDeleteAllSection state={deleteAllState} onDeleteAll={onDeleteAll} />
    </>
  )
}
