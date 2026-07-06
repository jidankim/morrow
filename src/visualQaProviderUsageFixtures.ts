import type { ProviderUsageReport } from "./domain/providerUsage"
import type { ProviderUsageState } from "./useProviderUsageController"

export const visualProviderUsageStates = {
  loaded: true,
  empty: true,
  error: true
} as const

export type VisualProviderUsageState = keyof typeof visualProviderUsageStates

const visualProviderUsageReport = {
  generatedAtUnixSeconds: 1_783_000_600,
  window: {
    key: "30d",
    startUnixSeconds: 1_780_408_600,
    endUnixSeconds: 1_783_000_600
  },
  totals: {
    totalOutcomes: 18,
    candidateCount: 11,
    quietCount: 7,
    quietRate: 7 / 18,
    candidateRate: 11 / 18
  },
  providers: [
    {
      providerId: "provider <script>alert(\"usage\")</script>",
      totalOutcomes: 12,
      candidateCount: 8,
      quietCount: 4,
      quietRate: 1 / 3,
      candidateRate: 2 / 3,
      averageConfidence: 0.872,
      models: [
        {
          modelId: "model </button><img src=x onerror=alert(\"usage\")>",
          totalOutcomes: 9,
          candidateCount: 7,
          quietCount: 2,
          quietRate: 2 / 9,
          candidateRate: 7 / 9,
          averageConfidence: 0.901,
          promptVersions: [
            {
              promptVersion: "prompt-v7 </td><script>alert(\"usage\")</script>",
              totalOutcomes: 9,
              candidateCount: 7,
              quietCount: 2,
              quietRate: 2 / 9,
              candidateRate: 7 / 9,
              averageConfidence: 0.901
            }
          ]
        },
        {
          modelId: "compact-router",
          totalOutcomes: 3,
          candidateCount: 1,
          quietCount: 2,
          quietRate: 2 / 3,
          candidateRate: 1 / 3,
          averageConfidence: 0.742,
          promptVersions: [
            {
              promptVersion: "usage-summary-v2",
              totalOutcomes: 3,
              candidateCount: 1,
              quietCount: 2,
              quietRate: 2 / 3,
              candidateRate: 1 / 3,
              averageConfidence: 0.742
            }
          ]
        }
      ]
    },
    {
      providerId: "local-provider",
      totalOutcomes: 6,
      candidateCount: 3,
      quietCount: 3,
      quietRate: 0.5,
      candidateRate: 0.5,
      averageConfidence: 0.688,
      models: [
        {
          modelId: "offline-small",
          totalOutcomes: 6,
          candidateCount: 3,
          quietCount: 3,
          quietRate: 0.5,
          candidateRate: 0.5,
          averageConfidence: 0.688,
          promptVersions: [
            {
              promptVersion: "private-route-v1",
              totalOutcomes: 6,
              candidateCount: 3,
              quietCount: 3,
              quietRate: 0.5,
              candidateRate: 0.5,
              averageConfidence: 0.688
            }
          ]
        }
      ]
    }
  ],
  recentOutcomes: [
    {
      providerId: "provider <script>alert(\"usage\")</script>",
      modelId: "model </button><img src=x onerror=alert(\"usage\")>",
      promptVersion: "prompt-v7 </td><script>alert(\"usage\")</script>",
      outcomeKind: "candidate",
      routeLabel: "route <svg onload=alert(\"usage\")>",
      confidence: 0.93,
      createdAtUnixSeconds: 1_783_000_580
    },
    {
      providerId: "local-provider",
      modelId: "offline-small",
      promptVersion: "private-route-v1",
      outcomeKind: "quiet",
      routeLabel: "no_action",
      confidence: 0.62,
      createdAtUnixSeconds: 1_783_000_520
    },
    {
      providerId: "provider <script>alert(\"usage\")</script>",
      modelId: "compact-router",
      promptVersion: "usage-summary-v2",
      outcomeKind: "candidate",
      routeLabel: "candidate_route",
      confidence: 0.81,
      createdAtUnixSeconds: 1_783_000_460
    }
  ]
} satisfies ProviderUsageReport

export function visualProviderUsageState(stateName: VisualProviderUsageState): ProviderUsageState {
  switch (stateName) {
    case "loaded":
      return { status: "loaded", report: visualProviderUsageReport }
    case "empty":
      return { status: "empty" }
    case "error":
      return { status: "failed", message: "Provider usage could not be loaded." }
    default:
      return assertNeverVisualProviderUsageState(stateName)
  }
}

function assertNeverVisualProviderUsageState(stateName: never): never {
  throw new Error(`Unsupported provider usage visual state: ${String(stateName)}`)
}
