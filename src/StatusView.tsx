import {
  AlertTriangle,
  CheckCircle2,
  KeyRound,
  Pause,
  Play,
  RefreshCw
} from "lucide-react"
import type { ChatPreviewDisclosure } from "./ChatPreviewControls"
import { ChatDiscoverySetupControls } from "./ChatDiscoverySetupControls"
import { StatusOnboardingWarnings } from "./StatusOnboardingWarnings"
import {
  type AppShellState,
  type ChatId,
  type MenuModel,
  type MenuStatusKind,
  type SyncReadinessItem,
  type SyncReadinessItemStatus,
  getSyncReadinessItems
} from "./domain/appShell"
import { formatLatestEvalStatus } from "./domain/syncResultCounts"
import type { RuntimeIdentity } from "./tauriBridge"

type StatusViewProps = {
  readonly state: AppShellState
  readonly menu: MenuModel
  readonly warnings: readonly string[]
  readonly syncing: boolean
  readonly syncEnabled: boolean
  readonly previewDisclosure?: ChatPreviewDisclosure | undefined
  readonly runtimeIdentity?: RuntimeIdentity | undefined
  readonly onRevealPreviews?: () => void
  readonly onHidePreviews?: () => void
  readonly onPause: () => void
  readonly onResume: () => void
  readonly onSyncNow: () => void
  readonly onRetryChatDiscovery: () => void
  readonly onOpenFullDiskAccess: () => void
  readonly onOpenSettings: () => void
  readonly onToggleChat: (chatId: ChatId) => void
  readonly onToggleBackfillPrompt: (chatId: ChatId, enabled: boolean) => void
}

export function StatusView({
  state,
  menu,
  warnings,
  syncing,
  syncEnabled,
  previewDisclosure,
  runtimeIdentity,
  onRevealPreviews,
  onHidePreviews,
  onPause,
  onResume,
  onSyncNow,
  onRetryChatDiscovery,
  onOpenFullDiskAccess,
  onOpenSettings,
  onToggleChat,
  onToggleBackfillPrompt
}: StatusViewProps): JSX.Element {
  const isPaused = state.mode === "paused"
  const readinessItems = getSyncReadinessItems(state, { syncing })
  const setupReadinessItems = readinessItems.filter(isSetupReadinessItem)
  const syncReadinessMessage = getSyncReadinessMessage(syncEnabled, readinessItems)

  return (
    <div className="panel">
      <div className="panel-header">
        <div>
          <p className="eyebrow">Menu status</p>
          <h2>App shell</h2>
        </div>
        <StatusPill statusKind={menu.statusKind} label={menu.statusLabel} />
      </div>
      <p className="lede">{menu.detail}</p>
      <StatusOnboardingWarnings warnings={warnings} />
      <div className="command-row">
        <button className="button primary" onClick={isPaused ? onResume : onPause} type="button">
          {isPaused ? <Play aria-hidden="true" size={16} /> : <Pause aria-hidden="true" size={16} />}
          {menu.pauseResumeLabel}
        </button>
        <button
          className="button secondary"
          data-visual-qa-control="sync-now"
          disabled={!syncEnabled || syncing}
          aria-describedby="sync-readiness-summary"
          onClick={onSyncNow}
          type="button"
        >
          <RefreshCw aria-hidden="true" size={16} />
          {syncing ? "Syncing" : "Sync Now"}
        </button>
      </div>
      <p
        className="sync-readiness-summary"
        data-visual-qa-text="sync-readiness-summary"
        id="sync-readiness-summary"
      >
        {syncReadinessMessage}
      </p>
      <dl className="state-grid">
        <Metric label="Sync Now state" testId="status-label" value={menu.statusLabel} />
        <Metric label="Sync Now" testId="sync-state" value={syncEnabled ? "Enabled" : "Disabled"} />
        <Metric label="Pending proposals" testId="pending-count" value={menu.pendingProposalLabel} />
        <Metric label="Sync results" testId="sync-result-counts" value={menu.syncResultLabel} />
        <Metric label="Feedback labels" testId="feedback-label-count" value={String(state.feedbackLabelCount)} />
        <Metric label="Eval snapshots" testId="feature-snapshot-count" value={String(state.featureSnapshotCount)} />
        <Metric label="Latest eval" testId="latest-eval-status" value={formatLatestEvalStatus(state.latestEvalStatus)} />
      </dl>
      <section className="setup-section" aria-labelledby="setup-heading">
        <h3 id="setup-heading">Setup readiness</h3>
        <ul className="readiness-list" aria-label="Sync Now setup checklist">
          {setupReadinessItems.map((item) => (
            <ReadinessItem item={item} key={item.id} />
          ))}
        </ul>
        {state.providerCredentialStatus === "missing" ? (
          <button className="button secondary" onClick={onOpenSettings} type="button">
            <KeyRound aria-hidden="true" size={16} />
            Configure provider
          </button>
        ) : null}
        <ChatDiscoverySetupControls
          state={state}
          previewDisclosure={previewDisclosure}
          runtimeIdentity={runtimeIdentity}
          onHidePreviews={onHidePreviews}
          onOpenFullDiskAccess={onOpenFullDiskAccess}
          onRevealPreviews={onRevealPreviews}
          onRetryChatDiscovery={onRetryChatDiscovery}
          onToggleBackfillPrompt={onToggleBackfillPrompt}
          onToggleChat={onToggleChat}
        />
      </section>
      {state.config.firstProposalGuidanceEnabled ? (
        <p className="guidance">
          First proposal guidance is on. New chat selections ask before backfilling older messages.
        </p>
      ) : null}
    </div>
  )
}

