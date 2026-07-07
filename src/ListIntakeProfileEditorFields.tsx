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
  const firstRule = categoryRules[0] ?? { categoryId: "seafood", displayName: "Seafood", keywords: [] }
  return (
    <fieldset className="option-group">
      <legend>Keyword category rules</legend>
      <label className="field">
        <span>Category name</span>
        <input
          value={firstRule.displayName}
          onChange={(event) =>
            onUpdate([
              {
                ...firstRule,
                displayName: event.currentTarget.value,
                categoryId: categoryIdFromName(event.currentTarget.value)
              }
            ])
          }
        />
      </label>
      <label className="field">
        <span>Category keywords</span>
        <input
          value={firstRule.keywords.join(", ")}
          onChange={(event) => onUpdate([{ ...firstRule, keywords: commaValues(event.currentTarget.value) }])}
        />
      </label>
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

function categoryIdFromName(name: string): string {
  const normalized = name
    .toLocaleLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
  return normalized.length === 0 ? "category" : normalized.slice(0, 48)
}
