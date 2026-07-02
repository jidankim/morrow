import { fireEvent, render, screen } from "@testing-library/react"
import { describe, expect, it, vi } from "vitest"
import { createDefaultAppConfig } from "./domain/appConfig"
import { SettingsPrivacyControls } from "./SettingsPrivacyControls"

const renderPrivacyControls = (onChange = vi.fn()): ReturnType<typeof vi.fn> => {
  render(
    <SettingsPrivacyControls
      config={createDefaultAppConfig("Asia/Seoul")}
      deleteAllState={{ status: "idle" }}
      onChange={onChange}
      onDeleteAll={vi.fn()}
      onOpenPrivacySettings={vi.fn(async () => undefined)}
    />
  )
  return onChange
}

describe("SettingsPrivacyControls", () => {
  it("shows local-only diagnostics copy and rejects malformed retention changes", () => {
    const onChange = renderPrivacyControls()

    expect(screen.getByText(/private local files on this Mac/i)).toBeInTheDocument()
    expect(screen.getByText(/not uploads/i)).toBeInTheDocument()
    expect(screen.getByText(/Delete All deletes these files/i)).toBeInTheDocument()
    expect(screen.queryByLabelText(/telemetry/i)).not.toBeInTheDocument()

    fireEvent.click(screen.getByRole("checkbox", { name: "Write private local diagnostics files" }))

    expect(onChange).toHaveBeenCalledWith({
      ...createDefaultAppConfig("Asia/Seoul"),
      localDiagnosticsEnabled: true
    })

    onChange.mockClear()
    const retentionInput = screen.getByLabelText("Local diagnostics retention (days)")

    fireEvent.change(retentionInput, { target: { value: "0" } })
    fireEvent.change(retentionInput, { target: { value: "366" } })
    fireEvent.change(retentionInput, { target: { value: "abc" } })

    expect(onChange).not.toHaveBeenCalled()

    fireEvent.change(retentionInput, { target: { value: "365" } })

    expect(onChange).toHaveBeenCalledWith({
      ...createDefaultAppConfig("Asia/Seoul"),
      localDiagnosticsRetentionDays: 365
    })
  })
})
