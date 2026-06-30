import { AlertTriangle } from "lucide-react"

type StatusOnboardingWarningsProps = {
  readonly warnings: readonly string[]
}

export function StatusOnboardingWarnings({
  warnings
}: StatusOnboardingWarningsProps): JSX.Element | null {
  if (warnings.length === 0) {
    return null
  }

  return (
    <div className="warning-surface" role="alert">
      <AlertTriangle aria-hidden="true" size={17} />
      <div>
        <h3>Onboarding required</h3>
        {warnings.map((warning) => (
          <p key={warning}>{warning}</p>
        ))}
      </div>
    </div>
  )
}
