import { z } from "zod"

export type ChatId = string
export type ParticipantId = string
export type ChatDiscoveryStatus =
  | "unverified"
  | "loading"
  | "ready"
  | "empty"
  | "permissionDenied"
  | "unavailable"

export type DiscoveredChat = {
  readonly id: ChatId
  readonly label: string
  readonly participantCount: number
  readonly participantIds: readonly ParticipantId[]
  readonly latestActivityTimestamp: number
}

export type ChatDiscovery = {
  readonly status: ChatDiscoveryStatus
  readonly chats: readonly DiscoveredChat[]
}

export type ChatOption = Pick<DiscoveredChat, "id" | "label">
export type SelectedChat = DiscoveredChat & { readonly backfillPromptEnabled: boolean }

export const defaultChatDiscovery: ChatDiscovery = { status: "unverified", chats: [] }
export const opaqueChatIdPattern = /^messages-chat-[0-9a-f]{32}$/
export const opaqueParticipantIdPattern = /^messages-participant-[0-9a-f]{32}$/

const chatIdSchema = z
  .string()
  .regex(opaqueChatIdPattern, "Chat IDs must be opaque Messages chat identifiers.")
const participantIdSchema = z
  .string()
  .regex(
    opaqueParticipantIdPattern,
    "Participant IDs must be opaque Messages participant identifiers."
  )

const discoveredChatFields = {
  id: chatIdSchema,
  label: z.string().min(1).max(80).refine((value) => !isRawHandleLike(value), {
    message: "Discovered chat labels must not expose raw handles."
  }),
  participantCount: z.number().int().min(1).max(65_535),
  participantIds: z.array(participantIdSchema),
  latestActivityTimestamp: z.number().int().min(0)
} as const

export const discoveredChatSchema = z
  .object(discoveredChatFields)
  .strict()
  .superRefine(refineParticipantCount)

export const selectedChatSchema = z
  .object({ ...discoveredChatFields, backfillPromptEnabled: z.boolean() })
  .strict()
  .superRefine(refineParticipantCount)

const emptyDiscoveryChatsSchema = z.array(discoveredChatSchema).length(0)

export const chatDiscoverySchema = z.discriminatedUnion("status", [
  z.object({ status: z.literal("unverified"), chats: emptyDiscoveryChatsSchema }).strict(),
  z.object({ status: z.literal("loading"), chats: emptyDiscoveryChatsSchema }).strict(),
  z.object({ status: z.literal("ready"), chats: z.array(discoveredChatSchema).min(1) }).strict(),
  z.object({ status: z.literal("empty"), chats: emptyDiscoveryChatsSchema }).strict(),
  z.object({ status: z.literal("permissionDenied"), chats: emptyDiscoveryChatsSchema }).strict(),
  z.object({ status: z.literal("unavailable"), chats: emptyDiscoveryChatsSchema }).strict()
])

export function chatDiscoveryWarning(discovery: ChatDiscovery): string | undefined {
  switch (discovery.status) {
    case "ready":
      return undefined
    case "unverified":
    case "loading":
      return "Native chat discovery is not ready."
    case "empty":
      return "Native chat discovery found no chats."
    case "permissionDenied":
      return "Native chat discovery needs Messages access."
    case "unavailable":
      return "Native chat discovery unavailable."
    default:
      return assertNever(discovery.status)
  }
}

export function selectedChatsAreVerified(
  discovery: ChatDiscovery,
  selectedChats: readonly SelectedChat[]
): boolean {
  switch (discovery.status) {
    case "ready": {
      return selectedChats.every((selectedChat) => {
        const discoveredChat = discovery.chats.find((chat) => chat.id === selectedChat.id)
        return discoveredChat !== undefined && chatMetadataMatches(discoveredChat, selectedChat)
      })
    }
    case "unverified":
    case "loading":
    case "empty":
    case "permissionDenied":
    case "unavailable":
      return false
    default:
      return assertNever(discovery.status)
  }
}

