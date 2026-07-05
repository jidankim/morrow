import { describe, expect, it } from "vitest"
import { decisionEvidenceReportSchema } from "./decisionEvidence"

describe("decisionEvidenceReportSchema", () => {
  it("normalizes optional source excerpts at the native schema boundary", () => {
    // Given
    const reportWithSourceExcerpt = decisionEvidenceReport({
      sourceExcerpt: "Finish review of the essay by July 25, 2026"
    })
    const reportWithNullSourceExcerpt = decisionEvidenceReport({ sourceExcerpt: null })
    const reportWithoutSourceExcerpt = decisionEvidenceReport({})
    const reportWithEmptySourceExcerpt = decisionEvidenceReport({ sourceExcerpt: "" })

    // When
    const parsedWithSourceExcerpt = decisionEvidenceReportSchema.parse(reportWithSourceExcerpt)
    const parsedWithNullSourceExcerpt =
      decisionEvidenceReportSchema.parse(reportWithNullSourceExcerpt)
    const parsedWithoutSourceExcerpt = decisionEvidenceReportSchema.parse(reportWithoutSourceExcerpt)

    // Then
    expect(parsedWithSourceExcerpt.items[0]?.sourceExcerpt).toBe(
      "Finish review of the essay by July 25, 2026"
    )
    expect(parsedWithNullSourceExcerpt.items[0]?.sourceExcerpt).toBeUndefined()
    expect(parsedWithoutSourceExcerpt.items[0]?.sourceExcerpt).toBeUndefined()
    expect(() => decisionEvidenceReportSchema.parse(reportWithEmptySourceExcerpt)).toThrow()
  })
})

function decisionEvidenceReport({
  sourceExcerpt
}: {
  readonly sourceExcerpt?: string | null | undefined
}) {
  const item =
    sourceExcerpt === undefined
      ? decisionEvidenceItem()
      : decisionEvidenceItem({ sourceExcerpt })
  return {
    items: [item],
    skippedTraceLineCount: 0,
    latestEvalStatus: "needs_review"
  }
}

function decisionEvidenceItem(extraFields: Record<string, string | null> = {}) {
  return {
    subjectType: "quietLog",
    route: "provider",
    reasonCode: "provider_unavailable",
    labelType: "quiet",
    labelValue: "rejected",
    sourceExcerptPolicy: "disabled",
    privacyTier: "safe",
    hasDiagnosticsHashes: false,
    createdAt: 1_783_000_000,
    traceRetention: "traceMissing",
    traceSequence: [
      {
        component: "scan",
        operation: "classify",
        decision: "quiet",
        outcome: "not actionable"
      }
    ],
    ...extraFields
  }
}
