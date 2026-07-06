import type { ProviderUsageWindowKey } from "./domain/providerUsage"

export function formatCount(value: number): string {
  if (!Number.isFinite(value)) {
    return "0"
  }
  return Math.max(0, Math.floor(value)).toLocaleString("en-US")
}

export function formatRate(value: number): string {
  return `${formatPercentNumber(value)}%`
}

export function formatOptionalPercent(value: number | undefined): string {
  if (value === undefined) {
    return "Not recorded"
  }
  return formatRate(value)
}

export function windowLabel(windowKey: ProviderUsageWindowKey): string {
  switch (windowKey) {
    case "7d":
      return "7 days"
    case "30d":
      return "30 days"
    case "90d":
      return "90 days"
    case "all":
      return "All time"
    default:
      return assertNeverWindowKey(windowKey)
  }
}

function formatPercentNumber(value: number): string {
  if (!Number.isFinite(value)) {
    return "0.0"
  }
  const bounded = Math.min(1, Math.max(0, value))
  return (Math.round(bounded * 1_000) / 10).toFixed(1)
}

function assertNeverWindowKey(windowKey: never): never {
  throw new Error(`Unsupported provider usage window: ${windowKey}`)
}
