import { StrictMode, useState } from "react"
import { createRoot } from "react-dom/client"
import { App } from "./App"
import { SettingsView } from "./SettingsView"
import { StatusView } from "./StatusView"
import type { ChatPreviewDisclosure } from "./ChatPreviewControls"
import { createDefaultAppShellState, getMenuModel, getOnboardingWarnings, isSyncNowEnabled, reduceAppShellState, type AppShellState, type ChatId } from "./domain/appShell"
import { createDefaultSyncSchedulerState, type SyncSchedulerIntervalSeconds } from "./domain/syncScheduler"
import type { RuntimeIdentity } from "./tauriBridge"
import { VisualProviderCredentialSetupHarness, isVisualProviderCredentialSetupState, type VisualProviderCredentialSetupState } from "./VisualProviderCredentialSetupHarness"
import { VisualSyncSchedulerHarness, isVisualSyncSchedulerState, type VisualSyncSchedulerState } from "./VisualSyncSchedulerHarness"
import {
  selectedVisualChat,
  staleSelectedVisualChat,
  visualChatPreviews,
  visualDiscoveredChats
} from "./visualQaChatFixtures"
import { VisualProviderUsageHarness, isVisualProviderUsageState, type VisualProviderUsageState } from "./visualQaProviderUsageRoute"
import { VisualQaShell } from "./visualQaShell"
import "./styles.css"

if (import.meta.env.DEV && import.meta.env["VITE_DISABLE_REACT_DEVTOOLS"] !== "1") {
  void import("react-grab")
  void import("react-scan")
}

const rootElement = document.getElementById("root")

if (rootElement === null) {
  throw new Error("Morrow root element is missing")
}

const visualChatDiscoveryStates = {
  loading: true, unverified: true, permissionDenied: true, unavailable: true, empty: true,
  "ready-multiple": true, "ready-selected": true, "ready-previews-hidden": true,
  "ready-previews-revealed": true, "provider-missing": true, "stale-selection": true
} as const

type VisualChatDiscoveryState = keyof typeof visualChatDiscoveryStates

const visualFullDiskAccessRecoveryStates = {
  "discovery-permission-denied-binary": true, "discovery-permission-denied-appBundle": true,
  "discovery-unavailable-binary": true, "settings-privacy-binary": true,
  "settings-privacy-appBundle": true
} as const

type VisualFullDiskAccessRecoveryState = keyof typeof visualFullDiskAccessRecoveryStates

type VisualQaState = { readonly kind: "chatDiscovery"; readonly stateName: VisualChatDiscoveryState }
  | { readonly kind: "fullDiskAccessRecovery"; readonly stateName: VisualFullDiskAccessRecoveryState }
  | { readonly kind: "providerCredentialSetup"; readonly stateName: VisualProviderCredentialSetupState }
  | { readonly kind: "providerUsage"; readonly stateName: VisualProviderUsageState }
  | { readonly kind: "syncScheduler"; readonly stateName: VisualSyncSchedulerState }

const visualQaState = import.meta.env.DEV ? getVisualQaState(window.location.search) : undefined
const VISUAL_QA_NOW_UNIX_SECONDS = 1_783_000_000
const visualSyncScheduler = createDefaultSyncSchedulerState(VISUAL_QA_NOW_UNIX_SECONDS)

createRoot(rootElement).render(<StrictMode>{visualQaState === undefined ? <App /> : <VisualQaHarness state={visualQaState} />}</StrictMode>)

function VisualQaHarness({ state }: { readonly state: VisualQaState }): JSX.Element {
  switch (state.kind) {
    case "chatDiscovery":
      return <VisualChatDiscoveryHarness stateName={state.stateName} />
    case "fullDiskAccessRecovery":
      return <VisualFullDiskAccessRecoveryHarness stateName={state.stateName} />
    case "providerCredentialSetup":
      return <VisualQaShell stateName={state.stateName} lede="Codex provider setup visual QA"><VisualProviderCredentialSetupHarness stateName={state.stateName} /></VisualQaShell>
    case "providerUsage":
      return <VisualProviderUsageHarness stateName={state.stateName} />
    case "syncScheduler":
      return <VisualQaShell stateName={state.stateName} lede="Automatic sync visual QA"><VisualSyncSchedulerHarness stateName={state.stateName} /></VisualQaShell>
    default:
      return assertNever(state)
  }
}

