import { DatabaseZap, MessageSquare, RefreshCw } from "lucide-react"
import {
  type ChatDiscovery,
  type ChatId,
  type SelectedChat
} from "./domain/appShell"

type ChatDiscoveryControlsProps = {
  readonly discovery: ChatDiscovery
  readonly selectedChats: readonly SelectedChat[]
  readonly onRetry: () => void
  readonly onOpenFullDiskAccess: () => void
  readonly onToggleChat: (chatId: ChatId) => void
  readonly onToggleBackfillPrompt: (chatId: ChatId, enabled: boolean) => void
}

export function ChatDiscoveryControls({
  discovery,
  selectedChats,
  onRetry,
  onOpenFullDiskAccess,
  onToggleChat,
  onToggleBackfillPrompt
}: ChatDiscoveryControlsProps): JSX.Element {
  switch (discovery.status) {
    case "loading":
      return <DiscoveryState icon={<RefreshCw aria-hidden="true" size={15} />} label="Loading Messages chats..." />
    case "empty":
      return <DiscoveryState actionLabel="Retry chat discovery" label="No Messages chats found." onAction={onRetry} />
    case "permissionDenied":
      return (
        <DiscoveryState
          actionLabel="Retry chat discovery"
          recoveryLabel="Open Full Disk Access"
          label="Messages access denied."
          onAction={onRetry}
          onRecovery={onOpenFullDiskAccess}
        />
      )
    case "unavailable":
    case "unverified":
      return (
        <DiscoveryState
          actionLabel="Retry chat discovery"
          recoveryLabel="Open Full Disk Access"
          label="Messages discovery unavailable."
          onAction={onRetry}
          onRecovery={onOpenFullDiskAccess}
        />
      )
    case "ready":
      return (
        <div className="chat-list" aria-label="Chats to monitor">
          {discovery.chats.map((chat) => {
            const selectedChat = selectedChats.find((selected) => selected.id === chat.id)
            return (
              <ChatChoice
                chatId={chat.id}
                key={chat.id}
                label={chat.label}
                selected={selectedChat !== undefined}
                participantCount={chat.participantCount}
                backfillEnabled={selectedChat?.backfillPromptEnabled ?? false}
                onToggleChat={onToggleChat}
                onToggleBackfillPrompt={onToggleBackfillPrompt}
              />
            )
          })}
        </div>
      )
  }
}

type DiscoveryStateProps = {
  readonly label: string
  readonly icon?: JSX.Element
  readonly actionLabel?: string
  readonly recoveryLabel?: string
  readonly onAction?: () => void
  readonly onRecovery?: () => void
}

function DiscoveryState({
  label,
  icon,
  actionLabel,
  recoveryLabel,
  onAction,
  onRecovery
}: DiscoveryStateProps): JSX.Element {
  return (
    <div className="discovery-state">
      <p className="empty-state">
        {icon ?? <DatabaseZap aria-hidden="true" size={15} />}
        {label}
      </p>
      {actionLabel !== undefined || recoveryLabel !== undefined ? (
        <div className="discovery-actions">
          {actionLabel !== undefined ? (
            <button className="button secondary" onClick={onAction} type="button">
              <RefreshCw aria-hidden="true" size={15} />
              {actionLabel}
            </button>
          ) : null}
          {recoveryLabel !== undefined ? (
            <button className="button secondary" onClick={onRecovery} type="button">
              <DatabaseZap aria-hidden="true" size={15} />
              {recoveryLabel}
            </button>
          ) : null}
        </div>
      ) : null}
    </div>
  )
}

type ChatChoiceProps = {
  readonly chatId: ChatId
  readonly label: string
  readonly participantCount: number
  readonly selected: boolean
  readonly backfillEnabled: boolean
  readonly onToggleChat: (chatId: ChatId) => void
  readonly onToggleBackfillPrompt: (chatId: ChatId, enabled: boolean) => void
}

function ChatChoice({
  chatId,
  label,
  participantCount,
  selected,
  backfillEnabled,
  onToggleChat,
  onToggleBackfillPrompt
}: ChatChoiceProps): JSX.Element {
  return (
    <div className="chat-choice">
      <label className="check-row">
        <input checked={selected} onChange={() => onToggleChat(chatId)} type="checkbox" />
        <span>
          <MessageSquare aria-hidden="true" size={15} />
          {label}
          <small>{participantCount} participants</small>
        </span>
      </label>
      {selected ? (
        <label className="check-row nested-check">
          <input
            checked={backfillEnabled}
            onChange={(event) => onToggleBackfillPrompt(chatId, event.currentTarget.checked)}
            type="checkbox"
          />
          <span>Ask before backfill</span>
        </label>
      ) : null}
    </div>
  )
}
