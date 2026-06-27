import { describe, expect, it } from "vitest"
import {
  DELETE_ALL_CONFIRMATION_TEXT,
  createDefaultDeleteAllOptions,
  parseDeleteAllConfirmation,
  scrubCrashLogText
} from "./privacyControls"

describe("privacy controls", () => {
  it("requires exact type-to-confirm text for delete-all", () => {
    expect(parseDeleteAllConfirmation("delete morrow data").ok).toBe(false)
    expect(parseDeleteAllConfirmation("DELETE MORROW DATA ").ok).toBe(false)
    expect(parseDeleteAllConfirmation(DELETE_ALL_CONFIRMATION_TEXT).ok).toBe(true)
  })

  it("defaults proposed cleanup on and empty containers kept", () => {
    const options = createDefaultDeleteAllOptions()

    expect(options.cleanupProposedItems).toBe(true)
    expect(options.deleteEmptyProposalContainers).toBe(false)
    expect(options.revokeProviderOAuth).toBe(true)
  })

  it("scrubs excerpts prompts and responses while preserving candidate ids", () => {
    const scrubbed = scrubCrashLogText(
      "candidate_id=morrow_1234567890abcdef excerpt=\"Meet at my home address\" prompt=\"full prompt\" response=\"full response\""
    )

    expect(scrubbed).toContain("morrow_1234567890abcdef")
    expect(scrubbed).not.toContain("home address")
    expect(scrubbed).not.toContain("full prompt")
    expect(scrubbed).not.toContain("full response")
  })
})
