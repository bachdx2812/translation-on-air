import { describe, it, expect } from "vitest";
import { hotkeyKeys } from "./hotkey";

describe("hotkeyKeys", () => {
  it("prints modifiers as macOS glyphs in menu order", () => {
    expect(hotkeyKeys("Cmd+Shift+T")).toEqual(["⇧", "⌘", "T"]);
    expect(hotkeyKeys("Shift+Alt+Ctrl+Cmd+F5")).toEqual(["⌃", "⌥", "⇧", "⌘", "F5"]);
  });

  it("keeps the key itself as-is", () => {
    expect(hotkeyKeys("Ctrl+1")).toEqual(["⌃", "1"]);
  });
});
