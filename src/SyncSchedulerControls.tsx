import {
  Clock3,
  PauseCircle,
  Power,
  PowerOff,
  RefreshCw,
  TimerReset,
  TriangleAlert
} from "lucide-react"
import {
  SYNC_SCHEDULER_INTERVAL_OPTIONS,
  formatSyncSchedulerCooldownReason,
  formatSyncSchedulerLastResult,
  formatSyncSchedulerNextRun,
  parseSyncSchedulerIntervalSeconds,
  type SyncSchedulerIntervalSeconds,
  type SyncSchedulerState,
  type SyncSchedulerStatus
} from "./domain/syncScheduler"

type SyncSchedulerControlsProps = {
  readonly nowUnixSeconds: number
  readonly scheduler: SyncSchedulerState
  readonly surface: "settings" | "status"
  readonly onIntervalChange: (intervalSeconds: SyncSchedulerIntervalSeconds) => void
  readonly onToggle: () => void
}

export function SyncSchedulerControls({
  nowUnixSeconds,
  scheduler,
  surface,
  onIntervalChange,
  onToggle
}: SyncSchedulerControlsProps): JSX.Element {
  const statusLabel = automaticSyncStatusLabel(scheduler.status)
  const toggleLabel = scheduler.enabled ? "Turn automatic sync off" : "Turn automatic sync on"
  const sectionClassName =
    surface === "settings" ? "settings-section sync-scheduler-section" : "sync-scheduler-section"

  return (
    <section className={sectionClassName} aria-labelledby={`${surface}-automatic-sync-heading`}>
      <div className="sync-scheduler-header">
        <div>
          <p className="eyebrow">Automatic Sync</p>
          <h3 id={`${surface}-automatic-sync-heading`}>Automatic sync</h3>
        </div>
        <span
          className={`status-pill sync-scheduler-pill ${scheduler.status}`}
          data-testid={`${surface}-automatic-sync-status`}
          role="status"
        >
          {automaticSyncStatusIcon(scheduler.status)}
          {statusLabel}
        </span>
      </div>
      <div className="sync-scheduler-controls">
        <button className="button secondary" onClick={onToggle} type="button">
          {scheduler.enabled ? <PowerOff aria-hidden="true" size={16} /> : <Power aria-hidden="true" size={16} />}
          {toggleLabel}
        </button>
        <label className="field sync-scheduler-interval">
          <span>Interval</span>
          <select
            aria-label="Automatic Sync interval"
            value={scheduler.interval_seconds}
            onChange={(event) =>
              onIntervalChange(parseSyncSchedulerIntervalSeconds(Number(event.currentTarget.value)))
            }
          >
            {SYNC_SCHEDULER_INTERVAL_OPTIONS.map((intervalSeconds) => (
              <option key={intervalSeconds} value={intervalSeconds}>
                {automaticSyncIntervalLabel(intervalSeconds)}
              </option>
            ))}
          </select>
        </label>
      </div>
      <dl className="sync-scheduler-details">
        <div>
          <dt>Next run</dt>
          <dd data-testid={`${surface}-automatic-sync-next-run`}>
            {formatSyncSchedulerNextRun(scheduler, nowUnixSeconds)}
          </dd>
        </div>
        <div>
          <dt>Last result</dt>
          <dd>{formatSyncSchedulerLastResult(scheduler)}</dd>
        </div>
        <div>
          <dt>Reason</dt>
          <dd data-testid={`${surface}-automatic-sync-reason`}>{automaticSyncReason(scheduler)}</dd>
        </div>
      </dl>
    </section>
  )
}

function automaticSyncIntervalLabel(intervalSeconds: SyncSchedulerIntervalSeconds): string {
  switch (intervalSeconds) {
    case 900:
      return "15 min"
    case 1_800:
      return "30 min"
    case 3_600:
      return "60 min"
    default:
      return assertNever(intervalSeconds)
  }
}

function automaticSyncStatusLabel(status: SyncSchedulerStatus): string {
  switch (status) {
    case "disabled":
      return "Off"
    case "scheduled":
      return "Enabled"
    case "running":
      return "Running"
    case "cooldown":
      return "Cooling down"
    case "blocked":
      return "Needs action"
    default:
      return assertNever(status)
  }
}

function automaticSyncReason(scheduler: SyncSchedulerState): string {
  switch (scheduler.status) {
    case "disabled":
      return scheduler.last_reason ?? "Automatic sync is off."
    case "scheduled":
      return scheduler.last_reason ?? "Automatic sync is scheduled."
    case "running":
      return scheduler.last_reason ?? "Automatic sync is running."
    case "cooldown":
      return formatSyncSchedulerCooldownReason(scheduler)
    case "blocked":
      return scheduler.last_reason ?? "Automatic sync needs action."
    default:
      return assertNever(scheduler.status)
  }
}

function automaticSyncStatusIcon(status: SyncSchedulerStatus): JSX.Element {
  switch (status) {
    case "disabled":
      return <PauseCircle aria-hidden="true" size={15} />
    case "scheduled":
      return <Clock3 aria-hidden="true" size={15} />
    case "running":
      return <RefreshCw aria-hidden="true" size={15} />
    case "cooldown":
      return <TimerReset aria-hidden="true" size={15} />
    case "blocked":
      return <TriangleAlert aria-hidden="true" size={15} />
    default:
      return assertNever(status)
  }
}

function assertNever(value: never): never {
  throw new Error(`Unhandled automatic sync variant: ${String(value)}`)
}
