import { describe, expect, it } from "vitest";
import { listKey, matches, typedCommand, type Entry } from "./skills";

const entry = (name: string): Entry => ({ name, args: "", description: "", command: false });
const names = (es: Entry[]) => es.map((e) => e.name);
const all = ["summary", "simplify", "explain", "translate", "persona", "help", "clear", "code"].map(entry);

describe("skill autocomplete", () => {
  it("SKL-2.1 lists prefix matches first, then fuzzy ones, each alphabetical", () => {
    expect(names(matches(all, "s"))).toEqual(["simplify", "summary", "persona", "translate"]);
    expect(names(matches(all, "sm"))).toEqual(["simplify", "summary"]);
    expect(names(matches(all, "co"))).toEqual(["code"]);
    expect(names(matches(all, "ae"))).toEqual(["translate"]);
    expect(names(matches(all, ""))).toEqual(["clear", "code", "explain", "help", "persona", "simplify", "summary", "translate"]);
    expect(matches(all, "zz")).toEqual([]);
  });

  it("SKL-2.1 opens on a slash and closes at the first space", () => {
    expect(typedCommand("/")).toBe("");
    expect(typedCommand("/Sum")).toBe("sum");
    expect(typedCommand("/summary ")).toBeNull();
    expect(typedCommand("what /is")).toBeNull();
  });

  it("SKL-2.1 ↑/↓ move and wrap, Tab or Enter complete, Esc closes", () => {
    expect(listKey("ArrowDown", 0, 3)).toBe(1);
    expect(listKey("ArrowDown", 2, 3)).toBe(0);
    expect(listKey("ArrowUp", 0, 3)).toBe(2);
    expect(listKey("Tab", 1, 3)).toBe("complete");
    expect(listKey("Enter", 1, 3)).toBe("complete");
    expect(listKey("Escape", 1, 3)).toBe("close");
    expect(listKey("a", 1, 3)).toBeNull();
    expect(listKey("Enter", 0, 1, true)).toBeNull(); // /help typed in full: Enter sends it
    expect(listKey("Tab", 0, 1, true)).toBe("complete");
  });
});
