import { fireEvent, render, screen } from "@testing-library/react"
import { describe, expect, it, vi } from "vitest"
import {
  createDefaultAppConfig,
  createListIntakeProfile,
  type AppConfig
} from "./domain/appConfig"
import { createDefaultSyncSchedulerState } from "./domain/syncScheduler"
import { SettingsView, type ProviderCredentialState } from "./SettingsView"

const migratedProfile = createListIntakeProfile({
  enabled: false,
  profileId: "list-intake-legacy-list-reminders",
  name: "Migrated quantity list",
  profileVersion: "list-intake-v2",
  kind: "quantityList",
  extractionMode: "providerConstrained",
  providerPromptVersion: "list-intake-v1",
  positiveExamples: ["2 anchovies, 3 salmon"],
  negativeExamples: [],
  categoryRules: [],
  aggregation: { window: "localDay", timezoneSource: "referenceTimezone" },
  chatScope: { mode: "allSelectedChats" },
  grouping: { chat: true, sender: "off" },
  captureFromScheduledMessages: false,
  outputPolicy: "aggregateOnly",
  quantityListBounds: {
    minItems: 1,
    maxItems: 20,
    minQuantity: 1,
    maxQuantity: 999,
    maxItemNameVisibleChars: 80,
    maxUnitVisibleChars: 24,
    uncategorizedCategoryId: "uncategorized"
  },
  thresholds: { autoAggregateThresholdMillis: 850, reviewThresholdMillis: 550 },
  migrationState: "needsReviewFromListReminderV1"
})

