import { readdirSync, readFileSync } from "node:fs"
import path from "node:path"
import { describe, expect, it } from "vitest"

const SRC = path.resolve(__dirname, "..")
const TAURI_IMPORT = /(?:from\s+|import\s*\(\s*)["']@tauri-apps\//

function sourceFiles(dir: string): string[] {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const full = path.join(dir, entry.name)
    if (entry.isDirectory()) return sourceFiles(full)
    if (!/\.(ts|tsx)$/.test(entry.name) || /\.test\.tsx?$/.test(entry.name)) return []
    return [full]
  })
}

describe("server-only frontend", () => {
  it("has no @tauri-apps imports", () => {
    const offenders = sourceFiles(SRC)
      .filter((file) => TAURI_IMPORT.test(readFileSync(file, "utf8")))
      .map((file) => path.relative(SRC, file))
    expect(offenders, "the frontend is a plain web app; use @/platform").toEqual([])
  })
})
