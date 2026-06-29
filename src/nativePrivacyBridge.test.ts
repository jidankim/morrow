import { describe, expect, it } from "vitest"
import { parseMorrowDataDeleteReceipt } from "./nativePrivacyBridge"

describe("parseMorrowDataDeleteReceipt", () => {
  const receiptWithoutDiagnosticsCleanup = {
    storageSurface: "morrowStore",
    databaseDeleted: true,
    approvedExternalItemsDeleted: false,
    providerOAuthDeleteRequested: true,
    providerOAuthDeleted: true,
    providerOAuthDeleteFailed: false,
    cleanupPlan: {
      proposedItems: "completed",
      emptyProposalContainers: "skippedByUser",
      proposedCalendarItemsDeleted: 0,
      proposedReminderItemsDeleted: 0
    }
  } as const

  it("parses diagnostics cleanup from native delete-all receipts", () => {
    const receipt = parseMorrowDataDeleteReceipt({
      ...receiptWithoutDiagnosticsCleanup,
      diagnosticsArtifactsDeleted: true
    })

    expect(receipt.diagnosticsArtifactsDeleted).toBe(true)
  })

  it("defaults omitted diagnostics cleanup to false for older delete-all receipts", () => {
    const receipt = parseMorrowDataDeleteReceipt(receiptWithoutDiagnosticsCleanup)

    expect(receipt.diagnosticsArtifactsDeleted).toBe(false)
  })

  it("preserves explicit false diagnostics cleanup from native delete-all receipts", () => {
    const receipt = parseMorrowDataDeleteReceipt({
      ...receiptWithoutDiagnosticsCleanup,
      diagnosticsArtifactsDeleted: false
    })

    expect(receipt.diagnosticsArtifactsDeleted).toBe(false)
  })
})
