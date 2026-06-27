import { z } from "zod"

const morrowDataCleanupPlanSchema = z.object({
  proposedItems: z.union([
    z.literal("completed"),
    z.literal("adapterDeferred"),
    z.literal("skippedByUser")
  ]),
  emptyProposalContainers: z.union([
    z.literal("completed"),
    z.literal("adapterDeferred"),
    z.literal("skippedByUser")
  ]),
  proposedCalendarItemsDeleted: z.number().int().min(0),
  proposedReminderItemsDeleted: z.number().int().min(0)
})

const morrowDataDeleteReceiptSchema = z.object({
  storageSurface: z.literal("morrowStore"),
  databaseDeleted: z.boolean(),
  approvedExternalItemsDeleted: z.literal(false),
  providerOAuthDeleteRequested: z.boolean(),
  providerOAuthDeleted: z.boolean(),
  providerOAuthDeleteFailed: z.boolean().default(false),
  providerOAuthDeleteError: z.string().min(1).optional(),
  cleanupPlan: morrowDataCleanupPlanSchema
})

const crashLogReceiptSchema = z.object({
  stored: z.boolean()
})

const privacySettingsPaneSchema = z.union([
  z.literal("fullDiskAccess"),
  z.literal("calendar"),
  z.literal("reminders")
])

const privacySettingsReceiptSchema = z.object({
  pane: privacySettingsPaneSchema,
  opened: z.boolean()
})

export type MorrowDataDeleteRequest = {
  readonly confirmation: "DELETE MORROW DATA"
  readonly cleanupProposedItems: boolean
  readonly deleteEmptyProposalContainers: boolean
  readonly revokeProviderOAuth: boolean
}

export type MorrowDataDeleteReceipt = z.infer<typeof morrowDataDeleteReceiptSchema>
export type PrivacySettingsPane = z.infer<typeof privacySettingsPaneSchema>
export type PrivacySettingsRequest = {
  readonly pane: PrivacySettingsPane
}
export type PrivacySettingsReceipt = z.infer<typeof privacySettingsReceiptSchema>

export type CrashLogRequest = {
  readonly message: string
}

export type CrashLogReceipt = z.infer<typeof crashLogReceiptSchema>

export function parseMorrowDataDeleteReceipt(value: unknown): MorrowDataDeleteReceipt {
  return morrowDataDeleteReceiptSchema.parse(value)
}

export function parsePrivacySettingsReceipt(value: unknown): PrivacySettingsReceipt {
  return privacySettingsReceiptSchema.parse(value)
}

export function parseCrashLogReceipt(value: unknown): CrashLogReceipt {
  return crashLogReceiptSchema.parse(value)
}
