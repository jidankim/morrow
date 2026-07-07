import { invoke } from "@tauri-apps/api/core"
import {
  parseListIntakeDecisionRequest,
  parseListIntakeReviewReport,
  type ListIntakeDecisionRequest,
  type ListIntakeReviewReport
} from "./domain/listIntakeReview"

class ListIntakeReviewLoadError extends Error {
  constructor() {
    super("List intake review data could not be loaded.")
    this.name = "ListIntakeReviewLoadError"
  }
}

class ListIntakeDecisionError extends Error {
  constructor() {
    super("List intake proposal decision could not be saved.")
    this.name = "ListIntakeDecisionError"
  }
}

export async function loadListIntakeReviewInTauri(): Promise<ListIntakeReviewReport> {
  try {
    const report = await invoke<unknown>("load_list_intake_review")
    return parseListIntakeReviewReport(report)
  } catch (error) {
    if (error instanceof Error) {
      throw new ListIntakeReviewLoadError()
    }
    throw new ListIntakeReviewLoadError()
  }
}

export async function decideListIntakeProposalInTauri(
  request: ListIntakeDecisionRequest
): Promise<ListIntakeReviewReport> {
  const parsedRequest = parseListIntakeDecisionRequest(request)
  try {
    const report = await invoke<unknown>("decide_list_intake_proposal", { request: parsedRequest })
    return parseListIntakeReviewReport(report)
  } catch (error) {
    if (error instanceof Error) {
      throw new ListIntakeDecisionError()
    }
    throw new ListIntakeDecisionError()
  }
}
