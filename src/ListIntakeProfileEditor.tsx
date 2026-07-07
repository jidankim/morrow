import { useEffect, useState } from "react"
import {
  createListIntakeProfile,
  type ListIntakeChatScope,
  type ListIntakeOutputPolicy,
  type ListIntakeProfile
} from "./domain/appConfig"
import type { ChatId } from "./domain/chatDiscovery"
import { DEFAULT_DIGEST_REMINDER } from "./domain/listIntakeProfileModel"
import {
  CategoryEditor,
  ExampleEditor,
  PreviewPanel
} from "./ListIntakeProfileEditorFields"
import { previewExamples } from "./listIntakePreview"

type ProfileEditorProps = {
  readonly chatScopeDraft: string
  readonly profile: ListIntakeProfile
  readonly onDelete: () => void
  readonly onDraftChatScope: (value: string) => void
  readonly onSave: (profile: ListIntakeProfile, message?: string) => boolean
}

const CHAT_ID_PATTERN = /^messages-chat-[a-f0-9]{32}$/
const MAX_PROFILE_NAME_LENGTH = 48

type ChatIdParseResult =
  | { readonly kind: "empty" }
  | { readonly kind: "invalid"; readonly token: string }
  | { readonly chatIds: readonly ChatId[]; readonly kind: "valid" }

