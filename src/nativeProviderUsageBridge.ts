import { invoke } from "@tauri-apps/api/core"
import {
  parseProviderUsageLoadRequest,
  parseProviderUsageReport,
  type ProviderUsageLoadRequest,
  type ProviderUsageReport
} from "./domain/providerUsage"

class ProviderUsageLoadError extends Error {
  constructor() {
    super("Provider usage report could not be loaded.")
    this.name = "ProviderUsageLoadError"
  }
}

export async function loadProviderUsageInTauri(
  request?: ProviderUsageLoadRequest
): Promise<ProviderUsageReport> {
  const parsedRequest = parseProviderUsageLoadRequest(request)
  try {
    const report = await invoke<unknown>("load_provider_usage", {
      request: parsedRequest
    })
    return parseProviderUsageReport(report)
  } catch (error) {
    if (error instanceof Error) {
      throw new ProviderUsageLoadError()
    }
    throw new ProviderUsageLoadError()
  }
}
