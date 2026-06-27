import { z } from "zod"

const permissionKindSchema = z.union([
  z.literal("fullDiskAccess"),
  z.literal("calendar"),
  z.literal("reminders"),
  z.literal("contacts"),
  z.literal("notifications"),
  z.literal("filesystemPath")
])

const permissionStateSchema = z.union([
  z.literal("granted"),
  z.literal("denied"),
  z.literal("unavailable")
])

const permissionOutcomeSchema = z.union([
  z.literal("success"),
  z.literal("warning"),
  z.literal("unavailable")
])

const nativePermissionStatusSchema = z.object({
  kind: permissionKindSchema,
  state: permissionStateSchema,
  outcome: permissionOutcomeSchema,
  warning: z.string().min(1).optional()
})

export type NativePermissionStatus = z.infer<typeof nativePermissionStatusSchema>

export function parseNativePermissionStatuses(
  value: unknown
): readonly NativePermissionStatus[] {
  return z.array(nativePermissionStatusSchema).parse(value)
}