type ReadinessItemProps = {
  readonly item: SyncReadinessItem
}

function ReadinessItem({ item }: ReadinessItemProps): JSX.Element {
  return (
    <li
      className={`readiness-item ${item.status}`}
      aria-label={`${item.label}: ${formatReadinessStatus(item.status)}. ${item.detail}`}
    >
      {readinessStatusIcon(item.status)}
      <span>
        <strong data-visual-qa-text={`setup-${item.id}-label`}>{item.label}</strong>
        <small>{formatReadinessStatus(item.status)}</small>
        <span>{item.detail}</span>
      </span>
    </li>
  )
}

type MetricProps = {
  readonly label: string
  readonly testId: string
  readonly value: string
}

function Metric({ label, testId, value }: MetricProps): JSX.Element {
  return (
    <div>
      <dt>{label}</dt>
      <dd data-testid={testId}>{value}</dd>
    </div>
  )
}

function getSyncReadinessMessage(
  syncEnabled: boolean,
  readinessItems: readonly SyncReadinessItem[]
): string {
  if (syncEnabled) {
    return "Sync Now ready: All setup checks are complete."
  }
  const blockingItem = readinessItems.find((item) => item.status === "blocking")
  return `Sync Now disabled: ${blockingItem?.detail ?? "Complete setup before scanning."}`
}

function isSetupReadinessItem(item: SyncReadinessItem): boolean {
  switch (item.id) {
    case "discovery":
    case "chat-selection":
    case "selected-chat-verification":
    case "provider-credential":
      return true
    case "pause-state":
    case "sync-activity":
      return false
    default:
      return assertNever(item.id)
  }
}

function formatReadinessStatus(status: SyncReadinessItemStatus): string {
  switch (status) {
    case "complete":
      return "Complete"
    case "blocking":
      return "Needs action"
    case "pending":
      return "Pending"
    default:
      return assertNever(status)
  }
}

function readinessStatusIcon(status: SyncReadinessItemStatus): JSX.Element {
  switch (status) {
    case "complete":
      return <CheckCircle2 aria-hidden="true" size={15} />
    case "blocking":
      return <AlertTriangle aria-hidden="true" size={15} />
    case "pending":
      return <RefreshCw aria-hidden="true" size={15} />
    default:
      return assertNever(status)
  }
}

type StatusPillProps = {
  readonly statusKind: MenuStatusKind
  readonly label: string
}

function StatusPill({ statusKind, label }: StatusPillProps): JSX.Element {
  return (
    <span className={`status-pill ${statusKind}`} role="status">
      {statusKind === "setup-needed" ? <AlertTriangle aria-hidden="true" size={15} /> : null}
      {statusKind === "error" ? <AlertTriangle aria-hidden="true" size={15} /> : null}
      {statusKind === "scanning" ? <CheckCircle2 aria-hidden="true" size={15} /> : null}
      {statusKind === "paused" ? <Pause aria-hidden="true" size={15} /> : null}
      {label}
    </span>
  )
}

function assertNever(value: never): never {
  throw new Error(`Unhandled readiness variant: ${String(value)}`)
}
