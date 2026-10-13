// macOS modifier glyphs, listed in the order macOS menus print them (⌃⌥⇧⌘).
const MODIFIERS: [token: string, glyph: string][] = [
  ["Ctrl", "⌃"],
  ["Alt", "⌥"],
  ["Shift", "⇧"],
  ["Cmd", "⌘"],
];

/** Split an accelerator such as "Cmd+Shift+T" into macOS keycap labels
 * (["⇧", "⌘", "T"]), modifiers in menu order. */
export function hotkeyKeys(accel: string): string[] {
  const parts = accel.split("+");
  const mods = MODIFIERS.filter(([token]) => parts.includes(token)).map(([, glyph]) => glyph);
  const keys = parts.filter((p) => !MODIFIERS.some(([token]) => token === p));
  return [...mods, ...keys];
}
