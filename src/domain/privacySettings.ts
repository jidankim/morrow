import type { MorrowDataDeleteReceipt, PrivacySettingsPane } from "../tauriBridge"

export function requireDeleteReceipt(
  receipt: MorrowDataDeleteReceipt | undefined
): MorrowDataDeleteReceipt {
  if (receipt === undefined) {
    throw new Error("Native Morrow data deletion is unavailable.")
  }
  return receipt
}

export function privacyPaneName(pane: PrivacySettingsPane): string {
  switch (pane) {
    case "fullDiskAccess":
      return "Full Disk Access"
    case "calendar":
      return "Calendar access"
    case "reminders":
      return "Reminders access"
    default:
      return assertNeverPrivacyPane(pane)
  }
}

function assertNeverPrivacyPane(value: never): never {
  throw new Error(`Unhandled privacy settings pane: ${String(value)}`)
}
