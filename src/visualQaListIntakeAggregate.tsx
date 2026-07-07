import { ListIntakeReviewView } from "./ListIntakeReviewView"
import { visualListIntakeReport } from "./visualQaListIntakeData"

export function AggregateReviewFixture(): JSX.Element {
  return (
    <ListIntakeReviewView
      state={{
        status: "loaded",
        report: { ...visualListIntakeReport, proposals: [] }
      }}
      onApproveEdited={noopAsync}
      onReject={noopAsync}
      onReload={noopAsync}
    />
  )
}

async function noopAsync(): Promise<void> {
  return undefined
}
