import { useState } from "react"
import {
  createListIntakeProfile,
  listIntakeProfilesSchema,
  type AppConfig,
  type ListIntakeProfile
} from "./domain/appConfig"
import {
  DEFAULT_AGGREGATION,
  DEFAULT_GROUPING,
  DEFAULT_QUANTITY_LIST_BOUNDS,
  DEFAULT_THRESHOLDS
} from "./domain/listIntakeProfileModel"
import { chatScopeText, ProfileEditor } from "./ListIntakeProfileEditor"
import type { ProviderCredentialState } from "./SettingsProviderCredentialSection"

type ListIntakeSettingsSectionProps = {
  readonly config: AppConfig
  readonly onChange: (config: AppConfig) => void
  readonly providerCredentialState: ProviderCredentialState
}

const DEFAULT_POSITIVE_EXAMPLE = "2 anchovies, 3 salmon"
const DEFAULT_SEAFOOD_KEYWORDS = ["anchovies", "anchovy", "salmon"] as const
export function ListIntakeSettingsSection({
  config,
  onChange,
  providerCredentialState
}: ListIntakeSettingsSectionProps): JSX.Element {
  const [profileError, setProfileError] = useState<string | undefined>(undefined)
  const [chatScopeDrafts, setChatScopeDrafts] = useState<Record<string, string>>({})
  const [selectedProfileId, setSelectedProfileId] = useState<string | undefined>(
    config.listIntakeProfiles[0]?.profileId
  )
  const selectedProfile =
    config.listIntakeProfiles.find((profile) => profile.profileId === selectedProfileId) ??
    config.listIntakeProfiles[0]

  const saveProfiles = (profiles: readonly ListIntakeProfile[], successMessage?: string): boolean => {
    const parsedProfiles = listIntakeProfilesSchema.safeParse(profiles)
    if (!parsedProfiles.success) {
      setProfileError(parsedProfiles.error.issues[0]?.message ?? "Profile settings need review.")
      return false
    }
    setProfileError(successMessage)
    onChange({ ...config, listIntakeProfiles: parsedProfiles.data })
    return true
  }

  const addProfile = (): void => {
    const newProfile = createListIntakeProfile({
      enabled: false,
      profileId: nextProfileId(config.listIntakeProfiles),
      name: nextProfileName(config.listIntakeProfiles),
      profileVersion: "list-intake-v2",
      kind: "quantityList",
      extractionMode: "providerConstrained",
      providerPromptVersion: "list-intake-v1",
      positiveExamples: [DEFAULT_POSITIVE_EXAMPLE],
      negativeExamples: [],
      categoryRules: [{ categoryId: "seafood", displayName: "Seafood", keywords: DEFAULT_SEAFOOD_KEYWORDS }],
      aggregation: DEFAULT_AGGREGATION,
      chatScope: { mode: "allSelectedChats" },
      grouping: DEFAULT_GROUPING,
      captureFromScheduledMessages: false,
      outputPolicy: "aggregateOnly",
      quantityListBounds: DEFAULT_QUANTITY_LIST_BOUNDS,
      thresholds: DEFAULT_THRESHOLDS
    })
    if (saveProfiles([...config.listIntakeProfiles, newProfile], "Profile draft created.")) {
      setSelectedProfileId(newProfile.profileId)
    }
  }

  return (
    <section aria-labelledby="list-intake-heading" className="settings-section list-intake-section">
      <div className="settings-section-header">
        <div>
          <h3 id="list-intake-heading">List intake</h3>
          <p className="settings-copy">
            LLM-assisted quantity-list profiles run beside scheduling and aggregate matched rows by
            the local day in the reference timezone.
          </p>
        </div>
        <button
          className="button secondary"
          disabled={config.listIntakeProfiles.length >= 10}
          onClick={addProfile}
          type="button"
        >
          Add profile
        </button>
      </div>
      <ProviderPreviewStatus state={providerCredentialState} />
      {profileError !== undefined ? <p className="inline-status">{profileError}</p> : null}
      {config.listIntakeProfiles.length === 0 ? (
        <p className="settings-note">No list-intake profiles are active.</p>
      ) : (
        <div className="list-intake-layout">
          <div className="list-intake-profile-list" aria-label="List intake profiles">
            {config.listIntakeProfiles.map((profile) => (
              <button
                className={profile.profileId === selectedProfile?.profileId ? "profile-tab active" : "profile-tab"}
                key={profile.profileId}
                onClick={() => setSelectedProfileId(profile.profileId)}
                type="button"
              >
                <span>{profile.name}</span>
                <span>{profile.enabled ? "Enabled" : "Disabled"}</span>
              </button>
            ))}
          </div>
          {selectedProfile !== undefined ? (
            <ProfileEditor
              chatScopeDraft={chatScopeDrafts[selectedProfile.profileId] ?? chatScopeText(selectedProfile.chatScope)}
              profile={selectedProfile}
              onDelete={() => {
                const remainingProfiles = config.listIntakeProfiles.filter(
                  (profile) => profile.profileId !== selectedProfile.profileId
                )
                setSelectedProfileId(remainingProfiles[0]?.profileId)
                saveProfiles(remainingProfiles, "Profile deleted.")
              }}
              onDraftChatScope={(value) =>
                setChatScopeDrafts({ ...chatScopeDrafts, [selectedProfile.profileId]: value })
              }
              onSave={(profile, message) =>
                saveProfiles(
                  config.listIntakeProfiles.map((currentProfile) =>
                    currentProfile.profileId === profile.profileId ? profile : currentProfile
                  ),
                  message
                )
              }
            />
          ) : null}
        </div>
      )}
    </section>
  )
}

function ProviderPreviewStatus({
  state
}: {
  readonly state: ProviderCredentialState
}): JSX.Element {
  switch (state.status) {
    case "ready":
      return <p className="inline-status success">Provider preview ready.</p>
    case "checking":
    case "installing":
    case "loginLaunching":
    case "loginPolling":
      return <p className="inline-status">Provider preview unavailable while readiness is updating.</p>
    case "idle":
    case "missingCli":
    case "installConfirming":
    case "notLoggedIn":
      return <p className="inline-status">Provider preview unavailable. Retry after Codex provider setup.</p>
    case "failed":
      return (
        <p className="inline-status error" role="alert">
          Provider preview unavailable. Retry provider readiness.
        </p>
      )
    default:
      return assertNever(state)
  }
}

function assertNever(value: never): never {
  throw new Error(`Unhandled provider credential variant: ${String(value)}`)
}

function nextProfileId(profiles: readonly ListIntakeProfile[]): string {
  const existingIds = new Set(profiles.map((profile) => profile.profileId))
  for (let index = 1; index <= 10; index += 1) {
    const candidate = `list-intake-profile-${index.toString().padStart(2, "0")}`
    if (!existingIds.has(candidate)) {
      return candidate
    }
  }
  return "list-intake-profile-10"
}

function nextProfileName(profiles: readonly ListIntakeProfile[]): string {
  const existingNames = new Set(profiles.map((profile) => profile.name.toLocaleLowerCase()))
  for (let index = 1; index <= 10; index += 1) {
    const candidate = index === 1 ? "Quantity list" : `Quantity list ${index}`
    if (!existingNames.has(candidate.toLocaleLowerCase())) {
      return candidate
    }
  }
  return "Quantity list 10"
}
