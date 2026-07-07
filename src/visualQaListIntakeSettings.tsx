import { useState } from "react"
import { ListIntakeSettingsSection } from "./ListIntakeSettingsSection"
import type { ProviderCredentialState } from "./SettingsProviderCredentialSection"
import {
  createDefaultAppConfig,
  createListIntakeProfile,
  type AppConfig
} from "./domain/appConfig"
import {
  DEFAULT_AGGREGATION,
  DEFAULT_GROUPING,
  DEFAULT_QUANTITY_LIST_BOUNDS,
  DEFAULT_THRESHOLDS
} from "./domain/listIntakeProfileModel"

type SettingsProfileFixtureProps = {
  readonly providerCredentialState?: ProviderCredentialState
}

const readyProviderCredentialState = { status: "ready" } as const satisfies ProviderCredentialState

export function SettingsProfileFixture({
  providerCredentialState = readyProviderCredentialState
}: SettingsProfileFixtureProps): JSX.Element {
  const [config, setConfig] = useState<AppConfig>(() => createFishProfileConfig())
  return (
    <div className="panel list-intake-review-panel">
      <ListIntakeSettingsSection
        config={config}
        providerCredentialState={providerCredentialState}
        onChange={setConfig}
      />
    </div>
  )
}

function createFishProfileConfig(): AppConfig {
  return {
    ...createDefaultAppConfig("Asia/Seoul"),
    referenceTimezone: "Asia/Seoul",
    permissionsGranted: true,
    listIntakeProfiles: [
      createListIntakeProfile({
        enabled: true,
        profileId: "list-intake-fish-count",
        name: "Fish count",
        profileVersion: "list-intake-v2",
        kind: "quantityList",
        extractionMode: "providerConstrained",
        providerPromptVersion: "list-intake-v1",
        positiveExamples: ["2 anchovies, 3 salmon", "salmon - 3 / anchovy - 2"],
        negativeExamples: ["remind me to buy fish tomorrow"],
        categoryRules: [{ categoryId: "seafood", displayName: "Seafood", keywords: ["anchovy", "salmon"] }],
        aggregation: DEFAULT_AGGREGATION,
        chatScope: { mode: "allSelectedChats" },
        grouping: { ...DEFAULT_GROUPING, sender: "displayAlias" },
        captureFromScheduledMessages: false,
        outputPolicy: "aggregateOnly",
        quantityListBounds: DEFAULT_QUANTITY_LIST_BOUNDS,
        thresholds: DEFAULT_THRESHOLDS
      })
    ]
  }
}
