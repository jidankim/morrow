import { z } from "zod"

export type RuntimeIdentity =
  | {
      readonly displayName: string
      readonly bundleIdentifier: string
      readonly executablePath: string
      readonly settingsTargetPath: string
      readonly runtimeKind: "appBundle"
    }
  | {
      readonly displayName: string
      readonly bundleIdentifier: string
      readonly executablePath: string
      readonly settingsTargetPath: string
      readonly runtimeKind: "binary"
    }

const runtimeIdentityFields = {
  displayName: z.string().min(1),
  bundleIdentifier: z.string().min(1),
  executablePath: z.string().min(1),
  settingsTargetPath: z.string().min(1)
} as const

const runtimeIdentitySchema: z.ZodType<RuntimeIdentity> = z.discriminatedUnion("runtimeKind", [
  z
    .object({
      ...runtimeIdentityFields,
      runtimeKind: z.literal("appBundle")
    })
    .strict(),
  z
    .object({
      ...runtimeIdentityFields,
      runtimeKind: z.literal("binary")
    })
    .strict()
])

export function parseRuntimeIdentity(value: unknown): RuntimeIdentity {
  return runtimeIdentitySchema.parse(value)
}
