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
import { DEFAULT_LIST_REMINDER_PROFILE, type ListReminderProfile } from "./domain/appConfig"
import { resolveReferenceTimeZonePreference } from "./domain/timeZone"

const chatIdSchema = z
  .string()
  .regex(opaqueChatIdPattern, "Chat IDs must be opaque Messages chat identifiers.")
const messagePreviewVisibleCharLimit = 120
const messagePreviewEllipsis = "..."
const messagePreviewMaxCharLimit = messagePreviewVisibleCharLimit + messagePreviewEllipsis.length
const previewTextSchema = z
  .string()
  .refine((value) => value.length === 0 || value.trim().length > 0, {
    message: "Preview text must be empty or include visible content."
  })
  .refine((value) => Array.from(value).length <= messagePreviewMaxCharLimit, {
    message: "Preview text must not exceed the native capped preview length."
  })
  .refine(
    (value) =>
      Array.from(value).length <= messagePreviewVisibleCharLimit ||
      value.endsWith(messagePreviewEllipsis),
    {
      message: "Preview text over the visible limit must be native-capped with an ellipsis."
    }
  )
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

const messagesChatPreviewRequestSchema = z.object({
  chatIds: z.array(chatIdSchema).min(1)
}).strict()

const messagesChatPreviewRowSchema = z.object({
  chatId: chatIdSchema,
  preview: previewTextSchema
}).strict()

const messagesChatPreviewReportSchema = z.object({
  chats: z.array(messagesChatPreviewRowSchema)
}).strict()

const syncScanListReminderProfileSchema = z
  .object({
    enabled: z.boolean(),
    profileId: z.literal(DEFAULT_LIST_REMINDER_PROFILE.profileId),
    profileVersion: z.literal(DEFAULT_LIST_REMINDER_PROFILE.profileVersion),
    routingMode: z.union([z.literal("explicitOnly"), z.literal("profileBareQuantityLists")]),
    defaultDueMode: z.union([z.literal("explicitOnly"), z.literal("nextLocalDayAtDefaultTime")]),
    defaultDueTime: z.literal(DEFAULT_LIST_REMINDER_PROFILE.defaultDueTime),
    recurrenceMode: z.literal(DEFAULT_LIST_REMINDER_PROFILE.recurrenceMode),
    itemOutputMode: z.literal(DEFAULT_LIST_REMINDER_PROFILE.itemOutputMode)
  })
  .strict() satisfies z.ZodType<ListReminderProfile>

const syncScanRequestSchema = z.object({
  selectedChatIds: z.array(chatIdSchema).min(1),
  selectedChats: z.array(discoveredChatSchema).min(1),
  referenceTimezone: z.string().min(1),
  referenceUnixSeconds: z.number().int().min(0),
  backfillPromptChatIds: z.array(chatIdSchema),
  sourceExcerptsEnabled: z.boolean(),
  feedbackTextSnapshotsEnabled: z.boolean(),
  localDiagnosticsEnabled: z.boolean(),
  localDiagnosticsRetentionDays: z.number().int().min(1).max(365),
  listReminderProfile: syncScanListReminderProfileSchema,
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
  readonly feedbackTextSnapshotsEnabled: boolean
  readonly localDiagnosticsEnabled: boolean
  readonly localDiagnosticsRetentionDays: number
  readonly listReminderProfile: ListReminderProfile
  readonly capPolicy: {
    readonly mode: "refillForPending"
    readonly maxVisible: number
    readonly pendingCount: number
  }
}

export type MessagesDiscoveryReport = z.infer<typeof messagesDiscoveryReportSchema>
export type MessagesChatPreviewRequest = {
  readonly chatIds: readonly ChatId[]
}
export type MessagesChatPreviewReport = {
  readonly chats: readonly {
    readonly chatId: ChatId
    readonly preview: string
  }[]
}
export type SyncScanRequestState = Pick<
  AppShellState,
  "config" | "pendingProposalCount" | "selectedChats"
>

export function currentSystemReferenceTimeZone(): string | undefined {
  return Intl.DateTimeFormat().resolvedOptions().timeZone
}

export function syncScanRequestFromState(
  state: SyncScanRequestState,
  systemTimeZone = currentSystemReferenceTimeZone()
): SyncScanRequest {
  return {
    selectedChatIds: state.selectedChats.map((chat) => chat.id),
    selectedChats: scanSelectedChatMetadata(state.selectedChats),
    referenceTimezone: resolveReferenceTimeZonePreference(
      state.config.referenceTimezone,
      systemTimeZone
    ),
    referenceUnixSeconds: Math.floor(Date.now() / 1000),
    backfillPromptChatIds: state.selectedChats
      .filter((chat) => chat.backfillPromptEnabled)
      .map((chat) => chat.id),
    sourceExcerptsEnabled: state.config.sourceExcerptsEnabled,
    feedbackTextSnapshotsEnabled: state.config.feedbackTextSnapshotsEnabled,
    localDiagnosticsEnabled: state.config.localDiagnosticsEnabled,
    localDiagnosticsRetentionDays: state.config.localDiagnosticsRetentionDays,
    listReminderProfile: state.config.listReminderProfile,
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

export function parseMessagesChatPreviewRequest(value: unknown): MessagesChatPreviewRequest {
  return messagesChatPreviewRequestSchema.parse(value)
}

export function parseMessagesChatPreviewReport(value: unknown): MessagesChatPreviewReport {
  return messagesChatPreviewReportSchema.parse(value)
}
