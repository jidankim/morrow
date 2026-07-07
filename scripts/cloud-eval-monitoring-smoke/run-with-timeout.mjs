#!/usr/bin/env node
import { spawn } from "node:child_process";

const [timeoutRaw, ...cmd] = process.argv.slice(2);
const timeoutSeconds = Number(timeoutRaw);
if (!Number.isFinite(timeoutSeconds) || timeoutSeconds < 1 || cmd.length === 0) {
  process.exit(125);
}

const child = spawn(cmd[0], cmd.slice(1), { stdio: "inherit" });
let completed = false;
const timer = setTimeout(() => {
  if (completed) return;
  console.error(`timeout_after_seconds: ${timeoutSeconds}`);
  child.kill("SIGTERM");
  setTimeout(() => child.kill("SIGKILL"), 1000).unref();
}, timeoutSeconds * 1000);

child.on("error", (error) => {
  completed = true;
  clearTimeout(timer);
  console.error(`spawn_error: ${error.message}`);
  process.exit(127);
});

child.on("exit", (code, signal) => {
  completed = true;
  clearTimeout(timer);
  console.error(`process_absence: ${child.kill(0) ? "FAIL" : "PASS"}`);
  if (signal) {
    console.error(`signal: ${signal}`);
    process.exit(124);
  }
  process.exit(code ?? 1);
});