export function ProfileEditor({
  chatScopeDraft,
  profile,
  onDelete,
  onDraftChatScope,
  onSave
}: ProfileEditorProps): JSX.Element {
  const [profileNameDraft, setProfileNameDraft] = useState(profile.name)
  const preview = previewExamples(profile)
  useEffect(() => {
    setProfileNameDraft(profile.name)
  }, [profile.profileId])

  const updateProfile = (profilePatch: ListIntakeProfile, message?: string): void => {
    onSave(createListIntakeProfile(profilePatch), message)
  }
  const updateProfileName = (draftName: string): void => {
    setProfileNameDraft(draftName)
    const nextName = draftName.trim()
    if (nextName.length > MAX_PROFILE_NAME_LENGTH || nextName.length === 0) {
      onSave(profile, "Profile name must be 1 to 48 visible characters.")
      return
    }
    updateProfile({ ...profile, name: nextName })
  }
  const updateOutputPolicy = (outputPolicy: ListIntakeOutputPolicy): void => {
    updateProfile({
      ...profile,
      outputPolicy,
      digestReminder: outputPolicy === "dailyDigestReminder" ? DEFAULT_DIGEST_REMINDER : undefined
    })
  }
  const updateChatScope = (mode: ListIntakeChatScope["mode"]): void => {
    if (mode === "allSelectedChats") {
      updateProfile({ ...profile, chatScope: { mode } })
      return
    }
    const parsedIds = parseChatIds(chatScopeDraft)
    switch (parsedIds.kind) {
      case "empty":
        onSave(profile, "Selected-chat scope needs at least one valid chat ID before enabling.")
        return
      case "invalid":
        onSave(profile, invalidChatIdMessage(parsedIds.token))
        return
      case "valid":
        updateProfile({ ...profile, chatScope: { mode, selectedChatIds: parsedIds.chatIds } })
        return
    }
  }
  const enableProfile = (): void => {
    if (chatScopeDraft.trim().length > 0) {
      const parsedIds = parseChatIds(chatScopeDraft)
      switch (parsedIds.kind) {
        case "empty":
          onSave(profile, "Invalid chat scope did not enable the profile.")
          return
        case "invalid":
          onSave(profile, invalidChatIdMessage(parsedIds.token))
          return
        case "valid":
          break
      }
    }
    if (profile.chatScope.mode === "selectedChatIds" && profile.chatScope.selectedChatIds.length === 0) {
      onSave(profile, "Invalid chat scope did not enable the profile.")
      return
    }
    updateProfile({ ...profile, enabled: true, migrationState: undefined }, "Profile enabled.")
  }

  return (
    <div className="list-intake-editor">
      {profile.migrationState === "needsReviewFromListReminderV1" ? (
        <div className="result-surface info" role="status">
          <h4>Review required before enabling</h4>
          <p>This migrated draft is disabled until its examples, scope, and output policy are reviewed.</p>
          <button className="button primary" onClick={enableProfile} type="button">
            Enable profile
          </button>
        </div>
      ) : null}
      {profile.migrationState === undefined ? (
        <div className="list-intake-toolbar">
          <button
            className={profile.enabled ? "button secondary" : "button primary"}
            onClick={() =>
              profile.enabled
                ? updateProfile({ ...profile, enabled: false }, "Profile disabled.")
                : enableProfile()
            }
            type="button"
          >
            {profile.enabled ? "Disable profile" : "Enable profile"}
          </button>
          <button className="button danger compact-danger" onClick={onDelete} type="button">
            Delete profile
          </button>
        </div>
      ) : null}
      <label className="field">
        <span>Profile name</span>
        <input
          maxLength={MAX_PROFILE_NAME_LENGTH}
          value={profileNameDraft}
          onBlur={() => setProfileNameDraft(profileNameDraft.trim())}
          onChange={(event) => updateProfileName(event.currentTarget.value)}
        />
      </label>
      <p className="settings-note">LLM-assisted quantity list · provider constrained</p>
      <fieldset className="option-group">
        <legend>Chat scope</legend>
        <label className="check-row">
          <input
            checked={profile.chatScope.mode === "allSelectedChats"}
            name={`chat-scope-${profile.profileId}`}
            onChange={() => updateChatScope("allSelectedChats")}
            type="radio"
          />
          <span>All selected chats</span>
        </label>
        <label className="check-row">
          <input
            checked={profile.chatScope.mode === "selectedChatIds"}
            name={`chat-scope-${profile.profileId}`}
            onChange={() => updateChatScope("selectedChatIds")}
            type="radio"
          />
          <span>Selected chats only</span>
        </label>
        <label className="field">
          <span>Selected chat IDs</span>
          <input
            value={chatScopeDraft}
            onBlur={() => updateChatScope(profile.chatScope.mode)}
            onChange={(event) => onDraftChatScope(event.currentTarget.value)}
          />
        </label>
      </fieldset>
      <ExampleEditor
        label="Positive examples"
        values={profile.positiveExamples}
        onUpdate={(positiveExamples) => updateProfile({ ...profile, positiveExamples })}
      />
      <ExampleEditor
        label="Negative examples"
        values={profile.negativeExamples}
        onUpdate={(negativeExamples) => updateProfile({ ...profile, negativeExamples })}
      />
      <fieldset className="option-group">
        <legend>Grouping</legend>
        <label className="check-row">
          <input checked disabled type="checkbox" />
          <span>By chat</span>
        </label>
        <label className="check-row">
          <input
            checked={profile.grouping.sender === "displayAlias"}
            onChange={(event) =>
              updateProfile({
                ...profile,
                grouping: { chat: true, sender: event.currentTarget.checked ? "displayAlias" : "off" }
              })
            }
            type="checkbox"
          />
          <span>By sender label</span>
        </label>
      </fieldset>
      <CategoryEditor
        categoryRules={profile.categoryRules}
        onUpdate={(categoryRules) => updateProfile({ ...profile, categoryRules })}
      />
      <fieldset className="option-group">
        <legend>Output policy</legend>
        <label className="check-row">
          <input
            checked={profile.outputPolicy === "aggregateOnly"}
            name={`output-${profile.profileId}`}
            onChange={() => updateOutputPolicy("aggregateOnly")}
            type="radio"
          />
          <span>Aggregate only</span>
        </label>
        <label className="check-row">
          <input
            checked={profile.outputPolicy === "dailyDigestReminder"}
            name={`output-${profile.profileId}`}
            onChange={() => updateOutputPolicy("dailyDigestReminder")}
            type="radio"
          />
          <span>Daily digest Reminder</span>
        </label>
      </fieldset>
      <PreviewPanel items={preview.items} rejected={preview.rejected} />
    </div>
  )
}

function parseChatIds(value: string): ChatIdParseResult {
  const tokens = value
    .split(",")
    .map((chatId) => chatId.trim())
    .filter((chatId) => chatId.length > 0)
  if (tokens.length === 0) {
    return { kind: "empty" }
  }
  const invalidToken = tokens.find((chatId) => !CHAT_ID_PATTERN.test(chatId))
  if (invalidToken !== undefined) {
    return { kind: "invalid", token: invalidToken }
  }
  return { chatIds: tokens, kind: "valid" }
}

function invalidChatIdMessage(token: string): string {
  return `Invalid chat ID "${token}" in selected-chat scope.`
}

export function chatScopeText(chatScope: ListIntakeChatScope): string {
  switch (chatScope.mode) {
    case "allSelectedChats":
      return ""
    case "selectedChatIds":
      return chatScope.selectedChatIds.join(", ")
  }
}
