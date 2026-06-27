export const DELETE_ALL_CONFIRMATION_TEXT = "DELETE MORROW DATA"

export type DeleteAllConfirmation =
  | { readonly ok: true }
  | { readonly ok: false; readonly reason: "confirmationMismatch" }

export type DeleteAllOptions = {
  readonly cleanupProposedItems: boolean
  readonly deleteEmptyProposalContainers: boolean
  readonly revokeProviderOAuth: boolean
}

export function parseDeleteAllConfirmation(value: string): DeleteAllConfirmation {
  if (value === DELETE_ALL_CONFIRMATION_TEXT) {
    return { ok: true }
  }
  return { ok: false, reason: "confirmationMismatch" }
}

export function createDefaultDeleteAllOptions(): DeleteAllOptions {
  return {
    cleanupProposedItems: true,
    deleteEmptyProposalContainers: false,
    revokeProviderOAuth: true
  }
}

export function scrubCrashLogText(text: string): string {
  return text
    .replace(/(excerpt=)"[^"]*"/g, "$1[redacted]")
    .replace(/(prompt=)"[^"]*"/g, "$1[redacted]")
    .replace(/(response=)"[^"]*"/g, "$1[redacted]")
    .replace(/(message=)"[^"]*"/g, "$1[redacted]")
}