export function reconcileSelectedChats(
  discovery: ChatDiscovery,
  selectedChats: readonly SelectedChat[]
): readonly SelectedChat[] {
  switch (discovery.status) {
    case "ready":
      return selectedChats.map((selectedChat) => {
        const discoveredChat = discovery.chats.find((chat) => chat.id === selectedChat.id)
        if (discoveredChat === undefined) {
          return selectedChat
        }
        return { ...discoveredChat, backfillPromptEnabled: selectedChat.backfillPromptEnabled }
      })
    case "unverified":
    case "loading":
    case "empty":
    case "permissionDenied":
    case "unavailable":
      return selectedChats
    default:
      return assertNever(discovery.status)
  }
}

export function scanSelectedChatMetadata(chats: readonly SelectedChat[]): readonly DiscoveredChat[] {
  return chats.map(({ id, label, participantCount, participantIds, latestActivityTimestamp }) => ({
    id,
    label,
    participantCount,
    participantIds,
    latestActivityTimestamp
  }))
}

export function toggleSelectedChatSelection(
  selectedChats: readonly SelectedChat[],
  discovery: ChatDiscovery,
  chatId: ChatId,
  eventChat: DiscoveredChat | undefined
): readonly SelectedChat[] {
  const existing = selectedChats.find((chat) => chat.id === chatId)
  if (existing !== undefined) {
    return selectedChats.filter((chat) => chat.id !== chatId)
  }
  const discoveredChat = eventChat ?? discovery.chats.find((chat) => chat.id === chatId)
  if (discoveredChat === undefined) {
    return selectedChats
  }
  return [...selectedChats, { ...discoveredChat, backfillPromptEnabled: true }]
}

export function setChatBackfillPromptSelection(
  selectedChats: readonly SelectedChat[],
  chatId: ChatId,
  enabled: boolean
): readonly SelectedChat[] {
  return selectedChats.map((chat) =>
    chat.id === chatId ? { ...chat, backfillPromptEnabled: enabled } : chat
  )
}

function chatMetadataMatches(discoveredChat: DiscoveredChat, selectedChat: SelectedChat): boolean {
  return (
    discoveredChat.label === selectedChat.label &&
    discoveredChat.participantCount === selectedChat.participantCount &&
    discoveredChat.latestActivityTimestamp === selectedChat.latestActivityTimestamp &&
    arraysMatch(discoveredChat.participantIds, selectedChat.participantIds)
  )
}

function arraysMatch(left: readonly string[], right: readonly string[]): boolean {
  return left.length === right.length && left.every((value, index) => value === right[index])
}

function refineParticipantCount(
  chat: { readonly participantCount: number; readonly participantIds: readonly string[] },
  context: z.RefinementCtx
): void {
  if (chat.participantIds.length !== chat.participantCount) {
    context.addIssue({
      code: z.ZodIssueCode.custom,
      path: ["participantIds"],
      message: "Participant IDs must match participant count."
    })
  }
}

export function isRawHandleLike(value: string): boolean {
  const normalized = value.trim().toLowerCase()
  return (
    normalized.includes("@") ||
    normalized.startsWith("tel:") ||
    normalized.startsWith("sms:") ||
    normalized.startsWith("sms;") ||
    normalized.startsWith("imessage:") ||
    normalized.startsWith("imessage;") ||
    isPhoneLike(normalized)
  )
}

function isPhoneLike(value: string): boolean {
  const digitCount = Array.from(value).filter((character) => character >= "0" && character <= "9")
    .length
  if (digitCount < 7) {
    return false
  }
  return Array.from(value).every((character) => "0123456789+-(). \t".includes(character))
}

function assertNever(value: never): never {
  throw new UnhandledChatDiscoveryVariantError(String(value))
}

class UnhandledChatDiscoveryVariantError extends Error {
  readonly renderedValue: string

  constructor(renderedValue: string) {
    super(`Unhandled chat discovery variant: ${renderedValue}`)
    this.name = "UnhandledChatDiscoveryVariantError"
    this.renderedValue = renderedValue
  }
}
