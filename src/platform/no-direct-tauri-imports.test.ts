import { readdirSync, readFileSync } from "node:fs"
import path from "node:path"
import { describe, expect, it } from "vitest"

const SRC = path.resolve(__dirname, "..")
const PLATFORM = path.resolve(__dirname)
const TAURI_IMPORT = /(?:from\s+|import\s*\(\s*)["']@tauri-apps\//

function sourceFiles(dir: string): string[] {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const full = path.join(dir, entry.name)
    if (entry.isDirectory()) return full === PLATFORM ? [] : sourceFiles(full)
    if (!/\.(ts|tsx)$/.test(entry.name) || /\.test\.tsx?$/.test(entry.name)) return []
    return [full]
  })
}

describe("platform seam", () => {
  it("keeps @tauri-apps imports inside src/platform", () => {
    const offenders = sourceFiles(SRC)
      .filter((file) => TAURI_IMPORT.test(readFileSync(file, "utf8")))
      .map((file) => path.relative(SRC, file))
    expect(offenders, "import shell capabilities from @/platform instead").toEqual([])
  })
})
