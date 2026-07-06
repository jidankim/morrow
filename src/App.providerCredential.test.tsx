import { act, fireEvent, render, screen, waitFor } from "@testing-library/react"
import { beforeEach, describe, expect, it, vi } from "vitest"
import { App } from "./App"
import { APP_SHELL_STATE_KEY, createDefaultAppShellState } from "./domain/appShell"
import type { SyncSchedulerState } from "./domain/syncScheduler"

const discoveredChat = {
  id: "messages-chat-11111111111111111111111111111111",
  label: "Chat alpha",
  participantCount: 2,
  participantIds: [
    "messages-participant-11111111111111111111111111111111",
    "messages-participant-22222222222222222222222222222222"
  ],
  latestActivityTimestamp: 1_783_000_000
} as const

const bridgeMock = vi.hoisted(() => {
  const defaultSchedulerState: SyncSchedulerState = {
    enabled: false,
    interval_seconds: 1_800,
    status: "disabled",
    retry_attempt: 0,
    updated_at: 1_783_000_000
  }
  return {
    getState: vi.fn(async () => undefined),
    setShellState: vi.fn(async () => undefined),
    getRuntimeIdentity: vi.fn(async () => undefined),
    subscribeAppState: vi.fn(async () => vi.fn()),
    subscribeMenuCommand: vi.fn(async () => vi.fn()),
    reconcileNow: vi.fn(async () => undefined),
    scanSelectedChats: vi.fn(async () => ({ pendingProposalCount: 12 })),
    loadDecisionEvidence: vi.fn(async () => ({
      items: [],
      skippedTraceLineCount: 0,
      latestEvalStatus: "never_run"
    })),
    getSyncSchedulerState: vi.fn(async () => defaultSchedulerState),
    setSyncSchedulerState: vi.fn(async (state: SyncSchedulerState) => state),
    storeMorrowToken: vi.fn(async () => ({
      storageSurface: "keychainBridge",
      stored: true,
      deleted: false
    })),
    checkProviderAuth: vi.fn(async () => ({
      status: "loggedInUsingChatGpt",
      ready: true,
      commandSurface: "codex login status",
      commandOutputRedacted: true,
      diagnostic: "Codex CLI ChatGPT session is ready."
    })),
    installCodexCli: vi.fn(async () => ({
      status: "installed",
      commandSurface: "codex setup",
      commandOutputRedacted: true,
      diagnostic: "Codex CLI is installed."
    })),
    startCodexLogin: vi.fn(async () => ({
      status: "launched",
      commandSurface: "codex login",
      commandOutputRedacted: true,
      diagnostic: "Codex login started."
    })),
    readMorrowToken: vi.fn(async () => ({
      storageSurface: "keychainBridge",
      present: true
    })),
    deleteMorrowToken: vi.fn(async () => ({
      storageSurface: "keychainBridge",
      stored: false,
      deleted: true
    })),
    discoverMessagesChats: vi.fn(async () => ({
      status: "ready",
      chats: [
        {
          chatId: "messages-chat-11111111111111111111111111111111",
          displayLabel: "Chat alpha",
          participantCount: 2,
          participantIds: [
            "messages-participant-11111111111111111111111111111111",
            "messages-participant-22222222222222222222222222222222"
          ],
          latestActivityTimestamp: 1_783_000_000
        }
      ]
    })),
    openPrivacySettings: vi.fn(async () => ({ pane: "fullDiskAccess", opened: true })),
    deleteMorrowData: vi.fn(async () => undefined),
    recordCrashLog: vi.fn(async () => ({ stored: true }))
  }
})

vi.mock("./tauriBridge", () => ({
  MORROW_KEYCHAIN_SERVICE: "com.morrow.desktop.token",
  MORROW_PROVIDER_TOKEN_KIND: "morrow-openai-provider-api-key",
  createNativeShellBridge: () => bridgeMock
}))

const renderSettings = (): void => {
  render(<App />)
  act(() => {
    window.location.hash = "#settings"
    window.dispatchEvent(new HashChangeEvent("hashchange"))
  })
}

