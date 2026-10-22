import { describe, expect, it } from "vitest"
import config from "../../vite.config"

type TestConfig = { test?: { exclude?: string[] } }

describe("what the tests of the app go over", () => {
  it("leaves out the leftovers of the old MCP server and of the Rust build", async () => {
    const resolved = await (config as unknown as (env: { command: string; mode: string }) => Promise<TestConfig>)({
      command: "serve",
      mode: "test",
    })
    const exclude = resolved.test?.exclude ?? []
    expect(exclude).toContain("**/mcp-server/**")
    expect(exclude).toContain("**/src-tauri/target/**")
    // The usual ones stay.
    expect(exclude).toContain("**/node_modules/**")
  })
})
