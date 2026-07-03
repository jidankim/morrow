import { DatabaseZap, MessageSquare, RefreshCw } from "lucide-react"
import type { ChatDiscovery, ChatId, DiscoveredChat, SelectedChat } from "./domain/appShell"
import { chatDisplayMetadata, currentChatDisplaySystemTimeZone } from "./chatDisplay"
import { ChatPreviewControls, type ChatPreviewDisclosure } from "./ChatPreviewControls"
import { FullDiskAccessRecoveryGuide } from "./FullDiskAccessRecoveryGuide"
import type { RuntimeIdentity } from "./tauriBridge"

type ChatDiscoveryControlsProps = {
  readonly discovery: ChatDiscovery
  readonly previewDisclosure?: ChatPreviewDisclosure | undefined
  readonly referenceTimezone: string
  readonly selectedChats: readonly SelectedChat[]
  readonly runtimeIdentity?: RuntimeIdentity | undefined
  readonly onRevealPreviews?: (() => void) | undefined
  readonly onHidePreviews?: (() => void) | undefined
  readonly onRetry: () => void
  readonly onOpenFullDiskAccess: () => void
  readonly onToggleChat: (chatId: ChatId) => void
  readonly onToggleBackfillPrompt: (chatId: ChatId, enabled: boolean) => void
}

export function ChatDiscoveryControls({
  discovery,
  previewDisclosure,
  referenceTimezone,
  selectedChats,
  runtimeIdentity,
  onRevealPreviews,
  onHidePreviews,
  onRetry,
  onOpenFullDiskAccess,
  onToggleChat,
  onToggleBackfillPrompt
}: ChatDiscoveryControlsProps): JSX.Element {
  switch (discovery.status) {
    case "loading":
      return (
        <DiscoveryState
          icon={<RefreshCw aria-hidden="true" size={15} />}
          label="Morrow is checking local Messages access."
        />
      )
    case "unverified":
      return (
        <DiscoveryState
          actionLabel="Retry chat discovery"
          label="Messages discovery has not completed yet."
          onAction={onRetry}
        />
      )
    case "empty":
      return (
        <DiscoveryState
          actionLabel="Retry chat discovery"
          detail="Retry after new Messages conversations appear."
          label="Messages discovery finished, but found no eligible chats."
          onAction={onRetry}
        />
      )
    case "permissionDenied":
      return (
        <FullDiskAccessRecoveryGuide
          runtimeIdentity={runtimeIdentity}
          surface="discovery"
          onOpenFullDiskAccess={onOpenFullDiskAccess}
          onRetry={onRetry}
        />
      )
    case "unavailable":
      return (
        <FullDiskAccessRecoveryGuide
          runtimeIdentity={runtimeIdentity}
          surface="discovery"
          onOpenFullDiskAccess={onOpenFullDiskAccess}
          onRetry={onRetry}
        />
      )
    case "ready":
      const previewControlsAvailable =
        previewDisclosure !== undefined && onRevealPreviews !== undefined && onHidePreviews !== undefined
      return (
        <div
          className="chat-list"
          aria-label="Chats to monitor"
          data-runtime-kind={runtimeIdentity?.runtimeKind}
        >
          <p className="chat-source-summary" data-visual-qa-text="chat-source-summary">
            Messages source: {formatEligibleChatCount(discovery.chats.length)},{" "}
            {formatSelectedDiscoveredChatCount(discovery, selectedChats)}.
          </p>
          {previewControlsAvailable ? (
            <ChatPreviewControls
              previewDisclosure={previewDisclosure}
              onHidePreviews={onHidePreviews}
              onRevealPreviews={onRevealPreviews}
              onRetry={onRetry}
            />
          ) : null}
          {discovery.chats.map((chat) => {
            const selectedChat = selectedChats.find((selected) => selected.id === chat.id)
            return (
              <ChatChoice
                chat={chat}
                key={chat.id}
                previewText={
                  previewDisclosure?.status === "ready"
                    ? previewDisclosure.previews.get(chat.id) || undefined
                    : undefined
                }
                selected={selectedChat !== undefined}
                referenceTimezone={referenceTimezone}
                backfillEnabled={selectedChat?.backfillPromptEnabled ?? false}
                onToggleChat={onToggleChat}
                onToggleBackfillPrompt={onToggleBackfillPrompt}
              />
            )
          })}
        </div>
      )
    default:
      return assertNever(discovery.status)
  }
}

