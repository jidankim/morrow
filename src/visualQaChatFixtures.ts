import type { ChatId, DiscoveredChat, SelectedChat } from "./domain/appShell"

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

export const visualDiscoveredChats = [visualChatAlpha, visualChatBeta, visualChatGamma] as const

export const visualChatPreviews = new Map<ChatId, string>([
  [visualChatAlpha.id, "Agenda moved to Thursday afternoon; bring the launch notes and confirm room setup before review."],
  [visualChatBeta.id, "Design notes are ready with the calmer status copy and the final checklist grouped by topic."],
  [visualChatGamma.id, "Quick reminder to compare the short list before choosing which thread stays selected."]
])

export const selectedVisualChat: SelectedChat = { ...visualChatAlpha, backfillPromptEnabled: true }

export const staleSelectedVisualChat: SelectedChat = {
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
