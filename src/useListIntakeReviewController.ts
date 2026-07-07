import { useCallback, useEffect, useState } from "react"
import type { Route } from "./appRuntime"
import type {
  ListIntakeDecisionRequest,
  ListIntakeReviewReport
} from "./domain/listIntakeReview"
import type { NativeShellBridge } from "./tauriBridge"

export type ListIntakeReviewState =
  | { readonly status: "idle" }
  | { readonly status: "loading" }
  | { readonly status: "empty" }
  | { readonly status: "loaded"; readonly report: ListIntakeReviewReport; readonly notice?: string }
  | { readonly status: "failed"; readonly message: string }

export type ListIntakeReviewController = {
  readonly state: ListIntakeReviewState
  readonly approveEdited: (request: ListIntakeDecisionRequest) => Promise<void>
  readonly reject: (request: ListIntakeDecisionRequest) => Promise<void>
  readonly reload: () => Promise<void>
}

export function useListIntakeReviewController(
  route: Route,
  nativeBridge: NativeShellBridge
): ListIntakeReviewController {
  const [reviewState, setReviewState] = useState<ListIntakeReviewState>({ status: "idle" })

  const loadReview = useCallback(async (notice?: string): Promise<void> => {
    setReviewState({ status: "loading" })
    const report = await nativeBridge.loadListIntakeReview()
    if (report === undefined || (report.aggregates.length === 0 && report.proposals.length === 0)) {
      setReviewState({ status: "empty" })
      return
    }
    setReviewState(loadedState(report, notice))
  }, [nativeBridge])

  useEffect(() => {
    if (route !== "list-intake") {
      return
    }
    let active = true
    setReviewState({ status: "loading" })
    void nativeBridge
      .loadListIntakeReview()
      .then((report) => {
        if (!active) {
          return
        }
        if (report === undefined || (report.aggregates.length === 0 && report.proposals.length === 0)) {
          setReviewState({ status: "empty" })
          return
        }
        setReviewState({ status: "loaded", report })
      })
      .catch((error: unknown) => {
        if (!active) {
          return
        }
        if (!(error instanceof Error) && typeof error !== "string") {
          throw error
        }
        setReviewState({ status: "failed", message: "List intake review data could not be loaded." })
      })
    return () => {
      active = false
    }
  }, [nativeBridge, route])

  const decide = useCallback(async (request: ListIntakeDecisionRequest, notice: string): Promise<void> => {
    try {
      const report = await nativeBridge.decideListIntakeProposal(request)
      if (report === undefined) {
        await loadReview(notice)
        return
      }
      setReviewState(loadedState(withoutDecidedProposal(report, request), notice))
    } catch (error: unknown) {
      if (!(error instanceof Error) && typeof error !== "string") {
        throw error
      }
      setReviewState({ status: "failed", message: "List intake proposal decision could not be saved." })
    }
  }, [loadReview, nativeBridge])

  return {
    approveEdited: (request) => decide(request, "Approved list-intake proposal."),
    reject: (request) => decide(request, "Rejected list-intake proposal."),
    reload: () => loadReview(),
    state: reviewState
  }
}

function withoutDecidedProposal(
  report: ListIntakeReviewReport,
  request: ListIntakeDecisionRequest
): ListIntakeReviewReport {
  return {
    ...report,
    proposals: report.proposals.filter((proposal) => proposal.proposalId !== request.proposalId)
  }
}

function loadedState(
  report: ListIntakeReviewReport,
  notice?: string
): Extract<ListIntakeReviewState, { readonly status: "loaded" }> {
  if (notice === undefined) {
    return { status: "loaded", report }
  }
  return { status: "loaded", report, notice }
}
