import { z } from "zod"

const syncResultCountSchema = z.number().int().min(0)
export const latestEvalStatusSchema = z
  .union([
    z.literal("never_run"),
    z.literal("passed"),
    z.literal("needs_review"),
    z.literal("failed")
  ])
  .default("never_run")

export const syncResultCountsSchema = z.object({
  pendingProposalCount: syncResultCountSchema,
  createdCandidateCount: syncResultCountSchema.default(0),
  quietLogCount: syncResultCountSchema.default(0),
  createdExternalProposalCount: syncResultCountSchema.default(0),
  failedExternalProposalCount: syncResultCountSchema.default(0),
  feedbackLabelCount: syncResultCountSchema.default(0),
  featureSnapshotCount: syncResultCountSchema.default(0),
  latestEvalStatus: latestEvalStatusSchema
})

export type SyncResultCounts = z.infer<typeof syncResultCountsSchema>
export type PartialSyncResultCounts = Pick<SyncResultCounts, "pendingProposalCount"> &
  Partial<Omit<SyncResultCounts, "pendingProposalCount">>

export const emptySyncResultCounts = {
  pendingProposalCount: 0,
  createdCandidateCount: 0,
  quietLogCount: 0,
  createdExternalProposalCount: 0,
  failedExternalProposalCount: 0,
  feedbackLabelCount: 0,
  featureSnapshotCount: 0,
  latestEvalStatus: "never_run"
} as const satisfies SyncResultCounts

export function syncResultCountsFrom(
  counts: PartialSyncResultCounts | undefined
): SyncResultCounts {
  return syncResultCountsSchema.parse(counts ?? emptySyncResultCounts)
}

export function formatSyncResultEvidence(counts: SyncResultCounts): string {
  return (
    `Candidates ${counts.createdCandidateCount} · Quiet logs ${counts.quietLogCount} · ` +
    `External proposals ${counts.createdExternalProposalCount} created / ${counts.failedExternalProposalCount} failed`
  )
}

export function formatLatestEvalStatus(status: SyncResultCounts["latestEvalStatus"]): string {
  switch (status) {
    case "never_run":
      return "Never run"
    case "passed":
      return "Passed"
    case "needs_review":
      return "Needs review"
    case "failed":
      return "Failed"
    default:
      return assertNever(status)
  }
}

function assertNever(value: never): never {
  throw new Error(`Unhandled eval status: ${String(value)}`)
}
