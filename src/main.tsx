import { StrictMode } from "react"
import { createRoot } from "react-dom/client"
import { App } from "./App"
import { StatusView } from "./StatusView"
import {
  createDefaultAppShellState,
  getMenuModel,
  getOnboardingWarnings,
  isSyncNowEnabled,
  type AppShellState,
  type ChatId,
  type DiscoveredChat,
  type SelectedChat
} from "./domain/appShell"
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
  loading: true,
  unverified: true,
  permissionDenied: true,
  unavailable: true,
  empty: true,
  "ready-multiple": true,
  "ready-selected": true,
  "stale-selection": true
} as const

type VisualChatDiscoveryState = keyof typeof visualChatDiscoveryStates

const visualQaState = import.meta.env.DEV
  ? getVisualChatDiscoveryState(window.location.search)
  : undefined

createRoot(rootElement).render(
  <StrictMode>
    {visualQaState === undefined ? <App /> : <VisualChatDiscoveryHarness stateName={visualQaState} />}
  </StrictMode>
)

type VisualChatDiscoveryHarnessProps = {
  readonly stateName: VisualChatDiscoveryState
}

function VisualChatDiscoveryHarness({
  stateName
}: VisualChatDiscoveryHarnessProps): JSX.Element {
  const state = visualChatDiscoveryAppState(stateName)
  const menu = getMenuModel(state)
  const warnings = getOnboardingWarnings(state)
  const syncEnabled = isSyncNowEnabled(state)

  return (
    <main className="app-shell visual-qa-shell" data-visual-qa-state={stateName}>
      <aside className="sidebar" aria-label="Visual QA fixture">
        <h1>Morrow</h1>
        <p className="lede">Messages setup visual QA</p>
      </aside>
      <section className="content" aria-live="polite">
        <StatusView
          menu={menu}
          state={state}
          warnings={warnings}
          syncing={false}
          syncEnabled={syncEnabled}
          onPause={noop}
          onResume={noop}
          onSyncNow={noop}
          onRetryChatDiscovery={noop}
          onOpenFullDiskAccess={noop}
          onTogglePermissions={noopPermissionToggle}
          onToggleChat={noopChatToggle}
          onToggleBackfillPrompt={noopBackfillToggle}
        />
      </section>
    </main>
  )
}

function getVisualChatDiscoveryState(search: string): VisualChatDiscoveryState | undefined {
  const params = new URLSearchParams(search)
  if (params.get("visualQa") !== "chat-discovery") {
    return undefined
  }
  const stateName = params.get("state")
  if (stateName === null) {
    throw new Error("Visual QA state is required for chat discovery")
  }
  if (!isVisualChatDiscoveryState(stateName)) {
    throw new Error(`Unsupported visual QA chat discovery state: ${stateName}`)
  }
  return stateName
}

function isVisualChatDiscoveryState(value: string): value is VisualChatDiscoveryState {
  return value in visualChatDiscoveryStates
}

function visualChatDiscoveryAppState(stateName: VisualChatDiscoveryState): AppShellState {
  const baseState = createVisualBaseState()
  switch (stateName) {
    case "loading":
      return { ...baseState, config: { ...baseState.config, permissionsGranted: false }, discovery: { status: "loading", chats: [] } }
    case "unverified":
      return { ...baseState, discovery: { status: "unverified", chats: [] } }
    case "permissionDenied":
      return {
        ...baseState,
        config: { ...baseState.config, permissionsGranted: false },
        discovery: { status: "permissionDenied", chats: [] }
      }
    case "unavailable":
      return { ...baseState, discovery: { status: "unavailable", chats: [] } }
    case "empty":
      return { ...baseState, discovery: { status: "empty", chats: [] } }
    case "ready-multiple":
      return { ...baseState, discovery: { status: "ready", chats: visualDiscoveredChats } }
    case "ready-selected":
      return {
        ...baseState,
        discovery: { status: "ready", chats: visualDiscoveredChats },
        selectedChats: [selectedVisualChat]
      }
    case "stale-selection":
      return {
        ...baseState,
        discovery: { status: "ready", chats: visualDiscoveredChats.slice(1) },
        selectedChats: [staleSelectedVisualChat]
      }
    default:
      return assertNever(stateName)
  }
}

function createVisualBaseState(): AppShellState {
  const state = createDefaultAppShellState()
  return {
    ...state,
    config: {
      ...state.config,
      referenceTimezone: "Asia/Seoul",
      permissionsGranted: true
    },
    pendingProposalCount: 2
  }
}

const visualChatAlpha: DiscoveredChat = {
  id: "messages-chat-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  label: "Planning circle",
  participantCount: 2,
  participantIds: [
    "messages-participant-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "messages-participant-bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
  ],
  latestActivityTimestamp: 1_783_000_000
}

const visualChatBeta: DiscoveredChat = {
  id: "messages-chat-bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
  label: "Design review",
  participantCount: 3,
  participantIds: [
    "messages-participant-cccccccccccccccccccccccccccccccc",
    "messages-participant-dddddddddddddddddddddddddddddddd",
    "messages-participant-eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"
  ],
  latestActivityTimestamp: 1_782_914_400
}

const visualChatGamma: DiscoveredChat = {
  id: "messages-chat-cccccccccccccccccccccccccccccccc",
  label: "Messages chat",
  participantCount: 1,
  participantIds: ["messages-participant-ffffffffffffffffffffffffffffffff"],
  latestActivityTimestamp: 1_782_828_000
}

const visualDiscoveredChats = [visualChatAlpha, visualChatBeta, visualChatGamma] as const

const selectedVisualChat: SelectedChat = {
  ...visualChatAlpha,
  backfillPromptEnabled: true
}

const staleSelectedVisualChat: SelectedChat = {
  id: "messages-chat-dddddddddddddddddddddddddddddddd",
  label: "Previous planning circle",
  participantCount: 2,
  participantIds: [
    "messages-participant-11111111111111111111111111111111",
    "messages-participant-22222222222222222222222222222222"
  ],
  latestActivityTimestamp: 1_782_741_600,
  backfillPromptEnabled: true
}

function noop(): void {}

function noopPermissionToggle(_enabled: boolean): void {}

function noopChatToggle(_chatId: ChatId): void {}

function noopBackfillToggle(_chatId: ChatId, _enabled: boolean): void {}

function assertNever(value: never): never {
  throw new Error(`Unhandled visual QA state: ${String(value)}`)
}
