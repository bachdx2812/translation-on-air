import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  getSettings,
  hidePopup,
  historySave,
  openAccessibilitySettings,
  requestAccessibility,
  resizePopup,
  showHistory,
  showSettings,
  translateReply,
  type CaptureDonePayload,
  type CaptureErrorCode,
  type CaptureErrorPayload,
} from "../shared/tauri-api";
import type { Conversation, HistoryTurn, Segment, TargetLang } from "../shared/types";
import { useTranslation } from "./use-translation";
import { FuriganaText } from "./furigana-text";
import { LangSwitcher } from "./lang-switcher";
import { ResultActions } from "./result-actions";
import { ReplyBox } from "./reply-box";
import "../styles/popup.css";

// phase 06 loads the persisted default; until then Vietnamese (the product default).
const DEFAULT_LANG: TargetLang = "vi";
const POPUP_WIDTH = 460;

const ERROR_MESSAGES: Record<string, string> = {
  "not-configured": "No provider configured — add an OpenAI key in Settings (or sign in to Claude).",
  "invalid-key": "OpenAI key rejected. Check it in Settings.",
  "rate-limited": "Rate limited — try again in a moment.",
  "quota-exhausted": "Claude credit exhausted — switch provider in Settings.",
  unavailable: "Translation service unavailable.",
  "bad-response": "Couldn't read the translation. Try again.",
  timeout: "Translation timed out.",
};

/** A reply turn: the user's reply (in the target lang) translated back into the
 * source language. */
type ReplyTurn = {
  input: string;
  status: "loading" | "done" | "error";
  output: string;
  segments: Segment[];
  code?: string;
};

export function PopupView() {
  const [source, setSource] = useState("");
  const [targetLang, setTargetLang] = useState<TargetLang>(DEFAULT_LANG);
  const [captureError, setCaptureError] = useState<CaptureErrorCode | null>(null);
  const { state, run } = useTranslation();
  const [replies, setReplies] = useState<ReplyTurn[]>([]);
  const [replyOpen, setReplyOpen] = useState(false);
  // Stable id/timestamp for the current conversation; reset on each capture.
  const convRef = useRef<{ id: string; createdAt: number } | null>(null);
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
          // New capture → start a fresh conversation thread.
          setReplies([]);
          setReplyOpen(false);
          convRef.current = { id: crypto.randomUUID(), createdAt: Date.now() };
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

  // Persist the conversation locally whenever the incoming result or replies
  // change (upsert by id, so language switches update the same entry).
  useEffect(() => {
    const conv = convRef.current;
    if (state.status !== "result" || !conv) return;
    const turns: HistoryTurn[] = [
      { kind: "incoming", input: source, output: state.result.translation, at: conv.createdAt },
      ...replies
        .filter((r) => r.status === "done")
        .map((r): HistoryTurn => ({ kind: "reply", input: r.input, output: r.output, at: conv.createdAt })),
    ];
    const conversation: Conversation = {
      id: conv.id,
      created_at: conv.createdAt,
      target_lang: targetLang,
      turns,
    };
    void historySave(conversation).catch(() => {});
  }, [state, replies, source, targetLang]);

  // Auto-size the window to the inner content + card padding, so the card fills
  // the window exactly (no window backing showing as a faint layer behind it).
  // Measuring the content wrapper (not the 100vh card) avoids a feedback loop.
  useLayoutEffect(() => {
    const el = rootRef.current;
    if (!el) return;
    const CARD_PADDING = 28; // .popup top+bottom padding (16 + 12)
    const maxH = Math.floor(window.screen.availHeight * 0.85);
    const height = Math.max(120, Math.min(Math.ceil(el.scrollHeight) + CARD_PADDING, maxH));
    void resizePopup(POPUP_WIDTH, height);
  }, [state, source, captureError, targetLang, replies, replyOpen]);

  const onLangChange = (lang: TargetLang) => {
    setTargetLang(lang);
    if (source) void run(source, lang);
  };

  const onReply = async (text: string) => {
    const trimmed = text.trim();
    if (!trimmed || state.status !== "result") return;
    const original = state.result.translation;
    const idx = replies.length;
    setReplies((rs) => [...rs, { input: trimmed, status: "loading", output: "", segments: [] }]);
    setReplyOpen(false);
    try {
      const res = await translateReply(trimmed, source, original, targetLang);
      setReplies((rs) =>
        rs.map((r, i) =>
          i === idx ? { ...r, status: "done", output: res.translation, segments: res.segments } : r,
        ),
      );
    } catch (e) {
      setReplies((rs) =>
        rs.map((r, i) => (i === idx ? { ...r, status: "error", code: String(e) } : r)),
      );
    }
  };

  const sourceSegments = state.status === "result" ? state.result.source_segments : [];

  return (
    // .popup fills the window (opaque rounded card); .popup-body holds the
    // measured content.
    <div className="popup">
      <div className="popup-body" ref={rootRef}>
        {captureError ? (
          <>
            <header className="popup-header">
              <span className="brand">Translate On Air</span>
              <HeaderActions />
            </header>
            <CaptureErrorView code={captureError} />
          </>
        ) : (
          <>
            <header className="popup-header">
              <LangSwitcher value={targetLang} onChange={onLangChange} />
              <HeaderActions />
            </header>

            {/* Translation first — the primary content the user reached for. */}
            <section className="panel translation-panel">
              <TranslationBody state={state} onRetry={() => source && run(source, targetLang)} />
            </section>

            <div className="divider" />

            {/* Source below, muted, no label (it reads as the original on its own). */}
            <section className="panel source-panel">
              {sourceSegments.length > 0 ? (
                <FuriganaText segments={sourceSegments} />
              ) : (
                <p className="src-text">{source || "…"}</p>
              )}
            </section>

            {/* Reply thread: each reply translated back into the source language. */}
            {replies.map((r, i) => (
              <ReplyTurnView key={i} turn={r} />
            ))}

            {/* Reply control: only once there's a translation to reply to. */}
            {state.status === "result" &&
              (replyOpen ? (
                <ReplyBox
                  lang={targetLang}
                  onSend={(t) => void onReply(t)}
                  onCancel={() => setReplyOpen(false)}
                />
              ) : (
                <button className="reply-btn" onClick={() => setReplyOpen(true)}>
                  ↩ Reply
                </button>
              ))}
          </>
        )}
      </div>
    </div>
  );
}

