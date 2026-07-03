export const SYSTEM_REFERENCE_TIME_ZONE = "system"

export const FALLBACK_REFERENCE_TIME_ZONES = [
  "Asia/Seoul",
  "America/Los_Angeles",
  "America/Chicago",
  "America/Denver",
  "America/New_York",
  "America/Toronto",
  "America/Vancouver",
  "America/Mexico_City",
  "America/Sao_Paulo",
  "Europe/London",
  "Europe/Paris",
  "Europe/Berlin",
  "Europe/Madrid",
  "Europe/Rome",
  "Europe/Amsterdam",
  "Europe/Dublin",
  "Asia/Tokyo",
  "Asia/Shanghai",
  "Asia/Hong_Kong",
  "Asia/Singapore",
  "Asia/Taipei",
  "Asia/Kolkata",
  "Asia/Dubai",
  "Australia/Sydney",
  "Australia/Melbourne",
  "Pacific/Auckland",
  "UTC"
] as const

export const SUPPORTED_REFERENCE_TIME_ZONES = FALLBACK_REFERENCE_TIME_ZONES

export type ReferenceTimeZonePreference = typeof SYSTEM_REFERENCE_TIME_ZONE | string

export type ReferenceTimeZoneOption = {
  readonly value: ReferenceTimeZonePreference
  readonly label: string
}

export function isSupportedReferenceTimeZone(value: string): boolean {
  return isConcreteReferenceTimeZone(value)
}

export function isReferenceTimeZonePreference(value: string): boolean {
  if (value === SYSTEM_REFERENCE_TIME_ZONE) {
    return true
  }
  return isConcreteReferenceTimeZone(value)
}

export function normalizeReferenceTimeZone(value: string): string {
  if (isReferenceTimeZonePreference(value)) {
    return value
  }
  return "UTC"
}

export function resolveReferenceTimeZonePreference(
  preference: string,
  systemTimeZone: string | undefined
): string {
  if (preference === SYSTEM_REFERENCE_TIME_ZONE) {
    return isConcreteReferenceTimeZone(systemTimeZone) ? systemTimeZone : "UTC"
  }
  return isConcreteReferenceTimeZone(preference) ? preference : "UTC"
}

export function referenceTimeZoneOptions(
  systemTimeZone: string | undefined
): readonly ReferenceTimeZoneOption[] {
  const resolvedSystemTimeZone = resolveReferenceTimeZonePreference(
    SYSTEM_REFERENCE_TIME_ZONE,
    systemTimeZone
  )
  const concreteOptions = concreteReferenceTimeZoneOptions()
  return [
    {
      value: SYSTEM_REFERENCE_TIME_ZONE,
      label: `System default (${resolvedSystemTimeZone})`
    },
    ...concreteOptions.map((timeZone) => ({
      value: timeZone,
      label: timeZone
    }))
  ]
}

function concreteReferenceTimeZoneOptions(): readonly string[] {
  const candidates = typeof Intl.supportedValuesOf === "function"
    ? Intl.supportedValuesOf("timeZone")
    : []
  const timeZones = [...candidates, ...FALLBACK_REFERENCE_TIME_ZONES]
  return Array.from(new Set(timeZones)).filter((timeZone) => isConcreteReferenceTimeZone(timeZone))
}

function isConcreteReferenceTimeZone(value: string | undefined): value is string {
  if (value === undefined || value.trim() !== value || value.length === 0) {
    return false
  }
  try {
    new Intl.DateTimeFormat("en-US", { timeZone: value })
    return true
  } catch (error: unknown) {
    if (error instanceof RangeError) {
      return false
    }
    throw error
  }
}
