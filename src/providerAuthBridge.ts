import { z } from "zod"

const codexAuthStatusSchema = z.union([
  z.literal("loggedInUsingChatGpt"),
  z.literal("missingCli"),
  z.literal("notLoggedIn"),
  z.literal("timeout"),
  z.literal("unknownFailure")
])

const codexProviderAuthReadinessSchema = z.object({
  status: codexAuthStatusSchema,
  ready: z.boolean(),
  commandSurface: z.string().min(1),
  commandOutputRedacted: z.boolean(),
  diagnostic: z.string().min(1)
})

export type CodexAuthStatus = z.infer<typeof codexAuthStatusSchema>
export type CodexProviderAuthReadiness = z.infer<typeof codexProviderAuthReadinessSchema>

export function parseCodexProviderAuthReadiness(value: unknown): CodexProviderAuthReadiness {
  return codexProviderAuthReadinessSchema.parse(value)
}