type DiscoveryStateProps = {
  readonly label: string
  readonly detail?: string
  readonly icon?: JSX.Element
  readonly actionLabel?: string
  readonly onAction?: () => void
}

function DiscoveryState({
  label,
  detail,
  icon,
  actionLabel,
  onAction
}: DiscoveryStateProps): JSX.Element {
  return (
    <div className="discovery-state" role="status">
      <p className="empty-state" data-visual-qa-text="discovery-state-label">
        {icon ?? <DatabaseZap aria-hidden="true" size={15} />}
        {label}
      </p>
      {detail !== undefined ? (
        <p className="discovery-detail" data-visual-qa-text="discovery-state-detail">
          {detail}
        </p>
      ) : null}
      {actionLabel !== undefined ? (
        <div className="discovery-actions">
          <button
            className="button secondary"
            data-visual-qa-control="retry-discovery"
            onClick={onAction}
            type="button"
          >
            <RefreshCw aria-hidden="true" size={15} />
            {actionLabel}
          </button>
        </div>
      ) : null}
    </div>
  )
}

type ChatChoiceProps = {
  readonly chat: DiscoveredChat
  readonly previewText?: string | undefined
  readonly referenceTimezone: string
  readonly selected: boolean
  readonly backfillEnabled: boolean
  readonly onToggleChat: (chatId: ChatId) => void
  readonly onToggleBackfillPrompt: (chatId: ChatId, enabled: boolean) => void
}

function ChatChoice({
  chat,
  previewText,
  referenceTimezone,
  selected,
  backfillEnabled,
  onToggleChat,
  onToggleBackfillPrompt
}: ChatChoiceProps): JSX.Element {
  const display = chatDisplayMetadata(chat, referenceTimezone, currentChatDisplaySystemTimeZone())
  const previewAccessibleText =
    previewText === undefined ? "" : `, Latest preview: ${previewText}`

  return (
    <div
      className={`chat-choice ${selected ? "selected" : "not-selected"}`}
      data-visual-qa-row={selected ? "selected-chat" : "chat"}
    >
      <label className="check-row chat-choice-main">
        <input
          aria-label={`${display.label}${previewAccessibleText}, ${display.participantCountText}, ${display.latestActivityText}, ${
            selected ? "selected" : "not selected"
          }`}
          checked={selected}
          data-visual-qa-control="chat-checkbox"
          onChange={() => onToggleChat(chat.id)}
          type="checkbox"
        />
        <span>
          <MessageSquare aria-hidden="true" size={15} />
          <strong data-visual-qa-text="chat-row-label">{display.label}</strong>
          <small data-visual-qa-text="chat-row-participant-count">
            {display.participantCountText}
          </small>
          <small data-visual-qa-text="chat-row-recency">{display.latestActivityText}</small>
          <small className="chat-selection-state">{selected ? "Selected" : "Not selected"}</small>
          {previewText !== undefined ? (
            <span className="chat-row-preview" data-visual-qa-text="chat-row-preview">
              {previewText}
            </span>
          ) : null}
        </span>
      </label>
      {selected ? (
        <div className="backfill-prompt">
          <label className="check-row nested-check">
            <input
              checked={backfillEnabled}
              data-visual-qa-control="backfill-checkbox"
              onChange={(event) => onToggleBackfillPrompt(chat.id, event.currentTarget.checked)}
              type="checkbox"
            />
            <span data-visual-qa-text="backfill-label">Ask before backfilling older messages</span>
          </label>
          <p data-visual-qa-text="backfill-detail">
            Morrow asks before using older messages from this chat.
          </p>
        </div>
      ) : null}
    </div>
  )
}

function formatEligibleChatCount(count: number): string {
  return `${count} eligible ${count === 1 ? "chat" : "chats"}`
}

function formatSelectedDiscoveredChatCount(
  discovery: ChatDiscovery,
  selectedChats: readonly SelectedChat[]
): string {
  const selectedCount = discovery.chats.filter((chat) =>
    selectedChats.some((selectedChat) => selectedChat.id === chat.id)
  ).length
  return `${selectedCount} selected`
}

function assertNever(value: never): never {
  throw new Error(`Unhandled chat discovery state: ${String(value)}`)
}