const seedReadyState = (): void => {
  const initial = createDefaultAppShellState()
  window.localStorage.setItem(
    APP_SHELL_STATE_KEY,
    JSON.stringify({
      ...initial,
      config: { ...initial.config, permissionsGranted: false },
      discovery: { status: "ready", chats: [discoveredChat] },
      selectedChats: [{ ...discoveredChat, backfillPromptEnabled: true }]
    })
  )
}

describe("App provider credential controls", () => {
  beforeEach(() => {
    window.localStorage.clear()
    window.location.hash = ""
    bridgeMock.storeMorrowToken.mockClear()
    bridgeMock.checkProviderAuth.mockClear()
    bridgeMock.installCodexCli.mockClear()
    bridgeMock.startCodexLogin.mockClear()
    bridgeMock.readMorrowToken.mockClear()
    bridgeMock.deleteMorrowToken.mockClear()
    bridgeMock.getSyncSchedulerState.mockClear()
    bridgeMock.setSyncSchedulerState.mockClear()
    bridgeMock.checkProviderAuth.mockResolvedValue({
      status: "loggedInUsingChatGpt",
      ready: true,
      commandSurface: "codex login status",
      commandOutputRedacted: true,
      diagnostic: "Codex CLI ChatGPT session is ready."
    })
    bridgeMock.installCodexCli.mockResolvedValue({
      status: "installed",
      commandSurface: "codex setup",
      commandOutputRedacted: true,
      diagnostic: "Codex CLI is installed."
    })
    bridgeMock.startCodexLogin.mockResolvedValue({
      status: "launched",
      commandSurface: "codex login",
      commandOutputRedacted: true,
      diagnostic: "Codex login started."
    })
  })

  it("shows Codex provider readiness without an API-key password field", async () => {
    renderSettings()

    await waitFor(() => expect(bridgeMock.checkProviderAuth).toHaveBeenCalledOnce())

    expect(screen.queryByText("OpenAI API key")).not.toBeInTheDocument()
    expect(screen.queryByLabelText("API key")).not.toBeInTheDocument()
    expect(document.querySelector('input[type="password"]')).toBeNull()
    expect(screen.getByRole("heading", { name: "Codex provider" })).toBeInTheDocument()
    expect(screen.getByText("Codex provider is ready.")).toBeInTheDocument()
    expect(bridgeMock.storeMorrowToken).not.toHaveBeenCalled()
    expect(bridgeMock.readMorrowToken).not.toHaveBeenCalled()
    expect(bridgeMock.deleteMorrowToken).not.toHaveBeenCalled()
    expect(document.body).not.toHaveTextContent("redacted-provider-token-for-ui")

    fireEvent.click(screen.getByRole("button", { name: "Refresh readiness" }))
    await waitFor(() => expect(bridgeMock.checkProviderAuth).toHaveBeenCalledTimes(2))
  })

  it("blocks Sync Now and links to Settings when Codex provider readiness is missing", async () => {
    bridgeMock.checkProviderAuth.mockResolvedValueOnce({
      status: "notLoggedIn",
      ready: false,
      commandSurface: "codex login status --raw codex_access_token=leaked",
      commandOutputRedacted: true,
      diagnostic: "raw codex_access_token=leaked"
    })
    seedReadyState()
    render(<App />)

    await waitFor(() => expect(bridgeMock.checkProviderAuth).toHaveBeenCalledOnce())
    await waitFor(() => expect(screen.getByRole("button", { name: "Sync Now" })).toBeDisabled())

    expect(screen.getByTestId("sync-state")).toHaveTextContent("Disabled")
    expect(screen.getByText("Codex provider")).toBeInTheDocument()
    expect(screen.getAllByText("Finish Codex CLI setup in Settings before scanning.")).not.toEqual([])
    expect(document.body).not.toHaveTextContent("redacted-provider-token-for-ui")
    expect(document.body).not.toHaveTextContent("codex_access_token=leaked")
    expect(document.body).not.toHaveTextContent("codex login status --raw")

    fireEvent.click(screen.getByRole("button", { name: "Sync Now" }))
    expect(bridgeMock.reconcileNow).not.toHaveBeenCalled()
    expect(bridgeMock.scanSelectedChats).not.toHaveBeenCalled()

    fireEvent.click(screen.getByRole("button", { name: "Configure provider" }))
    expect(screen.getByRole("heading", { name: "Settings" })).toBeInTheDocument()
  })

  it("shows ready provider setup state without a copy-command surface", async () => {
    renderSettings()

    await waitFor(() => expect(bridgeMock.checkProviderAuth).toHaveBeenCalledOnce())

    expect(screen.getByText("Codex provider is ready.")).toBeInTheDocument()
    expect(screen.getByRole("button", { name: "Refresh readiness" })).toBeEnabled()
    expect(screen.queryByRole("button", { name: /copy login command/i })).not.toBeInTheDocument()
    expect(document.body).not.toHaveTextContent("codex login")
  })

  it("confirms and cancels Codex CLI install before invoking the native installer", async () => {
    bridgeMock.checkProviderAuth.mockResolvedValueOnce({
      status: "missingCli",
      ready: false,
      commandSurface: "codex login status with secret raw output",
      commandOutputRedacted: true,
      diagnostic: "codex_access_token=raw-output"
    })

    renderSettings()
    await waitFor(() => expect(bridgeMock.checkProviderAuth).toHaveBeenCalledOnce())

    fireEvent.click(screen.getByRole("button", { name: "Install Codex CLI" }))

    expect(screen.getByText("Install Codex CLI now?")).toBeInTheDocument()
    expect(screen.getByRole("button", { name: "Install" })).toBeEnabled()
    expect(screen.getByRole("button", { name: "Cancel" })).toBeEnabled()
    expect(bridgeMock.installCodexCli).not.toHaveBeenCalled()

    fireEvent.click(screen.getByRole("button", { name: "Cancel" }))

    expect(screen.getByRole("button", { name: "Install Codex CLI" })).toBeInTheDocument()
    expect(bridgeMock.installCodexCli).not.toHaveBeenCalled()
    expect(document.body).not.toHaveTextContent("codex_access_token=raw-output")
    expect(document.body).not.toHaveTextContent("secret raw output")
  })

  it("installs Codex CLI after confirmation and refreshes readiness", async () => {
    bridgeMock.checkProviderAuth
      .mockResolvedValueOnce({
        status: "missingCli",
        ready: false,
        commandSurface: "codex login status",
        commandOutputRedacted: true,
        diagnostic: "missing"
      })
      .mockResolvedValueOnce({
        status: "loggedInUsingChatGpt",
        ready: true,
        commandSurface: "codex login status",
        commandOutputRedacted: true,
        diagnostic: "ready"
      })

    renderSettings()
    await waitFor(() => expect(bridgeMock.checkProviderAuth).toHaveBeenCalledOnce())

    fireEvent.click(screen.getByRole("button", { name: "Install Codex CLI" }))
    fireEvent.click(screen.getByRole("button", { name: "Install" }))

    await waitFor(() => expect(bridgeMock.installCodexCli).toHaveBeenCalledOnce())
    await waitFor(() => expect(bridgeMock.checkProviderAuth).toHaveBeenCalledTimes(2))
    expect(screen.getByText("Codex provider is ready.")).toBeInTheDocument()
  })

  it("shows sanitized install failure and allows retry setup", async () => {
    bridgeMock.checkProviderAuth.mockResolvedValueOnce({
      status: "missingCli",
      ready: false,
      commandSurface: "codex login status",
      commandOutputRedacted: true,
      diagnostic: "missing"
    })
    bridgeMock.installCodexCli.mockResolvedValueOnce({
      status: "failed",
      commandSurface: "installer secret output",
      commandOutputRedacted: true,
      diagnostic: "codex_access_token=raw-output"
    })

    renderSettings()
    await waitFor(() => expect(bridgeMock.checkProviderAuth).toHaveBeenCalledOnce())

    fireEvent.click(screen.getByRole("button", { name: "Install Codex CLI" }))
    fireEvent.click(screen.getByRole("button", { name: "Install" }))

    await waitFor(() => expect(bridgeMock.installCodexCli).toHaveBeenCalledOnce())
    expect(screen.getByRole("button", { name: "Retry setup" })).toBeEnabled()
    expect(screen.getByRole("alert")).toHaveTextContent("Codex CLI setup could not complete.")
    expect(document.body).not.toHaveTextContent("codex_access_token=raw-output")
    expect(document.body).not.toHaveTextContent("installer secret output")
  })

  it("requires install confirmation again when retrying a failed setup", async () => {
    bridgeMock.checkProviderAuth.mockResolvedValueOnce({
      status: "missingCli",
      ready: false,
      commandSurface: "codex login status",
      commandOutputRedacted: true,
      diagnostic: "missing"
    })
    bridgeMock.installCodexCli.mockResolvedValueOnce({
      status: "failed",
      commandSurface: "installer secret output",
      commandOutputRedacted: true,
      diagnostic: "codex_access_token=raw-output"
    })

    renderSettings()
    await waitFor(() => expect(bridgeMock.checkProviderAuth).toHaveBeenCalledOnce())

    fireEvent.click(screen.getByRole("button", { name: "Install Codex CLI" }))
    fireEvent.click(screen.getByRole("button", { name: "Install" }))
    await waitFor(() => expect(bridgeMock.installCodexCli).toHaveBeenCalledOnce())

    fireEvent.click(screen.getByRole("button", { name: "Retry setup" }))

    expect(screen.getByText("Install Codex CLI now?")).toBeInTheDocument()
    expect(screen.getByRole("button", { name: "Install" })).toBeEnabled()
    expect(screen.getByRole("button", { name: "Cancel" })).toBeEnabled()
    expect(bridgeMock.installCodexCli).toHaveBeenCalledOnce()

    fireEvent.click(screen.getByRole("button", { name: "Install" }))

    await waitFor(() => expect(bridgeMock.installCodexCli).toHaveBeenCalledTimes(2))
  })

  it("retries Codex login after launch failure without invoking setup", async () => {
    bridgeMock.checkProviderAuth
      .mockResolvedValueOnce({
        status: "notLoggedIn",
        ready: false,
        commandSurface: "codex login status",
        commandOutputRedacted: true,
        diagnostic: "login required"
      })
      .mockResolvedValueOnce({
        status: "loggedInUsingChatGpt",
        ready: true,
        commandSurface: "codex login status",
        commandOutputRedacted: true,
        diagnostic: "ready"
      })
    bridgeMock.startCodexLogin
      .mockResolvedValueOnce({
        status: "failedToStart",
        commandSurface: "codex login with secret output",
        commandOutputRedacted: true,
        diagnostic: "codex_access_token=raw-output"
      })
      .mockResolvedValueOnce({
        status: "launched",
        commandSurface: "codex login",
        commandOutputRedacted: true,
        diagnostic: "Codex login started."
      })

    renderSettings()
    await waitFor(() => expect(bridgeMock.checkProviderAuth).toHaveBeenCalledOnce())

    fireEvent.click(screen.getByRole("button", { name: "Start Codex login" }))
    await waitFor(() => expect(bridgeMock.startCodexLogin).toHaveBeenCalledOnce())

    expect(screen.getByRole("alert")).toHaveTextContent("Codex login could not be started.")
    expect(screen.getByRole("button", { name: "Start Codex login" })).toBeEnabled()
    expect(bridgeMock.installCodexCli).not.toHaveBeenCalled()
    expect(document.body).not.toHaveTextContent("codex_access_token=raw-output")
    expect(document.body).not.toHaveTextContent("secret output")

    vi.useFakeTimers()
    try {
      fireEvent.click(screen.getByRole("button", { name: "Start Codex login" }))
      await act(async () => {
        await Promise.resolve()
      })
      expect(bridgeMock.startCodexLogin).toHaveBeenCalledTimes(2)
      expect(bridgeMock.installCodexCli).not.toHaveBeenCalled()

      await act(async () => {
        await vi.advanceTimersByTimeAsync(2_000)
      })

      expect(screen.getByText("Codex provider is ready.")).toBeInTheDocument()
    } finally {
      vi.useRealTimers()
    }
  })

  it("launches Codex login and polls until provider readiness succeeds", async () => {
    bridgeMock.checkProviderAuth
      .mockResolvedValueOnce({
        status: "notLoggedIn",
        ready: false,
        commandSurface: "codex login status",
        commandOutputRedacted: true,
        diagnostic: "login required"
      })
      .mockResolvedValueOnce({
        status: "notLoggedIn",
        ready: false,
        commandSurface: "codex login status",
        commandOutputRedacted: true,
        diagnostic: "still waiting"
      })
      .mockResolvedValueOnce({
        status: "loggedInUsingChatGpt",
        ready: true,
        commandSurface: "codex login status",
        commandOutputRedacted: true,
        diagnostic: "ready"
      })

    renderSettings()
    await waitFor(() => expect(bridgeMock.checkProviderAuth).toHaveBeenCalledOnce())

    vi.useFakeTimers()
    try {
      fireEvent.click(screen.getByRole("button", { name: "Start Codex login" }))

      await act(async () => {
        await Promise.resolve()
      })
      expect(bridgeMock.startCodexLogin).toHaveBeenCalledOnce()
      expect(screen.getByRole("button", { name: "Waiting for browser login" })).toBeDisabled()

      await act(async () => {
        await vi.advanceTimersByTimeAsync(2_000)
      })

      expect(bridgeMock.checkProviderAuth).toHaveBeenCalledTimes(2)
      expect(screen.getByRole("button", { name: "Waiting for browser login" })).toBeDisabled()

      await act(async () => {
        await vi.advanceTimersByTimeAsync(2_000)
      })

      expect(bridgeMock.checkProviderAuth).toHaveBeenCalledTimes(3)
      expect(screen.getByText("Codex provider is ready.")).toBeInTheDocument()
    } finally {
      vi.useRealTimers()
    }
  })

  it("times out Codex login polling with sanitized refresh guidance", async () => {
    bridgeMock.checkProviderAuth.mockResolvedValue({
      status: "notLoggedIn",
      ready: false,
      commandSurface: "codex login status with raw output",
      commandOutputRedacted: true,
      diagnostic: "codex_access_token=raw-output"
    })

    renderSettings()
    await waitFor(() => expect(bridgeMock.checkProviderAuth).toHaveBeenCalledOnce())

    vi.useFakeTimers()
    try {
      fireEvent.click(screen.getByRole("button", { name: "Start Codex login" }))

      await act(async () => {
        await Promise.resolve()
      })
      expect(bridgeMock.startCodexLogin).toHaveBeenCalledOnce()
      await act(async () => {
        await vi.advanceTimersByTimeAsync(180_000)
      })

      expect(screen.getByRole("button", { name: "Refresh readiness" })).toBeEnabled()
      expect(screen.getByRole("alert")).toHaveTextContent("Codex login was not detected within 180 seconds.")
      expect(document.body).not.toHaveTextContent("codex_access_token=raw-output")
      expect(document.body).not.toHaveTextContent("raw output")
    } finally {
      vi.useRealTimers()
    }
  })

  it("maps unknown provider readiness failures to a sanitized retry action", async () => {
    bridgeMock.checkProviderAuth.mockResolvedValueOnce({
      status: "unknownFailure",
      ready: false,
      commandSurface: "codex login status --raw leaked",
      commandOutputRedacted: true,
      diagnostic: "codex_access_token=raw-output"
    })

    renderSettings()
    await waitFor(() => expect(bridgeMock.checkProviderAuth).toHaveBeenCalledOnce())

    expect(screen.getByRole("button", { name: "Refresh readiness" })).toBeEnabled()
    expect(screen.getByRole("alert")).toHaveTextContent("Codex provider readiness could not be confirmed.")
    expect(document.body).not.toHaveTextContent("codex_access_token=raw-output")
    expect(document.body).not.toHaveTextContent("codex login status --raw")
  })

  it("prevents duplicate setup actions while checking, installing, and polling login", async () => {
    let resolveInstall: (() => void) | undefined
    bridgeMock.checkProviderAuth
      .mockResolvedValueOnce({
        status: "loggedInUsingChatGpt",
        ready: true,
        commandSurface: "codex login status",
        commandOutputRedacted: true,
        diagnostic: "ready"
      })
      .mockResolvedValueOnce({
        status: "missingCli",
        ready: false,
        commandSurface: "codex login status",
        commandOutputRedacted: true,
        diagnostic: "missing"
      })
      .mockResolvedValueOnce({
        status: "notLoggedIn",
        ready: false,
        commandSurface: "codex login status",
        commandOutputRedacted: true,
        diagnostic: "not logged in"
      })
    bridgeMock.installCodexCli.mockReturnValueOnce(
      new Promise((resolve) => {
        resolveInstall = () =>
          resolve({
            status: "installed",
            commandSurface: "codex setup",
            commandOutputRedacted: true,
            diagnostic: "installed"
          })
      })
    )

    try {
      renderSettings()
      await waitFor(() => expect(bridgeMock.checkProviderAuth).toHaveBeenCalledOnce())

      fireEvent.click(screen.getByRole("button", { name: "Refresh readiness" }))
      expect(screen.getByRole("button", { name: "Checking" })).toBeDisabled()
      await waitFor(() => expect(bridgeMock.checkProviderAuth).toHaveBeenCalledTimes(2))
      await waitFor(() => expect(screen.getByRole("button", { name: "Install Codex CLI" })).toBeEnabled())

      fireEvent.click(screen.getByRole("button", { name: "Install Codex CLI" }))
      fireEvent.click(screen.getByRole("button", { name: "Install" }))
      expect(screen.getByRole("button", { name: "Installing Codex CLI" })).toBeDisabled()
      fireEvent.click(screen.getByRole("button", { name: "Installing Codex CLI" }))
      expect(bridgeMock.installCodexCli).toHaveBeenCalledOnce()

      await act(async () => {
        resolveInstall?.()
      })

      await waitFor(() => expect(screen.getByRole("button", { name: "Start Codex login" })).toBeEnabled())
      vi.useFakeTimers()
      fireEvent.click(screen.getByRole("button", { name: "Start Codex login" }))
      await act(async () => {
        await Promise.resolve()
      })
      expect(screen.getByRole("button", { name: "Waiting for browser login" })).toBeDisabled()
      fireEvent.click(screen.getByRole("button", { name: "Waiting for browser login" }))
      expect(bridgeMock.startCodexLogin).toHaveBeenCalledOnce()
    } finally {
      vi.useRealTimers()
    }
  })

  it("keeps Sync Now blocked until provider readiness is logged in with ChatGPT", async () => {
    bridgeMock.checkProviderAuth
      .mockResolvedValueOnce({
        status: "notLoggedIn",
        ready: false,
        commandSurface: "codex login status",
        commandOutputRedacted: true,
        diagnostic: "login required"
      })
      .mockResolvedValueOnce({
        status: "loggedInUsingChatGpt",
        ready: true,
        commandSurface: "codex login status",
        commandOutputRedacted: true,
        diagnostic: "ready"
      })
    seedReadyState()

    render(<App />)

    await waitFor(() => expect(screen.getByRole("button", { name: "Sync Now" })).toBeDisabled())
    vi.useFakeTimers()
    try {
      fireEvent.click(screen.getByRole("button", { name: "Configure provider" }))
      fireEvent.click(screen.getByRole("button", { name: "Start Codex login" }))
      await act(async () => {
        await Promise.resolve()
        await vi.advanceTimersByTimeAsync(2_000)
      })

      expect(screen.getByText("Codex provider is ready.")).toBeInTheDocument()
      vi.useRealTimers()
      fireEvent.click(screen.getByRole("link", { name: "Status" }))
      await waitFor(() => expect(screen.getByRole("button", { name: "Sync Now" })).toBeEnabled())
    } finally {
      vi.useRealTimers()
    }
  })
})
