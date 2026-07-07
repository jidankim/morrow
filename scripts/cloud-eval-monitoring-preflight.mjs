#!/usr/bin/env node

import { main } from "./cloud-eval-monitoring-preflight/main.mjs";

try {
  main();
} catch (error) {
  const message = error instanceof Error ? error.message : String(error);
  console.error(`FAIL phase 6 cloud eval monitoring preflight: ${message}`);
  process.exit(1);
}
