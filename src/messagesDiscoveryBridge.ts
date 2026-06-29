import { z } from "zod"
import {
  scanSelectedChatMetadata,
  discoveredChatSchema,
  isRawHandleLike,
  opaqueChatIdPattern,
  opaqueParticipantIdPattern,
  type ChatDiscovery,
  type ChatId,
  type DiscoveredChat
} from "./domain/chatDiscovery"
import type { AppShellState } from "./domain/appShell"

const chatIdSchema = z
  .string()
  .regex(opaqueChatIdPattern, "Chat IDs must be opaque Messages chat identifiers.")
const participantIdSchema = z
  .string()
  .regex(
    opaqueParticipantIdPattern,
    "Participant IDs must be opaque Messages participant identifiers."
  )

const nativeDiscoveredChatSchema = z
  .object({
    chatId: chatIdSchema,
    displayLabel: z.string().min(1).max(80).refine((value) => !isRawHandleLike(value), {
      message: "Discovered chat labels must not expose raw handles."
    }),
    participantCount: z.number().int().min(1).max(65_535),
    participantIds: z.array(participantIdSchema),
    latestActivityTimestamp: z.number().int().min(0)
  })
  .strict()
  .superRefine((chat, context) => {
    if (chat.participantIds.length !== chat.participantCount) {
      context.addIssue({
        code: z.ZodIssueCode.custom,
        path: ["participantIds"],
        message: "Participant IDs must match participant count."
      })
    }
  })

const emptyDiscoveryChatsSchema = z.array(nativeDiscoveredChatSchema).length(0)

const messagesDiscoveryReportSchema = z.discriminatedUnion("status", [
  z.object({ status: z.literal("ready"), chats: z.array(nativeDiscoveredChatSchema).min(1) }).strict(),
  z.object({ status: z.literal("empty"), chats: emptyDiscoveryChatsSchema }).strict(),
  z.object({ status: z.literal("permissionDenied"), chats: emptyDiscoveryChatsSchema }).strict(),
  z.object({ status: z.literal("unavailable"), chats: emptyDiscoveryChatsSchema }).strict()
])

const syncScanRequestSchema = z.object({
  selectedChatIds: z.array(chatIdSchema).min(1),
  selectedChats: z.array(discoveredChatSchema).min(1),
  referenceTimezone: z.string().min(1),
  referenceUnixSeconds: z.number().int().min(0),
  backfillPromptChatIds: z.array(chatIdSchema),
  sourceExcerptsEnabled: z.boolean(),
  capPolicy: z.object({
    mode: z.literal("refillForPending"),
    maxVisible: z.number().int().min(0),
    pendingCount: z.number().int().min(0)
  }).strict()
}).strict()

export type SyncScanRequest = {
  readonly selectedChatIds: readonly ChatId[]
  readonly selectedChats: readonly DiscoveredChat[]
  readonly referenceTimezone: string
  readonly referenceUnixSeconds: number
  readonly backfillPromptChatIds: readonly ChatId[]
  readonly sourceExcerptsEnabled: boolean
  readonly capPolicy: {
    readonly mode: "refillForPending"
    readonly maxVisible: number
    readonly pendingCount: number
  }
}

export type MessagesDiscoveryReport = z.infer<typeof messagesDiscoveryReportSchema>
export type SyncScanRequestState = Pick<
  AppShellState,
  "config" | "pendingProposalCount" | "selectedChats"
>

export function syncScanRequestFromState(state: SyncScanRequestState): SyncScanRequest {
  return {
    selectedChatIds: state.selectedChats.map((chat) => chat.id),
    selectedChats: scanSelectedChatMetadata(state.selectedChats),
    referenceTimezone: state.config.referenceTimezone,
    referenceUnixSeconds: Math.floor(Date.now() / 1000),
    backfillPromptChatIds: state.selectedChats
      .filter((chat) => chat.backfillPromptEnabled)
      .map((chat) => chat.id),
    sourceExcerptsEnabled: state.config.sourceExcerptsEnabled,
    capPolicy: {
      mode: "refillForPending",
      maxVisible: 10,
      pendingCount: state.pendingProposalCount
    }
  }
}

export function chatDiscoveryFromReport(report: MessagesDiscoveryReport | undefined): ChatDiscovery {
  if (report === undefined) {
    return { status: "unavailable", chats: [] }
  }
  switch (report.status) {
    case "ready":
      return {
        status: "ready",
        chats: report.chats.map((chat) => ({
          id: chat.chatId,
          label: chat.displayLabel,
          participantCount: chat.participantCount,
          participantIds: chat.participantIds,
          latestActivityTimestamp: chat.latestActivityTimestamp
        }))
      }
    case "empty":
    case "permissionDenied":
    case "unavailable":
      return { status: report.status, chats: [] }
  }
}

export function parseSyncScanRequest(value: unknown): SyncScanRequest {
  return syncScanRequestSchema.parse(value)
}

export function parseMessagesDiscoveryReport(value: unknown): MessagesDiscoveryReport {
  return messagesDiscoveryReportSchema.parse(value)
}
