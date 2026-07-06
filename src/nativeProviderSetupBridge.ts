import { invoke } from "@tauri-apps/api/core"
import {
  parseCodexProviderAuthReadiness,
  type CodexProviderAuthReadiness
} from "./providerAuthBridge"
import {
  parseCodexCliInstallReceipt,
  parseCodexLoginLaunchReceipt,
  type CodexCliInstallReceipt,
  type CodexLoginLaunchReceipt
} from "./providerSetupBridge"

export async function checkProviderAuthInTauri(): Promise<CodexProviderAuthReadiness> {
  const readiness = await invoke<unknown>("check_provider_auth")
  return parseCodexProviderAuthReadiness(readiness)
}

export async function installCodexCliInTauri(): Promise<CodexCliInstallReceipt> {
  const receipt = await invoke<unknown>("install_codex_cli")
  return parseCodexCliInstallReceipt(receipt)
}

export async function startCodexLoginInTauri(): Promise<CodexLoginLaunchReceipt> {
  const receipt = await invoke<unknown>("start_codex_login")
  return parseCodexLoginLaunchReceipt(receipt)
}
