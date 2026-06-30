import { ChatDiscoveryControls } from "./ChatDiscoveryControls"
import type { ChatPreviewDisclosure } from "./ChatPreviewControls"
import type { AppShellState, ChatId } from "./domain/appShell"
import type { RuntimeIdentity } from "./tauriBridge"

type ChatDiscoverySetupControlsProps = {
  readonly state: AppShellState
  readonly previewDisclosure?: ChatPreviewDisclosure | undefined
  readonly runtimeIdentity?: RuntimeIdentity | undefined
  readonly onRevealPreviews?: (() => void) | undefined
  readonly onHidePreviews?: (() => void) | undefined
  readonly onRetryChatDiscovery: () => void
  readonly onOpenFullDiskAccess: () => void
  readonly onToggleChat: (chatId: ChatId) => void
  readonly onToggleBackfillPrompt: (chatId: ChatId, enabled: boolean) => void
}

export function ChatDiscoverySetupControls({
  state,
  previewDisclosure,
  runtimeIdentity,
  onRevealPreviews,
  onHidePreviews,
  onRetryChatDiscovery,
  onOpenFullDiskAccess,
  onToggleChat,
  onToggleBackfillPrompt
}: ChatDiscoverySetupControlsProps): JSX.Element {
  return (
    <ChatDiscoveryControls
      discovery={state.discovery}
      previewDisclosure={previewDisclosure}
      referenceTimezone={state.config.referenceTimezone}
      selectedChats={state.selectedChats}
      runtimeIdentity={runtimeIdentity}
      onHidePreviews={onHidePreviews}
      onOpenFullDiskAccess={onOpenFullDiskAccess}
      onRevealPreviews={onRevealPreviews}
      onRetry={onRetryChatDiscovery}
      onToggleBackfillPrompt={onToggleBackfillPrompt}
      onToggleChat={onToggleChat}
    />
  )
}
