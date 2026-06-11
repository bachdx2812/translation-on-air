import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  getSettings,
  hidePopup,
  openAccessibilitySettings,
  resizePopup,
  showSettings,
  type CaptureDonePayload,
  type CaptureErrorCode,
  type CaptureErrorPayload,
} from "../shared/tauri-api";
import type { TargetLang } from "../shared/types";
import { useTranslation } from "./use-translation";
import { FuriganaText } from "./furigana-text";
import { LangSwitcher } from "./lang-switcher";
import { ResultActions } from "./result-actions";
import "../styles/popup.css";

// phase 06 loads the persisted default; until then Vietnamese (the product default).
const DEFAULT_LANG: TargetLang = "vi";
const POPUP_WIDTH = 680;

const ERROR_MESSAGES: Record<string, string> = {
  "not-configured": "No provider configured — add an OpenAI key in Settings (or sign in to Claude).",
  "invalid-key": "OpenAI key rejected. Check it in Settings.",
  "rate-limited": "Rate limited — try again in a moment.",
  "quota-exhausted": "Claude credit exhausted — switch provider in Settings.",
  unavailable: "Translation service unavailable.",
  "bad-response": "Couldn't read the translation. Try again.",
  timeout: "Translation timed out.",
};

export function PopupView() {
  const [source, setSource] = useState("");
  const [targetLang, setTargetLang] = useState<TargetLang>(DEFAULT_LANG);
  const [captureError, setCaptureError] = useState<CaptureErrorCode | null>(null);
  const { state, run } = useTranslation();
  const rootRef = useRef<HTMLDivElement>(null);

  // Keep latest target lang in a ref so the (subscribe-once) capture listener
  // translates into the current language without re-subscribing.
  const targetLangRef = useRef(targetLang);
  useEffect(() => {
    targetLangRef.current = targetLang;
  }, [targetLang]);

  // Load persisted default target language on mount.
  useEffect(() => {
    void getSettings()
      .then((s) => setTargetLang(s.target_lang))
      .catch(() => {});
  }, []);

  // ESC hides the popup.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") void hidePopup();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  // Capture events from the Rust hotkey pipeline (mount once).
  useEffect(() => {
    const unlistens: Array<() => void> = [];
    void (async () => {
      unlistens.push(
        await listen<CaptureDonePayload>("capture-done", async (e) => {
          setCaptureError(null);
          setSource(e.payload.text);
          let lang = targetLangRef.current;
          try {
            lang = (await getSettings()).target_lang;
          } catch {
            // keep current lang on failure
          }
          setTargetLang(lang);
          void run(e.payload.text, lang);
        }),
      );
      unlistens.push(
        await listen<CaptureErrorPayload>("capture-error", (e) => {
          setCaptureError(e.payload.code);
        }),
      );
    })();
    return () => unlistens.forEach((u) => u());
  }, [run]);

  // Auto-size the window to fit content (no internal scroll until very tall).
  useLayoutEffect(() => {
    const el = rootRef.current;
    if (!el) return;
    const maxH = Math.floor(window.screen.availHeight * 0.85);
    const height = Math.max(150, Math.min(Math.ceil(el.scrollHeight), maxH));
    void resizePopup(POPUP_WIDTH, height);
  }, [state, source, captureError, targetLang]);

  const onLangChange = (lang: TargetLang) => {
    setTargetLang(lang);
    if (source) void run(source, lang);
  };

  const sourceSegments = state.status === "result" ? state.result.source_segments : [];

  return (
    <div className="popup" ref={rootRef}>
      <button
        className="gear"
        onClick={() => void showSettings()}
        title="Settings"
        aria-label="Settings"
      >
        ⚙
      </button>

      {captureError ? (
        <CaptureErrorView code={captureError} />
      ) : (
        <>
          <LangSwitcher value={targetLang} onChange={onLangChange} />
          <div className="panels">
            <section className="panel">
              <span className="panel-label">Gốc</span>
              {sourceSegments.length > 0 ? (
                <FuriganaText segments={sourceSegments} />
              ) : (
                <p className="src-text">{source || "…"}</p>
              )}
            </section>
            <section className="panel">
              <span className="panel-label">Bản dịch</span>
              <TranslationBody state={state} onRetry={() => source && run(source, targetLang)} />
            </section>
          </div>
        </>
      )}

      <p className="esc-hint">ESC</p>
    </div>
  );
}

function TranslationBody({
  state,
  onRetry,
}: {
  state: ReturnType<typeof useTranslation>["state"];
  onRetry: () => void;
}) {
  switch (state.status) {
    case "idle":
      return <p className="hint">…</p>;
    case "loading":
      return <p className="loading">Translating…</p>;
    case "result":
      return (
        <div className="result">
          {state.result.segments.length > 0 ? (
            <FuriganaText segments={state.result.segments} />
          ) : (
            <p className="translation">{state.result.translation}</p>
          )}
          <ResultActions text={state.result.translation} />
        </div>
      );
    case "error":
      return (
        <div className="error">
          <p>{ERROR_MESSAGES[state.code] ?? "Translation failed."}</p>
          <button onClick={onRetry}>Retry</button>
        </div>
      );
  }
}

function CaptureErrorView({ code }: { code: CaptureErrorCode }) {
  if (code === "ax-missing") {
    return (
      <div className="error">
        <p>Accessibility permission is required to read the selected text.</p>
        <button onClick={() => void openAccessibilitySettings()}>Open System Settings</button>
        <button onClick={() => void showSettings()}>Settings</button>
      </div>
    );
  }
  if (code === "no-selection") {
    return <p className="hint">Select text first, then press the hotkey.</p>;
  }
  return <p className="error">Couldn't capture the selection. Try again.</p>;
}
