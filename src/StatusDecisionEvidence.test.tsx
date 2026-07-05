import { render, screen } from "@testing-library/react"
import { describe, expect, it } from "vitest"
import { StatusDecisionEvidence } from "./StatusDecisionEvidence"
import type { DecisionEvidenceItem } from "./domain/decisionEvidence"

describe("StatusDecisionEvidence", () => {
  it("renders a candidate source excerpt with the row evidence", () => {
    // Given
    const item = {
      subjectType: "candidate",
      candidateId: "morrow_8c4997c018681aed",
      candidateState: "draft",
      candidateKind: "calendar_event",
      route: "provider",
      reasonCode: "accepted_for_calendar",
      confidenceMillis: 842,
      sourceExcerpt: "Finish review of the essay by July 25, 2026",
      labelType: "candidate",
      labelValue: "created",
      sourceExcerptPolicy: "include",
      privacyTier: "local_private",
      hasDiagnosticsHashes: true,
      createdAt: 1_783_000_000,
      traceRetention: "retained",
      traceSequence: [
        {
          component: "scan",
          operation: "classify",
          decision: "candidate",
          outcome: "accepted"
        }
      ]
    } satisfies DecisionEvidenceItem

    // When
    render(
      <StatusDecisionEvidence
        items={[item]}
        latestEvalStatus="needs_review"
        loading={false}
      />
    )

    // Then
    expect(screen.getByText("Message")).toBeInTheDocument()
    expect(screen.getByText("Finish review of the essay by July 25, 2026")).toBeInTheDocument()
    expect(screen.getByLabelText("Candidate ID morrow_8c4997c018681aed")).toBeInTheDocument()
  })

  it("renders redacted provider candidates with useful non-sensitive context", () => {
    // Given
    const item = {
      subjectType: "candidate",
      candidateId: "morrow_8c4997c018681aed",
      candidateState: "draft",
      candidateKind: "task_reminder",
      route: "provider_candidate",
      reasonCode: "confidence_meets_threshold",
      confidenceMillis: 820,
      sourceExcerpt: "Source excerpt hidden by settings.",
      labelType: "detection_route",
      labelValue: "provider_candidate",
      sourceExcerptPolicy: "disabled",
      privacyTier: "local_private",
      hasDiagnosticsHashes: true,
      createdAt: 1_783_000_000,
      traceRetention: "notRetained",
      traceSequence: []
    } satisfies DecisionEvidenceItem

    // When
    render(
      <StatusDecisionEvidence
        items={[item]}
        latestEvalStatus="never_run"
        loading={false}
      />
    )

    // Then
    expect(screen.getByText("Reminder")).toBeInTheDocument()
    expect(screen.getByText("Source")).toBeInTheDocument()
    expect(screen.getByText("Hidden by privacy settings.")).toBeInTheDocument()
    expect(screen.queryByText("Message")).not.toBeInTheDocument()
    expect(screen.queryByText("Source excerpt hidden by settings.")).not.toBeInTheDocument()
    expect(screen.queryByText("Sequence unavailable.")).not.toBeInTheDocument()
  })
})
