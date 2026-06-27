export function isValidTimeZone(value: string): boolean {
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
