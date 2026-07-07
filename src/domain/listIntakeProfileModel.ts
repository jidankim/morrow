import type { ChatId } from "./chatDiscovery"

export type ListIntakeProfileVersion = "list-intake-v2"
export type ListIntakeProfileKind = "quantityList"
export type ListIntakeExtractionMode = "providerConstrained"
export type ListIntakeOutputPolicy = "aggregateOnly" | "dailyDigestReminder"
export type ListIntakeProviderPromptVersion = "list-intake-v1"
export type ListIntakeDigestOutputPolicyVersion = "list-intake-digest-v1"
export type ListIntakeMigrationState = "needsReviewFromListReminderV1"

export type ListIntakeCategoryRule = {
  readonly categoryId: string
  readonly displayName: string
  readonly keywords: readonly string[]
}

export type ListIntakeChatScope =
  | { readonly mode: "allSelectedChats" }
  | { readonly mode: "selectedChatIds"; readonly selectedChatIds: readonly ChatId[] }

export type ListIntakeDigestReminder = {
  readonly dueTimeLocal: "09:00"
  readonly dateOffsetDays: 1
  readonly outputPolicyVersion: ListIntakeDigestOutputPolicyVersion
}

export type ListIntakeProfile = {
  readonly enabled: boolean
  readonly profileId: string
  readonly name: string
  readonly profileVersion: ListIntakeProfileVersion
  readonly kind: ListIntakeProfileKind
  readonly extractionMode: ListIntakeExtractionMode
  readonly providerPromptVersion: ListIntakeProviderPromptVersion
  readonly positiveExamples: readonly string[]
  readonly negativeExamples: readonly string[]
  readonly categoryRules: readonly ListIntakeCategoryRule[]
  readonly examplesHash: string
  readonly aggregation: typeof DEFAULT_AGGREGATION
  readonly chatScope: ListIntakeChatScope
  readonly grouping: {
    readonly chat: true
    readonly sender: "off" | "displayAlias"
  }
  readonly captureFromScheduledMessages: boolean
  readonly outputPolicy: ListIntakeOutputPolicy
  readonly digestReminder?: ListIntakeDigestReminder | undefined
  readonly quantityListBounds: typeof DEFAULT_QUANTITY_LIST_BOUNDS
  readonly thresholds: typeof DEFAULT_THRESHOLDS
  readonly migrationState?: ListIntakeMigrationState | undefined
}

export type LegacyListReminderProfile = {
  readonly enabled: boolean
  readonly profileId: "list-reminders"
  readonly profileVersion: "list-reminders-v1"
  readonly routingMode: "explicitOnly" | "profileBareQuantityLists"
  readonly defaultDueMode: "explicitOnly" | "nextLocalDayAtDefaultTime"
  readonly defaultDueTime: "23:59"
  readonly recurrenceMode: "none"
  readonly itemOutputMode: "singleReminderTitle"
}

export type ListIntakeProfileWithoutHash = Omit<ListIntakeProfile, "examplesHash">

export const MAX_VISIBLE_PROFILE_NAME_LENGTH = 48
export const MAX_VISIBLE_EXAMPLE_LENGTH = 500
export const MAX_VISIBLE_CATEGORY_NAME_LENGTH = 32
export const MAX_VISIBLE_CATEGORY_KEYWORD_LENGTH = 48
export const PROFILE_ID_PATTERN = /^list-intake-[a-z0-9-]{8,48}$/
export const CATEGORY_ID_PATTERN = /^[a-z0-9][a-z0-9-]{0,47}$/

export const DEFAULT_AGGREGATION = {
  window: "localDay",
  timezoneSource: "referenceTimezone"
} as const
export const DEFAULT_GROUPING = { chat: true, sender: "off" } as const
export const DEFAULT_QUANTITY_LIST_BOUNDS = {
  minItems: 1,
  maxItems: 20,
  minQuantity: 1,
  maxQuantity: 999,
  maxItemNameVisibleChars: 80,
  maxUnitVisibleChars: 24,
  uncategorizedCategoryId: "uncategorized"
} as const
export const DEFAULT_THRESHOLDS = {
  autoAggregateThresholdMillis: 850,
  reviewThresholdMillis: 550
} as const
export const DEFAULT_DIGEST_REMINDER = {
  dueTimeLocal: "09:00",
  dateOffsetDays: 1,
  outputPolicyVersion: "list-intake-digest-v1"
} as const

export const DEFAULT_LIST_INTAKE_PROFILES: readonly ListIntakeProfile[] = []

export const DEFAULT_LEGACY_LIST_REMINDER_PROFILE = {
  enabled: false,
  profileId: "list-reminders",
  profileVersion: "list-reminders-v1",
  routingMode: "explicitOnly",
  defaultDueMode: "explicitOnly",
  defaultDueTime: "23:59",
  recurrenceMode: "none",
  itemOutputMode: "singleReminderTitle"
} as const satisfies LegacyListReminderProfile

export function createListIntakeProfile(profile: ListIntakeProfileWithoutHash): ListIntakeProfile {
  return {
    ...profile,
    examplesHash: createListIntakeExamplesHash(profile)
  }
}

export function createListIntakeExamplesHash(profile: ListIntakeProfileWithoutHash): string {
  const identity = JSON.stringify({
    profileVersion: profile.profileVersion,
    providerPromptVersion: profile.providerPromptVersion,
    positiveExamples: profile.positiveExamples,
    negativeExamples: profile.negativeExamples,
    categoryRules: profile.categoryRules,
    chatScope: profile.chatScope,
    thresholds: profile.thresholds,
    outputPolicy: profile.outputPolicy,
    digestReminder: profile.digestReminder,
    quantityListBounds: profile.quantityListBounds
  })
  return `list-intake-${fnv1a32(identity)}`
}

function fnv1a32(input: string): string {
  let hash = 0x811c9dc5
  for (const char of input) {
    const codePoint = char.codePointAt(0)
    if (codePoint !== undefined) {
      hash = Math.imul(hash ^ codePoint, 0x01000193) >>> 0
    }
  }
  return hash.toString(16).padStart(8, "0")
}
