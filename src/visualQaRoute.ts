import { isVisualListIntakeState, type VisualListIntakeState } from "./VisualListIntakeHarness"
import { isVisualProviderCredentialSetupState, type VisualProviderCredentialSetupState } from "./VisualProviderCredentialSetupHarness"
import { isVisualSyncSchedulerState, type VisualSyncSchedulerState } from "./VisualSyncSchedulerHarness"
import { isVisualProviderUsageState, type VisualProviderUsageState } from "./visualQaProviderUsageRoute"

const visualChatDiscoveryStates = {
  loading: true, unverified: true, permissionDenied: true, unavailable: true, empty: true,
  "ready-multiple": true, "ready-selected": true, "ready-previews-hidden": true,
  "ready-previews-revealed": true, "provider-missing": true, "stale-selection": true
} as const

export type VisualChatDiscoveryState = keyof typeof visualChatDiscoveryStates

const visualFullDiskAccessRecoveryStates = {
  "discovery-permission-denied-binary": true, "discovery-permission-denied-appBundle": true,
  "discovery-unavailable-binary": true, "settings-privacy-binary": true,
  "settings-privacy-appBundle": true
} as const

export type VisualFullDiskAccessRecoveryState = keyof typeof visualFullDiskAccessRecoveryStates

export type VisualQaState = { readonly kind: "chatDiscovery"; readonly stateName: VisualChatDiscoveryState }
  | { readonly kind: "fullDiskAccessRecovery"; readonly stateName: VisualFullDiskAccessRecoveryState }
  | { readonly kind: "providerCredentialSetup"; readonly stateName: VisualProviderCredentialSetupState }
  | { readonly kind: "listIntake"; readonly stateName: VisualListIntakeState }
  | { readonly kind: "providerUsage"; readonly stateName: VisualProviderUsageState }
  | { readonly kind: "syncScheduler"; readonly stateName: VisualSyncSchedulerState }

export function getVisualQaState(search: string): VisualQaState | undefined {
  const params = new URLSearchParams(search)
  const suite = params.get("visualQa")
  if (suite === null) {
    return undefined
  }
  const stateName = params.get("state")
  if (stateName === null) {
    throw new Error(`Visual QA state is required for ${suite}`)
  }
  switch (suite) {
    case "chat-discovery":
      if (!isVisualChatDiscoveryState(stateName)) {
        throw new Error(`Unsupported visual QA chat discovery state: ${stateName}`)
      }
      return { kind: "chatDiscovery", stateName }
    case "full-disk-access-recovery":
      if (!isVisualFullDiskAccessRecoveryState(stateName)) {
        throw new Error(`Unsupported visual QA Full Disk Access recovery state: ${stateName}`)
      }
      return { kind: "fullDiskAccessRecovery", stateName }
    case "provider-credential-setup":
      if (!isVisualProviderCredentialSetupState(stateName)) {
        throw new Error(`Unsupported visual QA provider credential setup state: ${stateName}`)
      }
      return { kind: "providerCredentialSetup", stateName }
    case "list-intake":
      if (!isVisualListIntakeState(stateName)) {
        throw new Error(`Unsupported visual QA list intake state: ${stateName}`)
      }
      return { kind: "listIntake", stateName }
    case "provider-usage":
      if (!isVisualProviderUsageState(stateName)) {
        throw new Error(`Unsupported visual QA provider usage state: ${stateName}`)
      }
      return { kind: "providerUsage", stateName }
    case "sync-scheduler":
      if (!isVisualSyncSchedulerState(stateName)) {
        throw new Error(`Unsupported visual QA sync scheduler state: ${stateName}`)
      }
      return { kind: "syncScheduler", stateName }
    default:
      return undefined
  }
}

function isVisualChatDiscoveryState(value: string): value is VisualChatDiscoveryState {
  return value in visualChatDiscoveryStates
}

function isVisualFullDiskAccessRecoveryState(value: string): value is VisualFullDiskAccessRecoveryState {
  return value in visualFullDiskAccessRecoveryStates
}
