import type { ChatPreviewDisclosure } from "./ChatPreviewControls"
import type { AppShellState, ChatId, MenuModel } from "./domain/appShell"
import type { SyncSchedulerIntervalSeconds, SyncSchedulerState } from "./domain/syncScheduler"
import type { RuntimeIdentity } from "./tauriBridge"

export type StatusViewProps = {
  readonly state: AppShellState
  readonly menu: MenuModel
  readonly warnings: readonly string[]
  readonly syncing: boolean
  readonly syncEnabled: boolean
  readonly previewDisclosure?: ChatPreviewDisclosure | undefined
  readonly runtimeIdentity?: RuntimeIdentity | undefined
  readonly syncScheduler: SyncSchedulerState
  readonly syncSchedulerNowUnixSeconds: number
  readonly onRevealPreviews?: () => void
  readonly onHidePreviews?: () => void
  readonly onChangeAutomaticSyncInterval: (intervalSeconds: SyncSchedulerIntervalSeconds) => void
  readonly onPause: () => void
  readonly onResume: () => void
  readonly onSyncNow: () => void
  readonly onToggleAutomaticSync: () => void
  readonly onRetryChatDiscovery: () => void
  readonly onOpenFullDiskAccess: () => void
  readonly onOpenSettings: () => void
  readonly onToggleChat: (chatId: ChatId) => void
  readonly onToggleBackfillPrompt: (chatId: ChatId, enabled: boolean) => void
}