describe("SettingsView list intake profiles", () => {
  it("creates and edits a named LLM-assisted profile with live preview", () => {
    const { onChange } = renderSettings()

    expect(screen.getByRole("heading", { name: "List intake" })).toBeInTheDocument()
    expect(screen.queryByText("Daily list reminders")).not.toBeInTheDocument()
    expect(screen.queryByRole("checkbox", { name: "Enable daily list reminders" })).not.toBeInTheDocument()

    fireEvent.click(screen.getByRole("button", { name: "Add profile" }))
    typeTextIntoInput("Profile name", "Fish count")
    fireEvent.change(screen.getByLabelText("Positive examples"), {
      target: { value: "2 anchovies, 3 salmon\nsalmon - 3 / anchovy - 2" }
    })
    fireEvent.change(screen.getByLabelText("Negative examples"), {
      target: { value: "remind me to buy fish tomorrow" }
    })
    fireEvent.click(screen.getByRole("checkbox", { name: "By sender label" }))
    fireEvent.change(screen.getByLabelText("Category 1 keywords"), {
      target: { value: "salmon, anchovy, anchovies" }
    })

    expect(screen.getByDisplayValue("Fish count")).toBeInTheDocument()
    expect(screen.getByText("LLM-assisted quantity list · provider constrained")).toBeInTheDocument()
    expect(screen.getByText("Aggregation window: Local day in reference timezone.")).toBeInTheDocument()
    expect(screen.getByText(/Needs review/)).toBeInTheDocument()
    expect(screen.getByText("Matched item rows")).toBeInTheDocument()
    expect(screen.getAllByText("salmon").length).toBeGreaterThan(0)
    expect(screen.getByText("Rejected negative example: remind me to buy fish tomorrow")).toBeInTheDocument()
    expect(rowForItem("anchovies")).toHaveTextContent("Seafood")
    expect(screen.queryByLabelText(/custom prompt/i)).not.toBeInTheDocument()
    expect(onChange).toHaveBeenLastCalledWith(
      expect.objectContaining({
        listIntakeProfiles: [
          expect.objectContaining({
            name: "Fish count",
            positiveExamples: ["2 anchovies, 3 salmon", "salmon - 3 / anchovy - 2"],
            negativeExamples: ["remind me to buy fish tomorrow"],
            grouping: { chat: true, sender: "displayAlias" },
            outputPolicy: "aggregateOnly"
          })
        ]
      })
    )
  })

  it("adds multiple keyword categories and applies them in the live preview", () => {
    const { onChange } = renderSettings()

    fireEvent.click(screen.getByRole("button", { name: "Add profile" }))
    fireEvent.change(screen.getByLabelText("Positive examples"), {
      target: { value: "2 anchovies, 3 salmon\n4 apples" }
    })
    expect(rowForItem("anchovies")).toHaveTextContent("Seafood")

    fireEvent.click(screen.getByRole("button", { name: "Add category" }))
    fireEvent.change(screen.getByLabelText("Category 2 name"), { target: { value: "Produce" } })
    fireEvent.change(screen.getByLabelText("Category 2 keywords"), { target: { value: "apple, apples" } })

    expect(rowForItem("apples")).toHaveTextContent("Produce")
    expect(onChange).toHaveBeenLastCalledWith(
      expect.objectContaining({
        listIntakeProfiles: [
          expect.objectContaining({
            categoryRules: [
              expect.objectContaining({
                categoryId: "seafood",
                displayName: "Seafood",
                keywords: ["anchovies", "anchovy", "salmon"]
              }),
              expect.objectContaining({
                categoryId: "produce",
                displayName: "Produce",
                keywords: ["apple", "apples"]
              })
            ]
          })
        ]
      })
    )
  })

  it("keeps category name input focused while editing the category id", () => {
    renderSettings()

    fireEvent.click(screen.getByRole("button", { name: "Add profile" }))
    const categoryNameInput = screen.getByLabelText("Category 1 name")
    categoryNameInput.focus()

    expect(categoryNameInput).toHaveFocus()
    fireEvent.change(categoryNameInput, { target: { value: "Seafoo" } })

    expect(screen.getByLabelText("Category 1 name")).toHaveFocus()
  })

  it("adds and deletes examples and deletes the profile", () => {
    const { onChange } = renderSettings()

    fireEvent.click(screen.getByRole("button", { name: "Add profile" }))
    fireEvent.click(screen.getByRole("button", { name: "Add positive example" }))
    expect(screen.getByLabelText("Positive examples")).toHaveDisplayValue(
      "2 anchovies, 3 salmon\n2 anchovies, 3 salmon"
    )

    fireEvent.click(screen.getByRole("button", { name: "Delete positive example 1" }))
    expect(screen.getByLabelText("Positive examples")).toHaveDisplayValue("2 anchovies, 3 salmon")

    fireEvent.click(screen.getByRole("button", { name: "Add negative example" }))
    fireEvent.click(screen.getByRole("button", { name: "Delete negative example 1" }))
    fireEvent.click(screen.getByRole("button", { name: "Delete profile" }))
    expect(onChange).toHaveBeenLastCalledWith(expect.objectContaining({ listIntakeProfiles: [] }))
  })

  it("shows provider preview unavailable and retry state when provider is not ready", () => {
    renderSettings(createDefaultAppConfig("Asia/Seoul"), { status: "notLoggedIn" })

    expect(screen.getByText("Provider preview unavailable. Retry after Codex provider setup.")).toBeInTheDocument()
  })

  it("handles chat scope, output policy, migrated enable control, and invalid drafts", () => {
    const { onChange } = renderSettings({
      ...createDefaultAppConfig("Asia/Seoul"),
      listIntakeProfiles: [migratedProfile]
    })

    expect(screen.getByText("Review required before enabling")).toBeInTheDocument()
    fireEvent.click(screen.getByRole("button", { name: "Enable profile" }))
    expect(onChange).toHaveBeenLastCalledWith(
      expect.objectContaining({
        listIntakeProfiles: [expect.objectContaining({ enabled: true })]
      })
    )

    fireEvent.change(screen.getByLabelText("Selected chat IDs"), { target: { value: "bad-chat-id" } })
    fireEvent.click(screen.getByRole("button", { name: "Disable profile" }))
    fireEvent.click(screen.getByRole("button", { name: "Enable profile" }))
    expect(screen.getByText('Invalid chat ID "bad-chat-id" in selected-chat scope.')).toBeInTheDocument()
    expect(onChange).toHaveBeenLastCalledWith(
      expect.objectContaining({
        listIntakeProfiles: [expect.objectContaining({ enabled: false })]
      })
    )

    fireEvent.change(screen.getByLabelText("Selected chat IDs"), {
      target: { value: "messages-chat-11111111111111111111111111111111, bad id!" }
    })
    fireEvent.click(screen.getByRole("radio", { name: "Selected chats only" }))
    expect(screen.getByText('Invalid chat ID "bad id!" in selected-chat scope.')).toBeInTheDocument()
    expect(onChange).toHaveBeenLastCalledWith(
      expect.objectContaining({
        listIntakeProfiles: [
          expect.objectContaining({ chatScope: { mode: "allSelectedChats" }, enabled: false })
        ]
      })
    )

    fireEvent.change(screen.getByLabelText("Selected chat IDs"), {
      target: { value: "messages-chat-11111111111111111111111111111111" }
    })
    fireEvent.click(screen.getByRole("radio", { name: "Selected chats only" }))
    fireEvent.click(screen.getByRole("radio", { name: "Daily digest Reminder" }))
    expect(onChange).toHaveBeenLastCalledWith(
      expect.objectContaining({
        listIntakeProfiles: [
          expect.objectContaining({
            chatScope: {
              mode: "selectedChatIds",
              selectedChatIds: ["messages-chat-11111111111111111111111111111111"]
            },
            outputPolicy: "dailyDigestReminder"
          })
        ]
      })
    )
  })

  it("rejects long names, too many examples, and unmatched positive examples without prompt fields", () => {
    renderSettings()

    fireEvent.click(screen.getByRole("button", { name: "Add profile" }))
    fireEvent.change(screen.getByLabelText("Profile name"), {
      target: { value: "This profile name is intentionally far beyond forty-eight visible characters" }
    })
    expect(screen.getByText("Profile name must be 1 to 48 visible characters.")).toBeInTheDocument()

    fireEvent.change(screen.getByLabelText("Positive examples"), {
      target: { value: Array.from({ length: 21 }, (_value, index) => `${index + 1} salmon`).join("\n") }
    })
    expect(screen.getByText(/20/)).toBeInTheDocument()

    fireEvent.change(screen.getByLabelText("Positive examples"), {
      target: { value: "plain words without quantities" }
    })
    expect(screen.getByText("Unmatched positive example: plain words without quantities")).toBeInTheDocument()
    fireEvent.change(screen.getByLabelText("Positive examples"), {
      target: { value: "555-123-4567, 2 salmon" }
    })
    expect(screen.getByText("Unmatched positive example: 555-123-4567, 2 salmon")).toBeInTheDocument()
    expect(screen.queryByRole("textbox", { name: /prompt/i })).not.toBeInTheDocument()
  })
})

