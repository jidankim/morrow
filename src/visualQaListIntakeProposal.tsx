import { useState } from "react"
import type { ListIntakeDecisionRequest, ListIntakeReviewReport } from "./domain/listIntakeReview"
import { ListIntakeReviewView } from "./ListIntakeReviewView"
import { visualListIntakeReport } from "./visualQaListIntakeData"

export function ProposalReviewFixture(): JSX.Element {
  const [report, setReport] = useState<ListIntakeReviewReport>({ ...visualListIntakeReport, aggregates: [] })
  const [notice, setNotice] = useState<string | undefined>(undefined)
  const decide = async (request: ListIntakeDecisionRequest): Promise<void> => {
    setReport({
      ...report,
      proposals: report.proposals.filter((proposal) => proposal.proposalId !== request.proposalId)
    })
    setNotice(
      request.decision === "approveEdited"
        ? "Approved list-intake proposal."
        : "Rejected list-intake proposal."
    )
  }
  const reviewState =
    notice === undefined
      ? { status: "loaded" as const, report }
      : { status: "loaded" as const, report, notice }

  return (
    <ListIntakeReviewView
      state={reviewState}
      onApproveEdited={decide}
      onReject={decide}
      onReload={noopAsync}
    />
  )
}

async function noopAsync(): Promise<void> {
  return undefined
}
