import { isRawHandleLike, type DiscoveredChat } from "./domain/chatDiscovery"

const DEFAULT_CHAT_LABEL = "Messages chat"

export type ChatDisplayMetadata = {
  readonly label: string
  readonly participantCountText: string
  readonly latestActivityText: string
}

export function chatDisplayMetadata(
  chat: DiscoveredChat,
  referenceTimezone: string
): ChatDisplayMetadata {
  return {
    label: displaySafeChatLabel(chat.label),
    participantCountText: formatParticipantCount(chat.participantCount),
    latestActivityText: formatLatestActivityTimestamp(chat.latestActivityTimestamp, referenceTimezone)
  }
}

function displaySafeChatLabel(label: string): string {
  const trimmedLabel = label.trim()
  return trimmedLabel.length === 0 || isRawHandleLike(trimmedLabel) ? DEFAULT_CHAT_LABEL : trimmedLabel
}

export function formatParticipantCount(participantCount: number): string {
  return participantCount === 1 ? "1 participant" : `${participantCount} participants`
}

export function formatLatestActivityTimestamp(
  latestActivityTimestamp: number,
  referenceTimezone: string
): string {
  const latestActivityDate = new Date(latestActivityTimestamp * 1000)
  const formattedTimestamp = new Intl.DateTimeFormat("en-US", {
    timeZone: referenceTimezone,
    month: "short",
    day: "numeric",
    year: "numeric",
    hour: "numeric",
    minute: "2-digit"
  }).format(latestActivityDate)
  return `Last active ${formattedTimestamp}`
}
