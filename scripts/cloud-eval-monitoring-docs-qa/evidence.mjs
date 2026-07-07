import { readdir, stat } from "node:fs/promises"
import { join } from "node:path"
import { requiredSmokeArtifacts } from "./config.mjs"

async function assertNonEmptyFile(path) {
  const info = await stat(path)
  if (!info.isFile() || info.size === 0) throw new Error(`empty evidence artifact: ${path}`)
}

async function assertNonEmptyDirectory(path) {
  const info = await stat(path)
  if (!info.isDirectory()) throw new Error(`evidence artifact is not a directory: ${path}`)
  const entries = await readdir(path)
  if (entries.length === 0) throw new Error(`empty evidence artifact: ${path}`)
}

export async function evidenceFailures(smokeDir) {
  const failures = []
  for (const artifactName of requiredSmokeArtifacts) {
    const artifactPath = join(smokeDir, artifactName)
    try {
      await assertNonEmptyFile(artifactPath)
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      failures.push(message.includes("ENOENT") ? `missing evidence artifact: ${artifactPath}` : message)
    }
  }
  try {
    await assertNonEmptyDirectory(join(smokeDir, "command-logs"))
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error)
    failures.push(message.includes("ENOENT") ? `missing evidence artifact: ${join(smokeDir, "command-logs")}` : message)
  }
  return failures
}