function VisualChatDiscoveryHarness({ stateName }: { readonly stateName: VisualChatDiscoveryState }): JSX.Element {
  const state = visualChatDiscoveryAppState(stateName)
  const previewDisclosure: ChatPreviewDisclosure | undefined =
    stateName === "ready-previews-hidden" ? { status: "hidden", previews: visualChatPreviews } :
    stateName === "ready-previews-revealed" ? { status: "ready", previews: visualChatPreviews } : undefined
  return (
    <VisualQaShell stateName={stateName} lede="Messages setup visual QA">
      <VisualStatusFixture state={state} previewDisclosure={previewDisclosure} runtimeIdentity={undefined} />
    </VisualQaShell>
  )
}

function VisualFullDiskAccessRecoveryHarness({ stateName }: { readonly stateName: VisualFullDiskAccessRecoveryState }): JSX.Element {
  const runtimeIdentity = visualRecoveryRuntimeIdentity(stateName)
  switch (stateName) {
    case "discovery-permission-denied-binary":
    case "discovery-permission-denied-appBundle":
    case "discovery-unavailable-binary": {
      const state = visualFullDiskAccessDiscoveryAppState(stateName)
      return (
        <VisualQaShell stateName={stateName}>
          <VisualStatusFixture state={state} runtimeIdentity={runtimeIdentity} />
        </VisualQaShell>
      )
    }
    case "settings-privacy-binary":
    case "settings-privacy-appBundle":
      return (
        <VisualQaShell stateName={stateName}>
          <SettingsView config={createVisualBaseState().config} deleteAllState={{ status: "idle" }}
            providerCredentialState={{ status: "ready" }} runtimeIdentity={runtimeIdentity}
            syncScheduler={visualSyncScheduler} syncSchedulerNowUnixSeconds={VISUAL_QA_NOW_UNIX_SECONDS}
            onChangeAutomaticSyncInterval={noopIntervalChange} onChange={noop}
            onCancelInstallCodexCli={noop} onCheckProviderCredential={noopAsync}
            onConfirmInstallCodexCli={noopAsync} onDeleteAll={noop} onOpenPrivacySettings={noopAsync}
            onStartCodexLogin={noopAsync}
            onToggleAutomaticSync={noop} />
        </VisualQaShell>
      )
    default:
      return assertNever(stateName)
  }
}

function VisualStatusFixture({ state, previewDisclosure, runtimeIdentity }: {
  readonly state: AppShellState
  readonly previewDisclosure?: ChatPreviewDisclosure | undefined
  readonly runtimeIdentity?: RuntimeIdentity | undefined
}): JSX.Element {
  const [fixtureState, setFixtureState] = useState(state)
  return (
    <StatusView
      menu={getMenuModel(fixtureState)} state={fixtureState} warnings={getOnboardingWarnings(fixtureState)} syncing={false}
      syncEnabled={isSyncNowEnabled(fixtureState)} previewDisclosure={previewDisclosure} runtimeIdentity={runtimeIdentity}
      syncScheduler={visualSyncScheduler} syncSchedulerNowUnixSeconds={VISUAL_QA_NOW_UNIX_SECONDS}
      onChangeAutomaticSyncInterval={noopIntervalChange}
      onPause={() => setFixtureState((current) => reduceAppShellState(current, { type: "pause" }))}
      onResume={() => setFixtureState((current) => reduceAppShellState(current, { type: "resume" }))}
      onSyncNow={noop} onOpenSettings={noop} onRetryChatDiscovery={noop}
      onOpenFullDiskAccess={noop} onRevealPreviews={noop} onHidePreviews={noop}
      onToggleAutomaticSync={noop} onToggleChat={noopChatToggle} onToggleBackfillPrompt={noopBackfillToggle}
    />
  )
}