/** A single reply turn: translated reply (source language) over the reply text. */
function ReplyTurnView({ turn }: { turn: ReplyTurn }) {
  return (
    <div className="reply-turn">
      <div className="divider" />
      {turn.status === "loading" ? (
        <p className="loading">Translating…</p>
      ) : turn.status === "error" ? (
        <p className="error">{ERROR_MESSAGES[turn.code ?? ""] ?? "Reply translation failed."}</p>
      ) : (
        <div className="result">
          {turn.segments.length > 0 ? (
            <FuriganaText segments={turn.segments} />
          ) : (
            <p className="translation">{turn.output}</p>
          )}
          <ResultActions text={turn.output} />
        </div>
      )}
      <p className="src-text reply-in">↳ {turn.input}</p>
    </div>
  );
}

/** History + settings + close buttons, top-right of the popup. ESC also closes. */
function HeaderActions() {
  return (
    <div className="header-actions">
      <button
        className="icon-btn"
        onClick={() => void showHistory()}
        title="History"
        aria-label="History"
      >
        🕘
      </button>
      <button
        className="icon-btn"
        onClick={() => void showSettings()}
        title="Settings"
        aria-label="Settings"
      >
        ⚙
      </button>
      <button
        className="icon-btn"
        onClick={() => void hidePopup()}
        title="Close"
        aria-label="Close"
      >
        ✕
      </button>
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
        <p className="hint">
          Already enabled in the list? Remove the app with “−” and add it back — app updates
          invalidate the old grant.
        </p>
        <button onClick={() => void requestAccessibility()}>Grant permission</button>
        <button onClick={() => void openAccessibilitySettings()}>Open System Settings</button>
      </div>
    );
  }
  if (code === "no-selection") {
    return <p className="hint">Select text first, then press the hotkey.</p>;
  }
  return <p className="error">Couldn't capture the selection. Try again.</p>;
}
