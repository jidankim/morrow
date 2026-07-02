import { invoke } from "@tauri-apps/api/core"
import {
  decisionEvidenceLoadRequestSchema,
  decisionEvidenceReportSchema,
  type DecisionEvidenceLoadRequest,
  type DecisionEvidenceReport
} from "./domain/decisionEvidence"

export async function loadDecisionEvidenceInTauri(
  request: DecisionEvidenceLoadRequest
): Promise<DecisionEvidenceReport> {
  const report = await invoke<unknown>("load_decision_evidence", {
    request: decisionEvidenceLoadRequestSchema.parse(request)
  })
  return decisionEvidenceReportSchema.parse(report)
}
