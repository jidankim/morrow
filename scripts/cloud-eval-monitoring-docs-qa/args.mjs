const allowedArgs = new Set(["--smoke-dir", "--fixture-overclaim"])

export function parseArgs(argv) {
  const args = new Map()
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index]
    if (!arg.startsWith("--")) throw new Error(`unexpected positional argument: ${arg}`)
    if (!allowedArgs.has(arg)) throw new Error(`unknown argument: ${arg}`)
    const value = argv[index + 1]
    if (!value || value.startsWith("--")) throw new Error(`missing value for ${arg}`)
    args.set(arg, value)
    index += 1
  }
  const smokeDir = args.get("--smoke-dir")
  if (!smokeDir) throw new Error("required argument missing: --smoke-dir")
  return {
    smokeDir,
    fixtureOverclaimPath: args.get("--fixture-overclaim") ?? null
  }
}
