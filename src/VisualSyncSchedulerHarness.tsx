import { useState } from "react"
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
import {
  createDefaultSyncSchedulerState,
  enableSyncScheduler,
  markSyncSchedulerBlocked,
  markSyncSchedulerRetryableFailure,
  markSyncSchedulerRunning,
  type SyncSchedulerState
} from "./domain/syncScheduler"

export const visualSyncSchedulerStates = {
  disabled: true,
  enabled: true,
  running: true,
  cooldown: true,
  blocked: true
} as const

export type VisualSyncSchedulerState = keyof typeof visualSyncSchedulerStates

const NOW = 1_783_000_000

export function VisualSyncSchedulerHarness({
  stateName
}: {
  readonly stateName: VisualSyncSchedulerState
}): JSX.Element {
  const appState = createVisualSyncSchedulerAppState()
  const [scheduler, setScheduler] = useState(() => visualSyncSchedulerState(stateName))
  return (
    <StatusView
      menu={getMenuModel(appState)}
      state={appState}
      warnings={getOnboardingWarnings(appState)}
      syncing={stateName === "running"}
      syncEnabled={isSyncNowEnabled(appState)}
      syncScheduler={scheduler}
      syncSchedulerNowUnixSeconds={NOW}
      onChangeAutomaticSyncInterval={(intervalSeconds) =>
        setScheduler((current) => ({ ...current, interval_seconds: intervalSeconds }))
      }
      onHidePreviews={noop}
      onOpenFullDiskAccess={noop}
      onOpenSettings={noop}
      onPause={noop}
      onResume={noop}
      onRetryChatDiscovery={noop}
      onRevealPreviews={noop}
      onSyncNow={noop}
      onToggleAutomaticSync={() =>
        setScheduler((current) => (current.enabled ? createDefaultSyncSchedulerState(NOW) : enableSyncScheduler(current, NOW)))
      }
      onToggleBackfillPrompt={noopBackfillToggle}
      onToggleChat={noopChatToggle}
    />
  )
}

export function isVisualSyncSchedulerState(value: string): value is VisualSyncSchedulerState {
  return value in visualSyncSchedulerStates
}

function visualSyncSchedulerState(stateName: VisualSyncSchedulerState): SyncSchedulerState {
  const enabled = enableSyncScheduler(createDefaultSyncSchedulerState(NOW), NOW)
  switch (stateName) {
    case "disabled":
      return createDefaultSyncSchedulerState(NOW)
    case "enabled":
      return enabled
    case "running":
      return markSyncSchedulerRunning(enabled, NOW - 30)
    case "cooldown":
      return markSyncSchedulerRetryableFailure(enabled, {
        nowUnixSeconds: NOW,
        randomUnit: 0.5,
        reason: "Provider timed out."
      })
    case "blocked":
      return markSyncSchedulerBlocked(enabled, {
        nowUnixSeconds: NOW,
        reason: "Finish Codex CLI setup in Settings before scanning."
      })
    default:
      return assertNever(stateName)
  }
}

function createVisualSyncSchedulerAppState(): AppShellState {
  const state = createDefaultAppShellState()
  const providerCredentialStatus: AppShellState["providerCredentialStatus"] = "configured"
  return {
    ...state,
    config: { ...state.config, permissionsGranted: true },
    decisionEvidence: {
      items: [
        {
          subjectType: "candidate",
          candidateId: "candidate-visual-opaque-id-with-long-suffix-0123456789abcdef",
          candidateState: "draft",
          candidateKind: "task_reminder",
          route: "provider_candidate",
          reasonCode: "confidence_meets_threshold",
          confidenceMillis: 820,
          sourceExcerpt: "Source excerpt hidden by settings.",
          labelType: "candidate",
          labelValue: "created",
          sourceExcerptPolicy: "disabled",
          privacyTier: "safe",
          hasDiagnosticsHashes: true,
          createdAt: NOW,
          traceRetention: "notRetained",
          traceSequence: []
        },
        {
          subjectType: "quietLog",
          route: "provider",
          reasonCode: "provider_unavailable",
          providerDiagnostic: "codex provider command timed out",
          labelType: "quiet",
          labelValue: "rejected",
          sourceExcerptPolicy: "disabled",
          privacyTier: "safe",
          hasDiagnosticsHashes: false,
          createdAt: NOW - 30,
          traceRetention: "traceMissing",
          traceSequence: [
            {
              component: "scan",
              operation: "classify",
              decision: "quiet",
              outcome: "not actionable"
            }
          ]
        }
      ],
      skippedTraceLineCount: 0,
      latestEvalStatus: "needs_review"
    },
    discovery: { status: "ready", chats: [visualChat] },
    providerCredentialStatus,
    selectedChats: [selectedVisualChat]
  }
}

const visualChat: DiscoveredChat = {
  id: "messages-chat-99999999999999999999999999999999",
  label: "Planning circle",
  latestActivityTimestamp: NOW,
  participantCount: 2,
  participantIds: [
    "messages-participant-99999999999999999999999999999999",
    "messages-participant-88888888888888888888888888888888"
  ]
}

const selectedVisualChat: SelectedChat = { ...visualChat, backfillPromptEnabled: true }

function noop(): void {}

function noopChatToggle(_chatId: ChatId): void {}

function noopBackfillToggle(_chatId: ChatId, _enabled: boolean): void {}

function assertNever(value: never): never {
  throw new Error(`Unhandled visual scheduler variant: ${String(value)}`)
}
