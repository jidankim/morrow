import { AggregateReviewFixture } from "./visualQaListIntakeAggregate"
import { ProposalReviewFixture } from "./visualQaListIntakeProposal"
import { SettingsProfileFixture } from "./visualQaListIntakeSettings"

const visualListIntakeStates = {
  "settings-profile": true,
  "settings-provider-unavailable": true,
  "aggregate-populated": true,
  "proposal-review": true
} as const

export type VisualListIntakeState = keyof typeof visualListIntakeStates

export function isVisualListIntakeState(value: string): value is VisualListIntakeState {
  return value in visualListIntakeStates
}

export function VisualListIntakeHarness({ stateName }: { readonly stateName: VisualListIntakeState }): JSX.Element {
  switch (stateName) {
    case "settings-profile":
      return <SettingsProfileFixture />
    case "settings-provider-unavailable":
      return <SettingsProfileFixture providerCredentialState={{
        status: "failed",
        message: "Codex provider readiness failed.",
        recoveryAction: "refresh"
      }} />
    case "aggregate-populated":
      return <AggregateReviewFixture />
    case "proposal-review":
      return <ProposalReviewFixture />
  }
}
