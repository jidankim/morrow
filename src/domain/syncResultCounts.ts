import { z } from "zod"

const syncResultCountSchema = z.number().int().min(0)

export const syncResultCountsSchema = z.object({
  pendingProposalCount: syncResultCountSchema,
  createdCandidateCount: syncResultCountSchema.default(0),
  quietLogCount: syncResultCountSchema.default(0),
  createdExternalProposalCount: syncResultCountSchema.default(0),
  failedExternalProposalCount: syncResultCountSchema.default(0)
})

export type SyncResultCounts = z.infer<typeof syncResultCountsSchema>
export type PartialSyncResultCounts = Pick<SyncResultCounts, "pendingProposalCount"> &
  Partial<Omit<SyncResultCounts, "pendingProposalCount">>

export const emptySyncResultCounts = {
  pendingProposalCount: 0,
  createdCandidateCount: 0,
  quietLogCount: 0,
  createdExternalProposalCount: 0,
  failedExternalProposalCount: 0
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
