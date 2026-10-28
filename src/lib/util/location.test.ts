import { describe, expect, it } from "vitest";
import { crumbs, joinLocal, parentOf, sameDrive } from "./location";

describe("location", () => {
  it("joins and climbs local paths", () => {
    expect(joinLocal("/home/ana", "Pictures")).toBe("/home/ana/Pictures");
    expect(joinLocal("/", "etc")).toBe("/etc");
    expect(joinLocal("C:\\Users", "ana")).toBe("C:\\Users\\ana");
    expect(parentOf({ kind: "local", path: "/home/ana" })).toEqual({ kind: "local", path: "/home" });
    expect(parentOf({ kind: "local", path: "/" })).toBeNull();
    expect(parentOf({ kind: "local", path: "C:\\Users" })).toEqual({ kind: "local", path: "C:\\" });
  });

  it("climbs remote paths", () => {
    expect(parentOf({ kind: "remote", remote: "gd", path: "a/b" })).toEqual({ kind: "remote", remote: "gd", path: "a" });
    expect(parentOf({ kind: "remote", remote: "gd", path: "a" })).toEqual({ kind: "remote", remote: "gd", path: "" });
    expect(parentOf({ kind: "remote", remote: "gd", path: "" })).toBeNull();
  });

  it("tells drives apart", () => {
    expect(sameDrive({ kind: "local", path: "/a" }, { kind: "local", path: "/b" })).toBe(true);
    expect(sameDrive({ kind: "local", path: "C:\\a" }, { kind: "local", path: "D:\\a" })).toBe(false);
    expect(sameDrive({ kind: "remote", remote: "x", path: "" }, { kind: "local", path: "/" })).toBe(false);
  });

  it("anchors breadcrumbs at Home", () => {
    const out = crumbs({ kind: "local", path: "/home/ana/Pictures/2026" }, "Computer", { path: "/home/ana", label: "Home" });
    expect(out.map((c) => c.label)).toEqual(["Home", "Pictures", "2026"]);
    const outside = crumbs({ kind: "local", path: "/etc" }, "Computer", { path: "/home/ana", label: "Home" });
    expect(outside.map((c) => c.label)).toEqual(["Computer", "etc"]);
  });
});
