import { z } from "zod"
import { opaqueChatIdPattern } from "./chatDiscovery"
import {
  CATEGORY_ID_PATTERN,
  DEFAULT_AGGREGATION,
  DEFAULT_DIGEST_REMINDER,
  DEFAULT_GROUPING,
  DEFAULT_LEGACY_LIST_REMINDER_PROFILE,
  DEFAULT_QUANTITY_LIST_BOUNDS,
  DEFAULT_THRESHOLDS,
  MAX_VISIBLE_CATEGORY_KEYWORD_LENGTH,
  MAX_VISIBLE_CATEGORY_NAME_LENGTH,
  MAX_VISIBLE_EXAMPLE_LENGTH,
  MAX_VISIBLE_PROFILE_NAME_LENGTH,
  PROFILE_ID_PATTERN,
  createListIntakeProfile,
  type LegacyListReminderProfile,
  type ListIntakeProfile
} from "./listIntakeProfileModel"

const visibleTextSchema = (field: string, maxVisibleLength: number) =>
  z
    .string()
    .transform((value) => value.trim())
    .refine((value) => visibleLength(value) >= 1, {
      message: `${field} must include visible content.`
    })
    .refine((value) => visibleLength(value) <= maxVisibleLength, {
      message: `${field} is too long.`
    })

const categoryRuleSchema = z
  .object({
    categoryId: z.string().regex(CATEGORY_ID_PATTERN),
    displayName: visibleTextSchema("Category display name", MAX_VISIBLE_CATEGORY_NAME_LENGTH),
    keywords: z.array(visibleTextSchema("Category keyword", MAX_VISIBLE_CATEGORY_KEYWORD_LENGTH)).max(20)
  })
  .strict()

const chatScopeSchema = z.discriminatedUnion("mode", [
  z.object({ mode: z.literal("allSelectedChats") }).strict(),
  z
    .object({
      mode: z.literal("selectedChatIds"),
      selectedChatIds: z.array(z.string().regex(opaqueChatIdPattern)).min(1).max(50)
    })
    .strict()
])

const digestReminderSchema = z
  .object({
    dueTimeLocal: z.literal(DEFAULT_DIGEST_REMINDER.dueTimeLocal),
    dateOffsetDays: z.literal(DEFAULT_DIGEST_REMINDER.dateOffsetDays),
    outputPolicyVersion: z.literal(DEFAULT_DIGEST_REMINDER.outputPolicyVersion)
  })
  .strict()

const listIntakeProfileInputSchema = z
  .object({
    enabled: z.boolean().default(false),
    profileId: z.string().regex(PROFILE_ID_PATTERN),
    name: visibleTextSchema("Profile name", MAX_VISIBLE_PROFILE_NAME_LENGTH),
    profileVersion: z.literal("list-intake-v2"),
    kind: z.literal("quantityList"),
    extractionMode: z.literal("providerConstrained"),
    providerPromptVersion: z.literal("list-intake-v1").default("list-intake-v1"),
    positiveExamples: z.array(visibleTextSchema("Positive example", MAX_VISIBLE_EXAMPLE_LENGTH)).min(1).max(20),
    negativeExamples: z.array(visibleTextSchema("Negative example", MAX_VISIBLE_EXAMPLE_LENGTH)).max(20).default([]),
    categoryRules: z.array(categoryRuleSchema).max(20).default([]),
    examplesHash: z.string().optional(),
    aggregation: z
      .object({
        window: z.literal(DEFAULT_AGGREGATION.window),
        timezoneSource: z.literal(DEFAULT_AGGREGATION.timezoneSource)
      })
      .strict()
      .default(DEFAULT_AGGREGATION),
    chatScope: chatScopeSchema.default({ mode: "allSelectedChats" }),
    grouping: z
      .object({
        chat: z.literal(true),
        sender: z.union([z.literal("off"), z.literal("displayAlias")]).default("off")
      })
      .strict()
      .default(DEFAULT_GROUPING),
    captureFromScheduledMessages: z.boolean().default(false),
    outputPolicy: z.union([z.literal("aggregateOnly"), z.literal("dailyDigestReminder")]).default("aggregateOnly"),
    digestReminder: digestReminderSchema.optional(),
    quantityListBounds: z
      .object({
        minItems: z.literal(DEFAULT_QUANTITY_LIST_BOUNDS.minItems),
        maxItems: z.literal(DEFAULT_QUANTITY_LIST_BOUNDS.maxItems),
        minQuantity: z.literal(DEFAULT_QUANTITY_LIST_BOUNDS.minQuantity),
        maxQuantity: z.literal(DEFAULT_QUANTITY_LIST_BOUNDS.maxQuantity),
        maxItemNameVisibleChars: z.literal(DEFAULT_QUANTITY_LIST_BOUNDS.maxItemNameVisibleChars),
        maxUnitVisibleChars: z.literal(DEFAULT_QUANTITY_LIST_BOUNDS.maxUnitVisibleChars),
        uncategorizedCategoryId: z.literal(DEFAULT_QUANTITY_LIST_BOUNDS.uncategorizedCategoryId)
      })
      .strict()
      .default(DEFAULT_QUANTITY_LIST_BOUNDS),
    thresholds: z
      .object({
        autoAggregateThresholdMillis: z.literal(DEFAULT_THRESHOLDS.autoAggregateThresholdMillis),
        reviewThresholdMillis: z.literal(DEFAULT_THRESHOLDS.reviewThresholdMillis)
      })
      .strict()
      .default(DEFAULT_THRESHOLDS),
    migrationState: z.literal("needsReviewFromListReminderV1").optional()
  })
  .strict()
  .superRefine((profile, context) => {
    const categoryIds = new Set<string>()
    for (const category of profile.categoryRules) {
      if (categoryIds.has(category.categoryId)) {
        context.addIssue({
          code: z.ZodIssueCode.custom,
          path: ["categoryRules"],
          message: "Category IDs must be unique."
        })
      }
      categoryIds.add(category.categoryId)
    }
    if (profile.outputPolicy === "dailyDigestReminder" && profile.digestReminder === undefined) {
      context.addIssue({
        code: z.ZodIssueCode.custom,
        path: ["digestReminder"],
        message: "Daily digest profiles require the v2 digest reminder settings."
      })
    }
    if (profile.outputPolicy === "aggregateOnly" && profile.digestReminder !== undefined) {
      context.addIssue({
        code: z.ZodIssueCode.custom,
        path: ["digestReminder"],
        message: "Aggregate-only profiles must not carry digest reminder settings."
      })
    }
  })
  .transform((profile) => createListIntakeProfile(profile))

