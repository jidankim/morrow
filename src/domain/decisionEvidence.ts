import { z } from "zod"
import { latestEvalStatusSchema } from "./syncResultCounts"

const optionalNativeStringSchema = z
  .union([z.string().min(1), z.null()])
  .optional()
  .transform((value) => value ?? undefined)

const decisionEvidenceSubjectTypeSchema = z.union([
  z.literal("candidate"),
  z.literal("quietLog")
])

const decisionEvidenceTraceRetentionSchema = z.union([
  z.literal("retained"),
  z.literal("notRetained"),
  z.literal("diagnosticsMissing"),
  z.literal("traceMissing")
])

const decisionTraceStepSchema = z
  .object({
    component: z.string().min(1),
    operation: z.string().min(1),
    decision: optionalNativeStringSchema,
    outcome: z.string().min(1)
  })
  .strict()

const decisionEvidenceItemSchema = z
  .object({
    subjectType: decisionEvidenceSubjectTypeSchema,
    candidateId: optionalNativeStringSchema,
    candidateState: optionalNativeStringSchema,
    candidateKind: optionalNativeStringSchema,
    route: optionalNativeStringSchema,
    reasonCode: optionalNativeStringSchema,
    providerDiagnostic: optionalNativeStringSchema,
    confidenceMillis: z.union([z.number().int().min(0), z.null()]).optional().transform((value) => value ?? undefined),
    labelType: z.string().min(1),
    labelValue: z.string().min(1),
    sourceExcerptPolicy: z.string().min(1),
    privacyTier: z.string().min(1),
    sourceExcerpt: optionalNativeStringSchema,
    hasDiagnosticsHashes: z.boolean(),
    createdAt: z.number().int(),
    traceRetention: decisionEvidenceTraceRetentionSchema,
    traceSequence: z.array(decisionTraceStepSchema)
  })
  .strict()

export const decisionEvidenceReportSchema = z
  .object({
    items: z.array(decisionEvidenceItemSchema),
    skippedTraceLineCount: z.number().int().min(0),
    latestEvalStatus: latestEvalStatusSchema
  })
  .strict()

export const decisionEvidenceLoadRequestSchema = z
  .object({
    createdCandidateIds: z.array(z.string().min(1)),
    limit: z.number().int().min(1).max(50)
  })
  .strict()

export type DecisionTraceStep = {
  readonly component: string
  readonly operation: string
  readonly decision?: string | undefined
  readonly outcome: string
}

export type DecisionEvidenceItem = {
  readonly subjectType: "candidate" | "quietLog"
  readonly candidateId?: string | undefined
  readonly candidateState?: string | undefined
  readonly candidateKind?: string | undefined
  readonly route?: string | undefined
  readonly reasonCode?: string | undefined
  readonly providerDiagnostic?: string | undefined
  readonly confidenceMillis?: number | undefined
  readonly labelType: string
  readonly labelValue: string
  readonly sourceExcerptPolicy: string
  readonly privacyTier: string
  readonly sourceExcerpt?: string | undefined
  readonly hasDiagnosticsHashes: boolean
  readonly createdAt: number
  readonly traceRetention: "retained" | "notRetained" | "diagnosticsMissing" | "traceMissing"
  readonly traceSequence: readonly DecisionTraceStep[]
}

export type DecisionEvidenceReport = {
  readonly items: readonly DecisionEvidenceItem[]
  readonly skippedTraceLineCount: number
  readonly latestEvalStatus: "never_run" | "passed" | "needs_review" | "failed"
}
export type DecisionEvidenceLoadRequest = {
  readonly createdCandidateIds: readonly string[]
  readonly limit: number
}

export const emptyDecisionEvidenceReport = {
  items: [],
  skippedTraceLineCount: 0,
  latestEvalStatus: "never_run"
} as const satisfies DecisionEvidenceReport
