import fs from "node:fs";
import path from "node:path";
import { validateInputFixture } from "./cloud-eval-monitoring-report/validator.mjs";

export const inputValidationName = "input-validation.json";
export { validateInputFixture };

export function die(message) {
  console.error(`error: ${message}`);
  process.exit(64);
}

export function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

export function writeJson(filePath, value) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`);
}

export function writeInputValidation(outDir, validation) {
  writeJson(path.join(outDir, inputValidationName), validation);
}
