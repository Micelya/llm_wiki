/**
 * Tests for `resolveMarkdownImageSrc`.
 *
 * Tauri's `convertFileSrc` is mocked to a deterministic identity
 * wrapper so we can assert the path that goes IN, not the actual
 * Tauri-side URL shape (which differs across platforms — `asset://`
 * on macOS, `https://asset.localhost/` on Windows, etc.).
 */
import { describe, it, expect, vi } from "vitest"

vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (path: string) => `tauri-asset:${path}`,
}))

import { resolveMarkdownImageSrc } from "./markdown-image-resolver"

describe("resolveMarkdownImageSrc", () => {
  const ROOT = "/Users/me/agent"

  it("passes http(s), data:, blob: and tauri: srcs through unchanged", () => {
    expect(resolveMarkdownImageSrc("https://example.com/img.png", ROOT)).toBe("https://example.com/img.png")
    expect(resolveMarkdownImageSrc("http://insecure.test/x.png", ROOT)).toBe("http://insecure.test/x.png")
    const data = "data:image/png;base64,iVBORw0K..."
    expect(resolveMarkdownImageSrc(data, ROOT)).toBe(data)
    expect(resolveMarkdownImageSrc("blob:abc-123", ROOT)).toBe("blob:abc-123")
    expect(resolveMarkdownImageSrc("tauri://asset/foo.png", ROOT)).toBe("tauri://asset/foo.png")
  })

  it("resolves a relative src against the root when no file directory is known", () => {
    expect(resolveMarkdownImageSrc("agent-workspace/chart.png", ROOT)).toBe(
      "tauri-asset:/Users/me/agent/agent-workspace/chart.png",
    )
    expect(resolveMarkdownImageSrc("./agent-workspace/chart.png", ROOT)).toBe(
      "tauri-asset:/Users/me/agent/agent-workspace/chart.png",
    )
  })

  it("converts an absolute path only when it stays inside the root", () => {
    expect(resolveMarkdownImageSrc("/Users/me/agent/agent-workspace/x.png", ROOT)).toBe(
      "tauri-asset:/Users/me/agent/agent-workspace/x.png",
    )
    expect(resolveMarkdownImageSrc("/var/data/screenshot.png", ROOT)).toBe("/var/data/screenshot.png")
    const escaping = "/Users/me/agent/../../../etc/passwd.png"
    expect(resolveMarkdownImageSrc(escaping, ROOT)).toBe(escaping)
    const sibling = "/Users/me/agentSecret/screenshot.png"
    expect(resolveMarkdownImageSrc(sibling, ROOT)).toBe(sibling)
  })

  it("decodes percent-encoded CJK in absolute paths and in the root", () => {
    expect(resolveMarkdownImageSrc("/Users/me/agent/%E4%B8%AD%E6%96%87/x.png", ROOT)).toBe(
      "tauri-asset:/Users/me/agent/中文/x.png",
    )
    expect(resolveMarkdownImageSrc("/Users/me/%E6%88%91%E7%9A%84/x.png", "/Users/me/我的")).toBe(
      "tauri-asset:/Users/me/我的/x.png",
    )
  })

  it("handles Windows drive letters, backslashes and UNC paths", () => {
    expect(resolveMarkdownImageSrc("C:/Users/me/agent/out/x.png", "C:/Users/me/agent")).toBe(
      "tauri-asset:C:/Users/me/agent/out/x.png",
    )
    expect(resolveMarkdownImageSrc("C:/Users/me/Pictures/x.png", "C:/Users/me/agent")).toBe(
      "C:/Users/me/Pictures/x.png",
    )
    expect(resolveMarkdownImageSrc("out/y.png", "C:\\Users\\me\\agent")).toBe(
      "tauri-asset:C:/Users/me/agent/out/y.png",
    )
    expect(resolveMarkdownImageSrc("\\\\share\\folder\\img.png", ROOT)).toBe("\\\\share\\folder\\img.png")
  })

  it("returns the raw src unchanged without a root, and an empty src verbatim", () => {
    expect(resolveMarkdownImageSrc("out/img.png", null)).toBe("out/img.png")
    expect(resolveMarkdownImageSrc("", ROOT)).toBe("")
  })

  describe("file-relative resolution (currentFileDir)", () => {
    it("resolves ../assets against the file's own directory (Obsidian-style)", () => {
      expect(resolveMarkdownImageSrc("../assets/img.png", ROOT, `${ROOT}/agent-workspace/report`)).toBe(
        "tauri-asset:/Users/me/agent/agent-workspace/assets/img.png",
      )
    })

    it("resolves a same-directory path, with or without ./", () => {
      expect(resolveMarkdownImageSrc("img.png", ROOT, `${ROOT}/agent-workspace`)).toBe(
        "tauri-asset:/Users/me/agent/agent-workspace/img.png",
      )
      expect(resolveMarkdownImageSrc("./img.png", ROOT, `${ROOT}/agent-workspace`)).toBe(
        "tauri-asset:/Users/me/agent/agent-workspace/img.png",
      )
    })

    it("accepts a root-relative directory and backslashes", () => {
      expect(resolveMarkdownImageSrc("../assets/x.png", ROOT, "agent-workspace/report")).toBe(
        "tauri-asset:/Users/me/agent/agent-workspace/assets/x.png",
      )
      expect(
        resolveMarkdownImageSrc("../assets/x.png", "C:\\Users\\me\\agent", "C:\\Users\\me\\agent\\out\\report"),
      ).toBe("tauri-asset:C:/Users/me/agent/out/assets/x.png")
    })

    it("does not convert a path that escapes the root", () => {
      expect(resolveMarkdownImageSrc("../../../../etc/x.png", ROOT, `${ROOT}/agent-workspace`)).toBe(
        "../../../../etc/x.png",
      )
      expect(resolveMarkdownImageSrc("x.png", ROOT, "/var/elsewhere")).toBe("x.png")
    })

    it("decodes percent-encoded CJK and keeps a malformed percent sequence", () => {
      expect(resolveMarkdownImageSrc("%E5%9B%BE%E7%89%87.png", ROOT, `${ROOT}/agent-workspace`)).toBe(
        "tauri-asset:/Users/me/agent/agent-workspace/图片.png",
      )
      expect(resolveMarkdownImageSrc("100%-done.png", ROOT, `${ROOT}/agent-workspace`)).toBe(
        "tauri-asset:/Users/me/agent/agent-workspace/100%-done.png",
      )
    })
  })
})
