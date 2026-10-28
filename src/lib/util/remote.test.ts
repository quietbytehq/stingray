import { describe, expect, it } from "vitest";
import { isValidRemoteName } from "./remote";

describe("isValidRemoteName", () => {
  it("accepts simple names and rejects the rest", () => {
    expect(isValidRemoteName("Work_Drive-2")).toBe(true);
    expect(isValidRemoteName("  padded  ")).toBe(true);
    expect(isValidRemoteName("")).toBe(false);
    expect(isValidRemoteName("has space")).toBe(false);
    expect(isValidRemoteName("x".repeat(41))).toBe(false);
  });
});
