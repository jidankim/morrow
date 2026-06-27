import {
  AlertTriangle,
  CheckCircle2,
  Pause,
  Play,
  RefreshCw
} from "lucide-react"
import { ChatDiscoveryControls } from "./ChatDiscoveryControls"
import {
  type AppShellState,
  type ChatId,
  type MenuModel,
  type MenuStatusKind
} from "./domain/appShell"

type StatusViewProps = {
  readonly state: AppShellState
  readonly menu: MenuModel
  readonly warnings: readonly string[]
  readonly syncing: boolean
  readonly syncEnabled: boolean
  readonly onPause: () => void
  readonly onResume: () => void
  readonly onSyncNow: () => void
  readonly onRetryChatDiscovery: () => void
  readonly onOpenFullDiskAccess: () => void
  readonly onTogglePermissions: (enabled: boolean) => void
  readonly onToggleChat: (chatId: ChatId) => void
  readonly onToggleBackfillPrompt: (chatId: ChatId, enabled: boolean) => void
}

export function StatusView({
  state,
  menu,
  warnings,
  syncing,
  syncEnabled,
  onPause,
  onResume,
  onSyncNow,
  onRetryChatDiscovery,
  onOpenFullDiskAccess,
  onTogglePermissions,
  onToggleChat,
  onToggleBackfillPrompt
}: StatusViewProps): JSX.Element {
  const isPaused = state.mode === "paused"

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
      <OnboardingWarnings warnings={warnings} />
      <div className="command-row">
        <button className="button primary" onClick={isPaused ? onResume : onPause} type="button">
          {isPaused ? <Play aria-hidden="true" size={16} /> : <Pause aria-hidden="true" size={16} />}
          {menu.pauseResumeLabel}
        </button>
        <button
          className="button secondary"
          disabled={!syncEnabled || syncing}
          onClick={onSyncNow}
          type="button"
        >
          <RefreshCw aria-hidden="true" size={16} />
          {syncing ? "Syncing" : "Sync Now"}
        </button>
      </div>
      <dl className="state-grid">
        <Metric label="Scanning state" testId="status-label" value={menu.statusLabel} />
        <Metric label="Sync Now" testId="sync-state" value={syncEnabled ? "Enabled" : "Disabled"} />
        <Metric label="Pending proposals" testId="pending-count" value={menu.pendingProposalLabel} />
      </dl>
      <section className="setup-section" aria-labelledby="setup-heading">
        <h3 id="setup-heading">Onboarding</h3>
        <label className="check-row">
          <input
            checked={state.config.permissionsGranted}
            onChange={(event) => onTogglePermissions(event.currentTarget.checked)}
            type="checkbox"
          />
          <span>Required permissions complete</span>
        </label>
        <ChatDiscoveryControls
          discovery={state.discovery}
          selectedChats={state.selectedChats}
          onOpenFullDiskAccess={onOpenFullDiskAccess}
          onRetry={onRetryChatDiscovery}
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

type OnboardingWarningsProps = {
  readonly warnings: readonly string[]
}

function OnboardingWarnings({ warnings }: OnboardingWarningsProps): JSX.Element | null {
  if (warnings.length === 0) {
    return null
  }

  return (
    <div className="warning-surface" role="alert">
      <AlertTriangle aria-hidden="true" size={17} />
      <div>
        <h3>Onboarding required</h3>
        {warnings.map((warning) => (
          <p key={warning}>{warning}</p>
        ))}
      </div>
    </div>
  )
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
