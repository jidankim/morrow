import { z } from "zod"

const codexSetupActionStatusSchema = z.union([
  z.literal("installed"),
  z.literal("alreadyInstalled"),
  z.literal("alreadyRunning"),
  z.literal("failed"),
  z.literal("timeout"),
  z.literal("unsupported")
])

const codexLoginLaunchStatusSchema = z.union([
  z.literal("launched"),
  z.literal("alreadyRunning"),
  z.literal("missingCli"),
  z.literal("failedToStart")
])

const codexSetupReceiptShape = {
  commandSurface: z.string().min(1),
  commandOutputRedacted: z.literal(true),
  diagnostic: z.string().min(1)
} as const

const codexCliInstallReceiptSchema = z.object({
  status: codexSetupActionStatusSchema,
  ...codexSetupReceiptShape
})

const codexLoginLaunchReceiptSchema = z.object({
  status: codexLoginLaunchStatusSchema,
  ...codexSetupReceiptShape
})

export type CodexSetupActionStatus = z.infer<typeof codexSetupActionStatusSchema>
export type CodexLoginLaunchStatus = z.infer<typeof codexLoginLaunchStatusSchema>
export type CodexCliInstallReceipt = z.infer<typeof codexCliInstallReceiptSchema>
export type CodexLoginLaunchReceipt = z.infer<typeof codexLoginLaunchReceiptSchema>

export function parseCodexCliInstallReceipt(value: unknown): CodexCliInstallReceipt {
  return codexCliInstallReceiptSchema.parse(value)
}

export function parseCodexLoginLaunchReceipt(value: unknown): CodexLoginLaunchReceipt {
  return codexLoginLaunchReceiptSchema.parse(value)
}
