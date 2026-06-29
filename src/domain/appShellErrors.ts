export class UnhandledAppShellVariantError extends Error {
  readonly renderedValue: string

  constructor(renderedValue: string) {
    super(`Unhandled app shell variant: ${renderedValue}`)
    this.name = "UnhandledAppShellVariantError"
    this.renderedValue = renderedValue
  }
}
