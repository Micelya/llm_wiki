import { describe, expect, it } from "vitest"
import { tokenizeQuery } from "./search"

describe("tokenizeQuery", () => {
  it("drops stop words and short tokens", () => {
    expect(tokenizeQuery("what is the contract of a lease")).toEqual(["contract", "lease"])
  })

  it("keeps CJK tokenization behavior", () => {
    const tokens = tokenizeQuery("默会知识")
    expect(tokens).toContain("默会")
    expect(tokens).toContain("知识")
    expect(tokens).toContain("默")
  })
})
