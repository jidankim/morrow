export const SUPPORTED_REFERENCE_TIME_ZONES = [
  "Asia/Seoul",
  "America/New_York",
  "Europe/London",
  "UTC"
] as const

export type SupportedReferenceTimeZone = (typeof SUPPORTED_REFERENCE_TIME_ZONES)[number]

export function isSupportedReferenceTimeZone(
  value: string
): value is SupportedReferenceTimeZone {
  return SUPPORTED_REFERENCE_TIME_ZONES.some((timeZone) => timeZone === value)
}

export function normalizeReferenceTimeZone(value: string): SupportedReferenceTimeZone {
  if (isSupportedReferenceTimeZone(value)) {
    return value
  }
  return "UTC"
}
