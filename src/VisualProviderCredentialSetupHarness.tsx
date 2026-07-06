import { SettingsView, type ProviderCredentialState } from "./SettingsView"
import { createDefaultAppShellState } from "./domain/appShell"
import { createDefaultSyncSchedulerState, type SyncSchedulerIntervalSeconds } from "./domain/syncScheduler"
import type { AppConfig } from "./domain/appShell"
import type { DeleteAllOptions } from "./domain/privacyControls"
import type { PrivacySettingsPane } from "./tauriBridge"

export const visualProviderCredentialSetupStates = {
  "missing-cli": true,
  "install-confirming": true,
  installing: true,
  "not-logged-in": true,
  "login-polling": true,
  ready: true,
  failed: true
} as const

export type VisualProviderCredentialSetupState = keyof typeof visualProviderCredentialSetupStates

const NOW = 1_783_000_000

export function VisualProviderCredentialSetupHarness({
  stateName
}: {
  readonly stateName: VisualProviderCredentialSetupState
}): JSX.Element {
  return (
    <SettingsView
      config={createDefaultAppShellState().config}
      deleteAllState={{ status: "idle" }}
      providerCredentialState={providerCredentialState(stateName)}
      syncScheduler={createDefaultSyncSchedulerState(NOW)}
      syncSchedulerNowUnixSeconds={NOW}
      onCancelInstallCodexCli={noop}
      onChange={noopConfigChange}
      onChangeAutomaticSyncInterval={noopIntervalChange}
      onCheckProviderCredential={noopAsync}
      onConfirmInstallCodexCli={noopAsync}
      onDeleteAll={noopDeleteAll}
      onOpenPrivacySettings={noopPrivacySettings}
      onStartCodexLogin={noopAsync}
      onToggleAutomaticSync={noop}
    />
  )
}

export function isVisualProviderCredentialSetupState(value: string): value is VisualProviderCredentialSetupState {
  return value in visualProviderCredentialSetupStates
}

function providerCredentialState(stateName: VisualProviderCredentialSetupState): ProviderCredentialState {
  switch (stateName) {
    case "missing-cli":
      return { status: "missingCli" }
    case "install-confirming":
      return { status: "installConfirming" }
    case "installing":
      return { status: "installing" }
    case "not-logged-in":
      return { status: "notLoggedIn" }
    case "login-polling":
      return { status: "loginPolling" }
    case "ready":
      return { status: "ready" }
    case "failed":
      return {
        status: "failed",
        message: "Codex provider setup could not be completed.",
        recoveryAction: "setup"
      }
    default:
      return assertNever(stateName)
  }
}

function noop(): void {}

async function noopAsync(): Promise<void> {}

function noopConfigChange(_config: AppConfig): void {}

function noopDeleteAll(_options: DeleteAllOptions): void {}

function noopIntervalChange(_intervalSeconds: SyncSchedulerIntervalSeconds): void {}

async function noopPrivacySettings(_pane: PrivacySettingsPane): Promise<void> {}

function assertNever(value: never): never {
  throw new Error(`Unhandled provider credential setup visual state: ${String(value)}`)
}
