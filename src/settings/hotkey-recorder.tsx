import { useEffect, useState } from "react";
import { setHotkey } from "../shared/tauri-api";

/** Map a KeyboardEvent.code to an accelerator key token (letters/digits/F-keys). */
function keyFromCode(code: string): string | null {
  if (code.startsWith("Key")) return code.slice(3);
  if (code.startsWith("Digit")) return code.slice(5);
  if (/^F\d{1,2}$/.test(code)) return code;
  return null;
}

/** Records a hotkey combo and applies it live via `set_hotkey` (Rust validates). */
export function HotkeyRecorder({ initial }: { initial: string }) {
  const [accel, setAccel] = useState(initial);
  const [recording, setRecording] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Listen at the window level while recording. WKWebView (every Tauri window on
  // macOS) does NOT focus a <button> on click, so an element-level onKeyDown
  // would never fire — key events land on <body>.
  useEffect(() => {
    if (!recording) return;

    const save = async (combo: string) => {
      try {
        await setHotkey(combo);
        setAccel(combo);
        setError(null);
      } catch (err) {
        setError(String(err));
      }
    };

    const onKeyDown = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      if (e.key === "Escape") {
        setRecording(false);
        return;
      }
      const mods: string[] = [];
      if (e.metaKey) mods.push("Cmd");
      if (e.ctrlKey) mods.push("Ctrl");
      if (e.altKey) mods.push("Alt");
      if (e.shiftKey) mods.push("Shift");
      const key = keyFromCode(e.code);
      if (!key || mods.length === 0) return; // need ≥1 modifier + 1 key
      setRecording(false);
      void save([...mods, key].join("+"));
    };

    window.addEventListener("keydown", onKeyDown, true);
    return () => window.removeEventListener("keydown", onKeyDown, true);
  }, [recording]);

  const message =
    error === "invalid-accelerator"
      ? "Invalid combination — use ≥1 modifier + a letter/number/F-key."
      : error === "register-failed"
        ? "Couldn't register (maybe already in use). Previous hotkey kept."
        : null;

  return (
    <section className="setting-section">
      <label>Global hotkey</label>
      <button
        type="button"
        className="hotkey-field"
        onClick={() => {
          setRecording(true);
          setError(null);
        }}
      >
        {recording ? "Recording… press combo (Esc to cancel)" : accel}
      </button>
      {message && <p className="err">{message}</p>}
    </section>
  );
}
