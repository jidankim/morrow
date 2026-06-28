import { KeyRound, Search, Trash2 } from "lucide-react"
import { useState } from "react"

export type ProviderCredentialState =
  | { readonly status: "idle" }
  | { readonly status: "checking" }
  | { readonly status: "saving" }
  | { readonly status: "deleting" }
  | { readonly status: "present" }
  | { readonly status: "missing" }
  | { readonly status: "saved" }
  | { readonly status: "deleted" }
  | { readonly status: "failed"; readonly message: string }

type SettingsProviderCredentialSectionProps = {
  readonly state: ProviderCredentialState
  readonly onCheckProviderCredential: () => Promise<void>
  readonly onDeleteProviderCredential: () => Promise<void>
  readonly onSaveProviderCredential: (token: string) => Promise<void>
}

export function SettingsProviderCredentialSection({
  state,
  onCheckProviderCredential,
  onDeleteProviderCredential,
  onSaveProviderCredential
}: SettingsProviderCredentialSectionProps): JSX.Element {
  const [providerToken, setProviderToken] = useState("")
  const actionInFlight =
    state.status === "checking" || state.status === "saving" || state.status === "deleting"
  const canSave = providerToken.trim().length > 0 && !actionInFlight

  const saveProviderCredential = async (): Promise<void> => {
    if (!canSave) {
      return
    }
    await onSaveProviderCredential(providerToken.trim())
    setProviderToken("")
  }

  return (
    <section className="settings-section" aria-labelledby="provider-credential-heading">
      <div>
        <p className="eyebrow">Provider credential</p>
        <h3 id="provider-credential-heading">OpenAI API key</h3>
      </div>
      <p className="settings-copy">
        Save an OpenAI API key in macOS Keychain for scheduling extraction. Morrow only shows
        whether a key is stored.
      </p>
      <label className="field provider-token-field">
        <span>API key</span>
        <input
          autoComplete="off"
          inputMode="text"
          type="password"
          value={providerToken}
          onChange={(event) => setProviderToken(event.currentTarget.value)}
        />
      </label>
      <div className="credential-action-row">
        <button
          className="button primary"
          disabled={!canSave}
          onClick={() => void saveProviderCredential()}
          type="button"
        >
          <KeyRound aria-hidden="true" size={16} />
          {state.status === "saving" ? "Saving key" : "Save key"}
        </button>
        <button
          className="button secondary"
          disabled={actionInFlight}
          onClick={() => void onCheckProviderCredential()}
          type="button"
        >
          <Search aria-hidden="true" size={16} />
          {state.status === "checking" ? "Checking" : "Check key"}
        </button>
        <button
          className="button secondary"
          disabled={actionInFlight}
          onClick={() => void onDeleteProviderCredential()}
          type="button"
        >
          <Trash2 aria-hidden="true" size={16} />
          {state.status === "deleting" ? "Deleting key" : "Delete key"}
        </button>
      </div>
      <ProviderCredentialMessage state={state} />
    </section>
  )
}

function ProviderCredentialMessage({
  state
}: {
  readonly state: ProviderCredentialState
}): JSX.Element | null {
  switch (state.status) {
    case "idle":
      return (
        <p className="inline-status">
          Provider credential has not been checked.
        </p>
      )
    case "checking":
      return (
        <p className="inline-status">
          Checking provider credential...
        </p>
      )
    case "saving":
      return (
        <p className="inline-status">
          Saving provider credential...
        </p>
      )
    case "deleting":
      return (
        <p className="inline-status">
          Deleting provider credential...
        </p>
      )
    case "present":
    case "saved":
      return (
        <p className="inline-status success">
          OpenAI API key is stored.
        </p>
      )
    case "missing":
    case "deleted":
      return (
        <p className="inline-status">
          No OpenAI API key is stored.
        </p>
      )
    case "failed":
      return (
        <p className="inline-status error" role="alert">
          {state.message}
        </p>
      )
    default:
      return assertNever(state)
  }
}

function assertNever(value: never): never {
  throw new Error(`Unhandled provider credential variant: ${String(value)}`)
}
