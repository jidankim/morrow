import { z } from "zod"

export function publicProviderUsageRouteLabel(raw: string): string {
  if (raw === "parser_time" || raw.startsWith("parser_time:")) {
    return "parser_time"
  }
  if (raw === "provider_candidate" || raw === "provider_route" || raw === "candidate_route") {
    return "provider_candidate"
  }
  if (
    raw === "provider_rejection" ||
    raw === "provider_rejected" ||
    raw.startsWith("provider_rejected:") ||
    raw === "provider_unavailable"
  ) {
    return "provider_rejection"
  }
  if (
    raw === "parser_pattern" ||
    raw === "calendar_route" ||
    raw === "quiet_route" ||
    raw === "no_action" ||
    raw.startsWith("parser_provider_route_") ||
    raw.startsWith("parser_pattern:")
  ) {
    return "parser_pattern"
  }
  return "unknown"
}

export const providerUsageWindowKeySchema = z.union([
  z.literal("7d"),
  z.literal("30d"),
  z.literal("90d"),
  z.literal("all")
])

const optionalNativeNumberSchema = z.preprocess(
  (value) => (value === null ? undefined : value),
  z.number().optional()
)

const optionalNativeConfidenceSchema = z.preprocess(
  (value) => (value === null ? undefined : value),
  z.number().min(0).max(1).optional()
)

const providerUsageGroupBaseSchema = z
  .object({
    totalOutcomes: z.number().int().min(0),
    candidateCount: z.number().int().min(0),
    quietCount: z.number().int().min(0),
    quietRate: z.number().min(0).max(1),
    candidateRate: z.number().min(0).max(1),
    averageConfidence: optionalNativeConfidenceSchema
  })
  .strict()

const providerUsagePromptGroupSchema = providerUsageGroupBaseSchema
  .extend({
    promptVersion: z.string().min(1)
  })
  .strict()

const providerUsageModelGroupSchema = providerUsageGroupBaseSchema
  .extend({
    modelId: z.string().min(1),
    promptVersions: z.array(providerUsagePromptGroupSchema)
  })
  .strict()

const providerUsageProviderGroupSchema = providerUsageGroupBaseSchema
  .extend({
    providerId: z.string().min(1),
    models: z.array(providerUsageModelGroupSchema)
  })
  .strict()

const providerUsageRecentOutcomeSchema = z
  .object({
    providerId: z.string().min(1),
    modelId: z.string().min(1),
    promptVersion: z.string().min(1),
    outcomeKind: z.string().min(1),
    routeLabel: z.string().min(1).transform(publicProviderUsageRouteLabel),
    confidence: optionalNativeConfidenceSchema,
    createdAtUnixSeconds: z.number().int().min(0)
  })
  .strict()

const providerUsageWindowSchema = z
  .object({
    key: providerUsageWindowKeySchema,
    startUnixSeconds: optionalNativeNumberSchema,
    endUnixSeconds: z.number().int().min(0)
  })
  .strict()

const providerUsageTotalsSchema = providerUsageGroupBaseSchema.omit({
  averageConfidence: true
})

export const providerUsageReportSchema = z
  .object({
    generatedAtUnixSeconds: z.number().int().min(0),
    window: providerUsageWindowSchema,
    totals: providerUsageTotalsSchema,
    providers: z.array(providerUsageProviderGroupSchema),
    recentOutcomes: z.array(providerUsageRecentOutcomeSchema).max(25)
  })
  .strict()

export const providerUsageLoadRequestSchema = z
  .object({
    windowKey: providerUsageWindowKeySchema.optional()
  })
  .strict()

export type ProviderUsageWindowKey = z.infer<typeof providerUsageWindowKeySchema>
export type ProviderUsageLoadRequest = {
  readonly windowKey?: ProviderUsageWindowKey | undefined
}

export type ProviderUsageTotals = {
  readonly totalOutcomes: number
  readonly candidateCount: number
  readonly quietCount: number
  readonly quietRate: number
  readonly candidateRate: number
}

export type ProviderUsagePromptGroup = ProviderUsageTotals & {
  readonly promptVersion: string
  readonly averageConfidence?: number | undefined
}

export type ProviderUsageModelGroup = ProviderUsageTotals & {
  readonly modelId: string
  readonly promptVersions: readonly ProviderUsagePromptGroup[]
  readonly averageConfidence?: number | undefined
}

export type ProviderUsageProviderGroup = ProviderUsageTotals & {
  readonly providerId: string
  readonly models: readonly ProviderUsageModelGroup[]
  readonly averageConfidence?: number | undefined
}

export type ProviderUsageWindow = {
  readonly key: ProviderUsageWindowKey
  readonly startUnixSeconds?: number | undefined
  readonly endUnixSeconds: number
}

export type ProviderUsageRecentOutcome = {
  readonly providerId: string
  readonly modelId: string
  readonly promptVersion: string
  readonly outcomeKind: string
  readonly routeLabel: string
  readonly confidence?: number | undefined
  readonly createdAtUnixSeconds: number
}

export type ProviderUsageReport = {
  readonly generatedAtUnixSeconds: number
  readonly window: ProviderUsageWindow
  readonly totals: ProviderUsageTotals
  readonly providers: readonly ProviderUsageProviderGroup[]
  readonly recentOutcomes: readonly ProviderUsageRecentOutcome[]
}

export function parseProviderUsageReport(value: unknown): ProviderUsageReport {
  return providerUsageReportSchema.parse(value)
}

export function parseProviderUsageLoadRequest(value: unknown): ProviderUsageLoadRequest {
  return providerUsageLoadRequestSchema.parse(value ?? {})
}
