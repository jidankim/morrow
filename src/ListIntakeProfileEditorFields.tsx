import type { ListIntakeCategoryRule } from "./domain/appConfig"
import type { PreviewItem } from "./listIntakePreview"

type ExampleEditorProps = {
  readonly label: "Positive examples" | "Negative examples"
  readonly values: readonly string[]
  readonly onUpdate: (values: readonly string[]) => void
}

const DEFAULT_POSITIVE_EXAMPLE = "2 anchovies, 3 salmon"
const DEFAULT_NEGATIVE_EXAMPLE = "remind me to buy fish tomorrow"

export function ExampleEditor({ label, values, onUpdate }: ExampleEditorProps): JSX.Element {
  const fallbackExample = label === "Positive examples" ? DEFAULT_POSITIVE_EXAMPLE : DEFAULT_NEGATIVE_EXAMPLE
  return (
    <label className="field">
      <span>{label}</span>
      <textarea
        rows={Math.max(3, values.length)}
        value={values.join("\n")}
        onChange={(event) => onUpdate(linesFromText(event.currentTarget.value))}
      />
      <span className="field-actions">
        <button className="button secondary" onClick={() => onUpdate([...values, fallbackExample])} type="button">
          Add {label === "Positive examples" ? "positive" : "negative"} example
        </button>
        {values.length > 0 ? (
          <button className="button secondary" onClick={() => onUpdate(values.slice(1))} type="button">
            Delete {label === "Positive examples" ? "positive" : "negative"} example 1
          </button>
        ) : null}
      </span>
    </label>
  )
}

export function CategoryEditor({
  categoryRules,
  onUpdate
}: {
  readonly categoryRules: readonly ListIntakeCategoryRule[]
  readonly onUpdate: (rules: readonly ListIntakeCategoryRule[]) => void
}): JSX.Element {
  return (
    <fieldset className="option-group">
      <legend>Keyword category rules</legend>
      {categoryRules.length === 0 ? <p className="settings-note">No keyword categories yet.</p> : null}
      {categoryRules.map((rule, index) => (
        <div className="category-rule-editor" key={`category-rule-${index.toString()}`}>
          <label className="field">
            <span>{`Category ${(index + 1).toString()} name`}</span>
            <input
              value={rule.displayName}
              onChange={(event) =>
                onUpdate(updateCategoryRule(categoryRules, index, categoryNamePatch(categoryRules, index, event.currentTarget.value)))
              }
            />
          </label>
          <label className="field">
            <span>{`Category ${(index + 1).toString()} keywords`}</span>
            <input
              value={rule.keywords.join(", ")}
              onChange={(event) =>
                onUpdate(updateCategoryRule(categoryRules, index, { keywords: commaValues(event.currentTarget.value) }))
              }
            />
          </label>
          <button
            className="button secondary"
            onClick={() => onUpdate(categoryRules.filter((_rule, ruleIndex) => ruleIndex !== index))}
            type="button"
          >
            {`Delete category ${(index + 1).toString()}`}
          </button>
        </div>
      ))}
      <button
        className="button secondary"
        disabled={categoryRules.length >= 20}
        onClick={() => onUpdate([...categoryRules, nextCategoryRule(categoryRules)])}
        type="button"
      >
        Add category
      </button>
    </fieldset>
  )
}

export function PreviewPanel({
  items,
  rejected
}: {
  readonly items: readonly PreviewItem[]
  readonly rejected: readonly string[]
}): JSX.Element {
  return (
    <div className="result-surface list-intake-preview">
      <h4>Constrained extraction preview</h4>
      <p>Aggregation window: Local day in reference timezone.</p>
      <p>Confidence behavior: 850+ aggregates automatically; 550-849 is Needs review.</p>
      <table>
        <caption>Matched item rows</caption>
        <thead>
          <tr>
            <th>Item</th>
            <th>Quantity</th>
            <th>Category</th>
          </tr>
        </thead>
        <tbody>
          {items.length === 0 ? (
            <tr>
              <td colSpan={3}>No matched rows yet.</td>
            </tr>
          ) : (
            items.map((item, index) => (
              <tr key={`${item.name}-${item.quantity}-${index.toString()}`}>
                <td>{item.name}</td>
                <td>{item.quantity}</td>
                <td>{item.category}</td>
              </tr>
            ))
          )}
        </tbody>
      </table>
      <p>Rejected examples/errors</p>
      <ul>
        {rejected.length === 0 ? (
          <li>No rejected examples.</li>
        ) : (
          rejected.map((reason) => <li key={reason}>{reason}</li>)
        )}
      </ul>
    </div>
  )
}

function linesFromText(value: string): readonly string[] {
  return value
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line.length > 0)
}

function commaValues(value: string): readonly string[] {
  return value
    .split(",")
    .map((keyword) => keyword.trim())
    .filter((keyword) => keyword.length > 0)
}

function updateCategoryRule(
  rules: readonly ListIntakeCategoryRule[],
  index: number,
  patch: Partial<ListIntakeCategoryRule>
): readonly ListIntakeCategoryRule[] {
  return rules.map((rule, ruleIndex) => (ruleIndex === index ? { ...rule, ...patch } : rule))
}

function categoryNamePatch(
  rules: readonly ListIntakeCategoryRule[],
  index: number,
  displayName: string
): Pick<ListIntakeCategoryRule, "categoryId" | "displayName"> {
  return {
    categoryId: uniqueCategoryId(categoryIdFromName(displayName), rules, index),
    displayName
  }
}

function nextCategoryRule(rules: readonly ListIntakeCategoryRule[]): ListIntakeCategoryRule {
  const displayName = `Category ${(rules.length + 1).toString()}`
  return {
    categoryId: uniqueCategoryId(categoryIdFromName(displayName), rules, undefined),
    displayName,
    keywords: []
  }
}

function uniqueCategoryId(
  baseCategoryId: string,
  rules: readonly ListIntakeCategoryRule[],
  currentIndex: number | undefined
): string {
  const usedCategoryIds = new Set(
    rules.flatMap((rule, index) => (currentIndex === index ? [] : [rule.categoryId]))
  )
  if (!usedCategoryIds.has(baseCategoryId)) {
    return baseCategoryId
  }
  for (let suffix = 2; suffix <= 20; suffix += 1) {
    const suffixText = `-${suffix.toString()}`
    const candidate = `${baseCategoryId.slice(0, 48 - suffixText.length)}${suffixText}`
    if (!usedCategoryIds.has(candidate)) {
      return candidate
    }
  }
  return baseCategoryId
}

function categoryIdFromName(name: string): string {
  const normalized = name
    .toLocaleLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
  return normalized.length === 0 ? "category" : normalized.slice(0, 48)
}
