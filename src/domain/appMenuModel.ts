import {
  chatDiscoveryWarning,
  selectedChatsAreVerified
} from "./chatDiscovery"
import type { AppMode, AppShellState, ProviderCredentialStatus } from "./appShell"
import { formatSyncResultEvidence } from "./syncResultCounts"

export type MenuStatusKind = "setup-needed" | AppMode

export type MenuModel = {
  readonly statusKind: MenuStatusKind
  readonly statusLabel: "Setup needed" | "Ready" | "Sync Now disabled" | "Error"
  readonly detail: string
  readonly pauseResumeLabel: "Disable Sync Now" | "Enable Sync Now"
  readonly syncNowEnabled: boolean
  readonly pendingProposalLabel: string
  readonly syncResultLabel: string
}

export function getMenuModel(state: AppShellState): MenuModel {
  const onboardingComplete = isOnboardingComplete(state)
  const pendingProposalLabel = formatPendingProposalCount(state.pendingProposalCount)
  const syncResultLabel = formatSyncResultEvidence(state)
  switch (state.mode) {
    case "scanning":
      if (!onboardingComplete) {
        return {
          statusKind: "setup-needed",
          statusLabel: "Setup needed",
          detail: "Complete setup and choose chats before Sync Now can scan.",
          pauseResumeLabel: "Disable Sync Now",
          syncNowEnabled: false,
          pendingProposalLabel,
          syncResultLabel
        }
      }
      return {
        statusKind: "scanning",
        statusLabel: "Ready",
        detail: "Ready. Use Sync Now to reconcile calendars and scan selected chats.",
        pauseResumeLabel: "Disable Sync Now",
        syncNowEnabled: true,
        pendingProposalLabel,
        syncResultLabel
      }
    case "paused":
      return {
        statusKind: "paused",
        statusLabel: "Sync Now disabled",
        detail: "Sync Now is disabled on this Mac.",
        pauseResumeLabel: "Enable Sync Now",
        syncNowEnabled: false,
        pendingProposalLabel,
        syncResultLabel
      }
    case "error":
      return {
        statusKind: "error",
        statusLabel: "Error",
        detail: state.errorMessage ?? "Morrow needs attention.",
        pauseResumeLabel: "Disable Sync Now",
        syncNowEnabled: onboardingComplete,
        pendingProposalLabel,
        syncResultLabel
      }
    default:
      return assertNever(state.mode)
  }
}

export function isSyncNowEnabled(state: AppShellState): boolean {
  return getMenuModel(state).syncNowEnabled
}

export function isOnboardingComplete(state: AppShellState): boolean {
  return getOnboardingWarnings(state).length === 0
}

export function getOnboardingWarnings(state: AppShellState): readonly string[] {
  const warnings: string[] = []
  const discoveryWarning = chatDiscoveryWarning(state.discovery)
  if (discoveryWarning !== undefined) {
    warnings.push(discoveryWarning)
  }
  if (state.selectedChats.length === 0) {
    warnings.push("Select at least one chat before scanning.")
  } else if (!selectedChatsAreVerified(state.discovery, state.selectedChats)) {
    warnings.push("Refresh chat discovery before scanning selected chats.")
  }
  const providerCredentialWarning = providerCredentialWarnings[state.providerCredentialStatus]
  if (providerCredentialWarning !== undefined) {
    warnings.push(providerCredentialWarning)
  }
  return warnings
}

const providerCredentialWarnings = {
  unchecked: "Morrow is checking Codex provider readiness before scanning.",
  configured: undefined,
  missing: "Finish Codex CLI setup in Settings before scanning."
} as const satisfies Record<ProviderCredentialStatus, string | undefined>

export function formatPendingProposalCount(count: number): string {
  return count >= 9 ? "9+" : String(count)
}

function assertNever(value: never): never {
  throw new UnhandledMenuModelVariantError(String(value))
}

class UnhandledMenuModelVariantError extends Error {
  readonly renderedValue: string

  constructor(renderedValue: string) {
    super(`Unhandled menu model variant: ${renderedValue}`)
    this.name = "UnhandledMenuModelVariantError"
    this.renderedValue = renderedValue
  }
}