export const listIntakeProfilesSchema = z
  .array(listIntakeProfileInputSchema)
  .max(10)
  .superRefine((profiles, context) => {
    const profileIds = new Set<string>()
    const names = new Set<string>()
    for (const [index, profile] of profiles.entries()) {
      const normalizedName = profile.name.toLocaleLowerCase()
      if (profileIds.has(profile.profileId)) {
        context.addIssue({
          code: z.ZodIssueCode.custom,
          path: [index, "profileId"],
          message: "List-intake profile IDs must be unique."
        })
      }
      if (names.has(normalizedName)) {
        context.addIssue({
          code: z.ZodIssueCode.custom,
          path: [index, "name"],
          message: "List-intake profile names must be unique."
        })
      }
      profileIds.add(profile.profileId)
      names.add(normalizedName)
    }
  })

export const legacyListReminderProfileSchema = z
  .object({
    enabled: z.boolean().default(DEFAULT_LEGACY_LIST_REMINDER_PROFILE.enabled),
    profileId: z
      .literal(DEFAULT_LEGACY_LIST_REMINDER_PROFILE.profileId)
      .default(DEFAULT_LEGACY_LIST_REMINDER_PROFILE.profileId),
    profileVersion: z
      .literal(DEFAULT_LEGACY_LIST_REMINDER_PROFILE.profileVersion)
      .default(DEFAULT_LEGACY_LIST_REMINDER_PROFILE.profileVersion),
    routingMode: z
      .union([z.literal("explicitOnly"), z.literal("profileBareQuantityLists")])
      .default(DEFAULT_LEGACY_LIST_REMINDER_PROFILE.routingMode),
    defaultDueMode: z
      .union([z.literal("explicitOnly"), z.literal("nextLocalDayAtDefaultTime")])
      .default(DEFAULT_LEGACY_LIST_REMINDER_PROFILE.defaultDueMode),
    defaultDueTime: z
      .literal(DEFAULT_LEGACY_LIST_REMINDER_PROFILE.defaultDueTime)
      .default(DEFAULT_LEGACY_LIST_REMINDER_PROFILE.defaultDueTime),
    recurrenceMode: z
      .literal(DEFAULT_LEGACY_LIST_REMINDER_PROFILE.recurrenceMode)
      .default(DEFAULT_LEGACY_LIST_REMINDER_PROFILE.recurrenceMode),
    itemOutputMode: z
      .literal(DEFAULT_LEGACY_LIST_REMINDER_PROFILE.itemOutputMode)
      .default(DEFAULT_LEGACY_LIST_REMINDER_PROFILE.itemOutputMode)
  })
  .strict()

export function migrateLegacyListReminderProfile(
  legacyProfile: LegacyListReminderProfile | undefined
): readonly ListIntakeProfile[] {
  if (legacyProfile === undefined || !legacyProfile.enabled) {
    return []
  }
  return [
    createListIntakeProfile({
      enabled: false,
      profileId: "list-intake-legacy-list-reminders",
      name: "Migrated quantity list",
      profileVersion: "list-intake-v2",
      kind: "quantityList",
      extractionMode: "providerConstrained",
      providerPromptVersion: "list-intake-v1",
      positiveExamples: ["2 anchovies, 3 salmon"],
      negativeExamples: [],
      categoryRules: [],
      aggregation: DEFAULT_AGGREGATION,
      chatScope: { mode: "allSelectedChats" },
      grouping: DEFAULT_GROUPING,
      captureFromScheduledMessages: false,
      outputPolicy: "aggregateOnly",
      quantityListBounds: DEFAULT_QUANTITY_LIST_BOUNDS,
      thresholds: DEFAULT_THRESHOLDS,
      migrationState: "needsReviewFromListReminderV1"
    })
  ]
}

function visibleLength(value: string): number {
  return Array.from(value.trim()).length
}

export type {
  ListIntakeCategoryRule,
  ListIntakeChatScope,
  ListIntakeDigestReminder,
  ListIntakeOutputPolicy,
  ListIntakeProfile
} from "./listIntakeProfileModel"
export {
  DEFAULT_LEGACY_LIST_REMINDER_PROFILE,
  DEFAULT_LIST_INTAKE_PROFILES,
  createListIntakeExamplesHash,
  createListIntakeProfile
} from "./listIntakeProfileModel"
