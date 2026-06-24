import { useEffect } from "react";
import { bubbleTranslate, hideBubble } from "../shared/tauri-api";
import "../styles/bubble.css";

/**
 * The floating "Dịch" button shown next to a fresh text selection. Clicking it
 * hands the captured selection (stored Rust-side on mouse-up) to the translate
 * pipeline. Esc dismisses it.
 */
export function BubbleView() {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") void hideBubble();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  return (
    <button className="bubble-btn" onClick={() => void bubbleTranslate()} title="Dịch">
      <span className="bubble-glyph" aria-hidden>
        🌐
      </span>
      Dịch
    </button>
  );
}
