#!/usr/bin/env node

import { main } from "./messages-calendar-approval-trajectory-preflight/main.mjs";

try {
  main();
} catch (error) {
  const message = error instanceof Error ? error.message : String(error);
  console.error(`FAIL phase 5 preflight: ${message}`);
  process.exit(1);
}