function renderSettings(
  initialConfig = createDefaultAppConfig("Asia/Seoul"),
  providerCredentialState: ProviderCredentialState = { status: "ready" }
): {
  readonly onChange: ReturnType<typeof vi.fn<(config: AppConfig) => void>>
} {
  let config = initialConfig
  const onChange = vi.fn((nextConfig: AppConfig) => {
    config = nextConfig
    rerenderSettings()
  })
  const rendered = render(settingsView(config, onChange, providerCredentialState))
  const rerenderSettings = (): void => {
    rendered.rerender(settingsView(config, onChange, providerCredentialState))
  }
  return { onChange }
}

function typeTextIntoInput(label: string, text: string): void {
  const field = screen.getByLabelText(label)
  if (!(field instanceof HTMLInputElement)) {
    throw new TypeError(`${label} is not an input`)
  }
  fireEvent.change(field, { target: { value: "" } })
  for (const character of text) {
    fireEvent.change(field, { target: { value: `${field.value}${character}` } })
  }
}

function rowForItem(itemName: string): HTMLTableRowElement {
  const itemCell = screen.getByText(itemName)
  const row = itemCell.closest("tr")
  if (!(row instanceof HTMLTableRowElement)) {
    throw new TypeError(`No preview row for ${itemName}`)
  }
  return row
}

function settingsView(
  config: AppConfig,
  onChange: (config: AppConfig) => void,
  providerCredentialState: ProviderCredentialState
): JSX.Element {
  return (
    <SettingsView
      config={config}
      deleteAllState={{ status: "idle" }}
      providerCredentialState={providerCredentialState}
      syncScheduler={createDefaultSyncSchedulerState(1_783_000_000)}
      syncSchedulerNowUnixSeconds={1_783_000_000}
      onCancelInstallCodexCli={vi.fn()}
      onChange={onChange}
      onChangeAutomaticSyncInterval={vi.fn()}
      onCheckProviderCredential={vi.fn(async () => undefined)}
      onConfirmInstallCodexCli={vi.fn(async () => undefined)}
      onDeleteAll={vi.fn()}
      onOpenPrivacySettings={vi.fn(async () => undefined)}
      onStartCodexLogin={vi.fn(async () => undefined)}
      onToggleAutomaticSync={vi.fn()}
    />
  )
}
