import { z } from "zod"

const tokenStorageSurfaceSchema = z.literal("keychainBridge")

const tokenCommandReceiptSchema = z.object({
  storageSurface: tokenStorageSurfaceSchema,
  stored: z.boolean(),
  deleted: z.boolean()
})

const tokenReadResponseSchema = z.object({
  storageSurface: tokenStorageSurfaceSchema,
  present: z.boolean(),
  token: z.string().min(1).optional()
})

export const MORROW_KEYCHAIN_SERVICE = "com.morrow.desktop.token"
export const MORROW_TOKEN_KIND = "morrow-owned-token"
export const MORROW_PROVIDER_TOKEN_KIND = "morrow-openai-provider-api-key"

export type MorrowTokenKind = typeof MORROW_TOKEN_KIND | typeof MORROW_PROVIDER_TOKEN_KIND

export type MorrowTokenLookupRequest = {
  readonly service: typeof MORROW_KEYCHAIN_SERVICE
  readonly tokenKind: MorrowTokenKind
}

export type MorrowTokenWriteRequest = MorrowTokenLookupRequest & {
  readonly token: string
}

export type MorrowTokenCommandReceipt = z.infer<typeof tokenCommandReceiptSchema>
export type MorrowTokenReadResponse = z.infer<typeof tokenReadResponseSchema>

export function parseTokenCommandReceipt(value: unknown): MorrowTokenCommandReceipt {
  return tokenCommandReceiptSchema.parse(value)
}

export function parseTokenReadResponse(value: unknown): MorrowTokenReadResponse {
  return tokenReadResponseSchema.parse(value)
}
