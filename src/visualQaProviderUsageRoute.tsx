import { ProviderUsageDashboard } from "./ProviderUsageDashboard"
import { visualProviderUsageState, visualProviderUsageStates, type VisualProviderUsageState } from "./visualQaProviderUsageFixtures"
import { VisualQaShell } from "./visualQaShell"

export type { VisualProviderUsageState }

export function VisualProviderUsageHarness({ stateName }: { readonly stateName: VisualProviderUsageState }): JSX.Element {
  return (
    <VisualQaShell stateName={stateName} lede="Provider usage visual QA">
      <ProviderUsageDashboard
        selectedWindowKey="30d"
        state={visualProviderUsageState(stateName)}
        onWindowChange={noopProviderUsageWindowChange}
      />
    </VisualQaShell>
  )
}

export function isVisualProviderUsageState(value: string): value is VisualProviderUsageState {
  return value in visualProviderUsageStates
}

function noopProviderUsageWindowChange(): void {}
