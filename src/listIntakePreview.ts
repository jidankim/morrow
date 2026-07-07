import type { ListIntakeCategoryRule, ListIntakeProfile } from "./domain/appConfig"

export type PreviewItem = {
  readonly name: string
  readonly quantity: string
  readonly category: string
}

export function previewExamples(profile: ListIntakeProfile): {
  readonly items: readonly PreviewItem[]
  readonly rejected: readonly string[]
} {
  const items = profile.positiveExamples.flatMap((example) =>
    parsePreviewItems(example, profile.categoryRules)
  )
  const unmatchedPositiveExamples = profile.positiveExamples
    .filter((example) => parsePreviewItems(example, profile.categoryRules).length === 0)
    .map((example) => `Unmatched positive example: ${example}`)
  const rejected = [
    ...unmatchedPositiveExamples,
    ...profile.negativeExamples.map((example) => `Rejected negative example: ${example}`)
  ]
  return { items, rejected }
}

function parsePreviewItems(
  example: string,
  categoryRules: readonly ListIntakeCategoryRule[]
): readonly PreviewItem[] {
  if (hasPrivateRiskText(example)) {
    return []
  }
  const itemPattern = /(?:^|[,/])\s*(?:(\d{1,3})\s+([a-z][a-z -]{1,79})|([a-z][a-z -]{1,79})\s*-\s*(\d{1,3}))/gi
  const items: PreviewItem[] = []
  for (const match of example.matchAll(itemPattern)) {
    const quantity = match[1] ?? match[4]
    const rawName = match[2] ?? match[3]
    if (quantity !== undefined && rawName !== undefined) {
      const name = rawName.trim()
      items.push({ name, quantity, category: categoryForItem(name, categoryRules) })
    }
  }
  return items
}

function hasPrivateRiskText(value: string): boolean {
  return value
    .split(/[\s,;\n]+/)
    .some((token) => looksLikeEmail(token) || looksLikePhone(token))
}

function looksLikeEmail(token: string): boolean {
  const trimmed = token.replace(/^[^a-z0-9@.]+|[^a-z0-9@.]+$/gi, "")
  const [local, domain] = trimmed.split("@")
  return local !== undefined && local.length > 0 && domain !== undefined && domain.includes(".")
}

function looksLikePhone(token: string): boolean {
  const trimmed = token.replace(/^[^+0-9a-z-]+|[^+0-9a-z-]+$/gi, "")
  const normalized =
    trimmed.startsWith("tel:") || trimmed.startsWith("sms:") || trimmed.startsWith("imessage:")
      ? trimmed.slice(trimmed.indexOf(":") + 1)
      : trimmed
  if (/^\d{4}-\d{2}-\d{2}$/.test(normalized)) {
    return false
  }
  const digitCount = Array.from(normalized).filter((character) => /\d/.test(character)).length
  return digitCount >= 7 && /^[+\d(). -]+$/.test(normalized)
}

function categoryForItem(itemName: string, categoryRules: readonly ListIntakeCategoryRule[]): string {
  const normalizedName = itemName.toLocaleLowerCase()
  const match = categoryRules.find((rule) =>
    rule.keywords.some((keyword) => normalizedName.includes(keyword.toLocaleLowerCase()))
  )
  return match?.displayName ?? "Uncategorized"
}
