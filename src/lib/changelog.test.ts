import { readFileSync } from "node:fs"
import { describe, expect, it } from "vitest"
import { CHANGELOG } from "@/lib/changelog"

describe("release metadata", () => {
  it("keeps package.json and the latest changelog version aligned", () => {
    const packageJson = JSON.parse(
      readFileSync(new URL("../../package.json", import.meta.url), "utf8"),
    ) as { version: string }

    expect(CHANGELOG[0]?.version).toBe(packageJson.version)
  })
})
