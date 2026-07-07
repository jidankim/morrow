import { readFile } from "node:fs/promises"
import { join } from "node:path"
import {
  docs,
  forbiddenPrivacyPatterns,
  overclaimPatterns,
  phase6SmokeCommand,
  phase6SmokeCommandForOutDir,
  requiredDocsPhrases,
  requiredSmokeArtifacts,
  staleFutureWorkPatterns
} from "./config.mjs"
import {
  cleanupFailures,
  gateFailures,
  negativeMatrixFailures,
  parseJson,
  patternFailures,
  reportFailures,
  summaryFailures
} from "./checks.mjs"
import { evidenceFailures } from "./evidence.mjs"

async function readInputs(smokeDir, fixtureOverclaimPath) {
  const [summaryText, dashboardReportText, releaseGateText, negativeMatrixText, cleanupText, fixtureText, ...docTexts] =
    await Promise.all([
      readFile(join(smokeDir, "summary.txt"), "utf8"),
      readFile(join(smokeDir, "cloud-eval-monitoring-report.json"), "utf8"),
      readFile(join(smokeDir, "release-gate.json"), "utf8"),
      readFile(join(smokeDir, "negative-matrix.json"), "utf8"),
      readFile(join(smokeDir, "cleanup-receipt.txt"), "utf8"),
      fixtureOverclaimPath ? readFile(fixtureOverclaimPath, "utf8") : "",
      ...docs.map((doc) => readFile(doc, "utf8"))
    ])
  return { summaryText, dashboardReportText, releaseGateText, negativeMatrixText, cleanupText, fixtureText, docTexts }
}

function structuralFailures(inputs, expectedSmokeCommand) {
  return [
    ...summaryFailures(inputs.summaryText, expectedSmokeCommand),
    ...reportFailures(parseJson(inputs.dashboardReportText, "cloud-eval-monitoring-report.json")),
    ...gateFailures(parseJson(inputs.releaseGateText, "release-gate.json")),
    ...negativeMatrixFailures(parseJson(inputs.negativeMatrixText, "negative-matrix.json")),
    ...cleanupFailures(inputs.cleanupText)
  ]
}

function docsFailures(docTexts, fixtureText) {
  const docsText = docTexts.join("\n")
  const combinedText = `${docsText}\n${fixtureText}`
  return [
    ...requiredDocsPhrases.filter((phrase) => !docsText.includes(phrase)).map((phrase) => `missing required docs wording: ${phrase}`),
    ...patternFailures(combinedText, staleFutureWorkPatterns, "stale docs wording present"),
    ...patternFailures(combinedText, overclaimPatterns, "forbidden overclaim present"),
    ...patternFailures(combinedText, forbiddenPrivacyPatterns, "forbidden private content present")
  ]
}

export async function runDocsQa({ smokeDir, fixtureOverclaimPath }) {
  const inputs = await readInputs(smokeDir, fixtureOverclaimPath)
  const expectedSmokeCommand = phase6SmokeCommandForOutDir(smokeDir)
  const failures = [
    ...(await evidenceFailures(smokeDir)),
    ...structuralFailures(inputs, expectedSmokeCommand),
    ...docsFailures(inputs.docTexts, inputs.fixtureText)
  ]

  if (failures.length > 0) {
    console.error("FAIL cloud eval monitoring docs QA")
    console.error(failures.map((failure) => `- ${failure}`).join("\n"))
    return 1
  }

  console.log("PASS cloud eval monitoring docs QA")
  console.log(`docs: ${docs.join(", ")}`)
  console.log(`smoke: ${smokeDir}/summary.txt`)
  console.log(`command: ${phase6SmokeCommand}`)
  console.log(`artifacts: ${requiredSmokeArtifacts.join(", ")}, command-logs/`)
  return 0
}
