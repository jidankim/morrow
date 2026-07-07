import { z } from "zod"

const listIntakeItemSchema = z
  .object({
    itemName: z.string().trim().min(1).max(80),
    quantity: z.number().int().min(1).max(999),
    unit: z.string().trim().min(1).max(24).optional(),
    categoryId: z.string().trim().min(1).max(40),
    categoryLabel: z.string().trim().min(1).max(40)
  })
  .strict()

const listIntakeCategorySchema = z
  .object({
    categoryId: z.string().trim().min(1).max(40),
    label: z.string().trim().min(1).max(40)
  })
  .strict()

const listIntakeAggregateGroupSchema = z
  .object({
    profileId: z.string().trim().min(1).max(80),
    profileName: z.string().trim().min(1).max(80),
    outputPolicy: z.string().trim().min(1).max(40),
    localDate: z.string().trim().regex(/^\d{4}-\d{2}-\d{2}$/u),
    chatLabel: z.string().trim().min(1).max(80),
    senderLabel: z.string().trim().min(1).max(80),
    categoryLabel: z.string().trim().min(1).max(40),
    items: z.array(listIntakeItemSchema).min(1).max(20)
  })
  .strict()

const listIntakeProposalSchema = z
  .object({
    proposalId: z.string().trim().min(1).max(120),
    profileName: z.string().trim().min(1).max(80),
    chatLabel: z.string().trim().min(1).max(80),
    senderLabel: z.string().trim().min(1).max(80),
    items: z.array(listIntakeItemSchema).min(1).max(20),
    categories: z.array(listIntakeCategorySchema).min(1).max(20)
  })
  .strict()

export const listIntakeReviewReportSchema = z
  .object({
    generatedAtUnixSeconds: z.number().int().min(0),
    aggregates: z.array(listIntakeAggregateGroupSchema).max(50),
    proposals: z.array(listIntakeProposalSchema).max(50)
  })
  .strict()

const listIntakeDecisionRequestSchema = z.discriminatedUnion("decision", [
  z
    .object({
      decision: z.literal("approveEdited"),
      proposalId: z.string().trim().min(1).max(120),
      items: z.array(listIntakeItemSchema).min(1).max(20)
    })
    .strict(),
  z
    .object({
      decision: z.literal("reject"),
      proposalId: z.string().trim().min(1).max(120)
    })
    .strict()
])

export type ListIntakeItem = z.infer<typeof listIntakeItemSchema>
export type ListIntakeCategory = z.infer<typeof listIntakeCategorySchema>
export type ListIntakeAggregateGroup = z.infer<typeof listIntakeAggregateGroupSchema>
export type ListIntakeProposal = z.infer<typeof listIntakeProposalSchema>
export type ListIntakeReviewReport = z.infer<typeof listIntakeReviewReportSchema>
export type ListIntakeDecisionRequest =
  | {
      readonly decision: "approveEdited"
      readonly proposalId: string
      readonly items: readonly ListIntakeItem[]
    }
  | {
      readonly decision: "reject"
      readonly proposalId: string
    }

export function parseListIntakeReviewReport(value: unknown): ListIntakeReviewReport {
  return listIntakeReviewReportSchema.parse(value)
}

export function parseListIntakeDecisionRequest(value: unknown): ListIntakeDecisionRequest {
  return listIntakeDecisionRequestSchema.parse(value)
}
