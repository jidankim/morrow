#!/usr/bin/env node

import { parseArgs } from "./cloud-eval-monitoring-docs-qa/args.mjs"
import { runDocsQa } from "./cloud-eval-monitoring-docs-qa/run.mjs"

async function main() {
  process.exitCode = await runDocsQa(parseArgs(process.argv.slice(2)))
}

main().catch((error) => {
  const message = error instanceof Error ? error.message : "unknown docs QA failure"
  console.error(`FAIL cloud eval monitoring docs QA: ${message}`)
  process.exitCode = 1
})
