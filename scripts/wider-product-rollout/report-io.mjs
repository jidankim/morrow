import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";

export function parseArgs(argv) {
  const [mode, ...rest] = argv;
  const args = { mode };

  for (let index = 0; index < rest.length; index += 1) {
    const token = rest[index];
    if (!token.startsWith("--")) {
      throw new Error(`unexpected positional argument: ${token}`);
    }

    const key = token.slice(2).replaceAll("-", "_");
    const value = rest[index + 1];
    if (value === undefined || value.startsWith("--")) {
      throw new Error(`missing value for --${token.slice(2)}`);
    }
    args[key] = value;
    index += 1;
  }

  return args;
}

export async function readJson(filePath) {
  const source = await readFile(filePath, "utf8");
  return JSON.parse(source);
}

export function buildInvocation() {
  return `node ${process.argv
    .slice(1)
    .map((value) => {
      if (path.isAbsolute(value)) {
        const relative = path.relative(process.cwd(), value);
        return relative.startsWith("..") ? path.basename(value) : relative;
      }
      return value;
    })
    .map((value) => JSON.stringify(value))
    .join(" ")}`;
}

export async function writeReport(outDir, fileName, report) {
  await mkdir(outDir, { recursive: true });
  const reportPath = path.join(outDir, fileName);
  await writeFile(reportPath, `${JSON.stringify(report, null, 2)}\n`, "utf8");
  return reportPath;
}