function getVisualQaState(search: string): VisualQaState | undefined {
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

function isVisualChatDiscoveryState(value: string): value is VisualChatDiscoveryState { return value in visualChatDiscoveryStates }

function isVisualFullDiskAccessRecoveryState(value: string): value is VisualFullDiskAccessRecoveryState { return value in visualFullDiskAccessRecoveryStates }

function visualChatDiscoveryAppState(stateName: VisualChatDiscoveryState): AppShellState {
  const baseState = createVisualBaseState()
  switch (stateName) {
    case "loading":
      return { ...baseState, config: { ...baseState.config, permissionsGranted: false }, discovery: { status: "loading", chats: [] } }
    case "unverified":
      return { ...baseState, discovery: { status: "unverified", chats: [] } }
    case "permissionDenied":
      return { ...baseState, config: { ...baseState.config, permissionsGranted: false }, discovery: { status: "permissionDenied", chats: [] } }
    case "unavailable":
      return { ...baseState, discovery: { status: "unavailable", chats: [] } }
    case "empty":
      return { ...baseState, discovery: { status: "empty", chats: [] } }
    case "ready-multiple":
      return { ...baseState, discovery: { status: "ready", chats: visualDiscoveredChats } }
    case "ready-selected":
      return { ...baseState, discovery: { status: "ready", chats: visualDiscoveredChats }, selectedChats: [selectedVisualChat] }
    case "ready-previews-hidden":
    case "ready-previews-revealed":
      return { ...baseState, discovery: { status: "ready", chats: visualDiscoveredChats } }
    case "provider-missing":
      return { ...baseState, providerCredentialStatus: "missing", discovery: { status: "ready", chats: visualDiscoveredChats }, selectedChats: [selectedVisualChat] }
    case "stale-selection":
      return { ...baseState, discovery: { status: "ready", chats: visualDiscoveredChats.slice(1) }, selectedChats: [staleSelectedVisualChat] }
    default:
      return assertNever(stateName)
  }
}

function visualFullDiskAccessDiscoveryAppState(stateName: VisualFullDiskAccessRecoveryState): AppShellState {
  const baseState = createVisualBaseState()
  switch (stateName) {
    case "discovery-permission-denied-binary":
    case "discovery-permission-denied-appBundle":
      return { ...baseState, config: { ...baseState.config, permissionsGranted: false }, discovery: { status: "permissionDenied", chats: [] } }
    case "discovery-unavailable-binary":
      return { ...baseState, discovery: { status: "unavailable", chats: [] } }
    case "settings-privacy-binary":
    case "settings-privacy-appBundle":
      return baseState
    default:
      return assertNever(stateName)
  }
}

function createVisualBaseState(): AppShellState {
  const state = createDefaultAppShellState()
  return {
    ...state, config: { ...state.config, referenceTimezone: "Asia/Seoul", permissionsGranted: true },
    providerCredentialStatus: "configured", pendingProposalCount: 2, createdCandidateCount: 4,
    quietLogCount: 2, createdExternalProposalCount: 3,
    failedExternalProposalCount: 1
  }
}

const visualBinaryRuntimeIdentity: RuntimeIdentity = {
  displayName: "morrow", bundleIdentifier: "dev.morrow.local",
  executablePath: "/Users/example/workspace/morrow/src-tauri/target/debug/morrow",
  settingsTargetPath: "/Users/example/workspace/morrow/src-tauri/target/debug/morrow",
  runtimeKind: "binary"
}

const visualAppBundleRuntimeIdentity: RuntimeIdentity = {
  displayName: "Morrow.app", bundleIdentifier: "app.morrow.desktop",
  executablePath: "/Applications/Morrow.app/Contents/MacOS/Morrow", settingsTargetPath: "/Applications/Morrow.app",
  runtimeKind: "appBundle"
}

function visualRecoveryRuntimeIdentity(stateName: VisualFullDiskAccessRecoveryState): RuntimeIdentity {
  switch (stateName) {
    case "discovery-permission-denied-binary":
    case "discovery-unavailable-binary":
    case "settings-privacy-binary":
      return visualBinaryRuntimeIdentity
    case "discovery-permission-denied-appBundle":
    case "settings-privacy-appBundle":
      return visualAppBundleRuntimeIdentity
    default:
      return assertNever(stateName)
  }
}

function noop(): void {}

async function noopAsync(): Promise<void> {}

function noopIntervalChange(_intervalSeconds: SyncSchedulerIntervalSeconds): void {}

function noopChatToggle(_chatId: ChatId): void {}

function noopBackfillToggle(_chatId: ChatId, _enabled: boolean): void {}

function assertNever(value: never): never {
  throw new Error(`Unhandled visual QA state: ${String(value)}`)
}
