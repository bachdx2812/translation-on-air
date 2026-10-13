import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  getSettings,
  hidePopup,
  openAccessibilitySettings,
  requestAccessibility,
  resizePopup,
  showSettings,
  translateReply,
  type CaptureDonePayload,
  type CaptureErrorCode,
  type CaptureErrorPayload,
} from "../shared/tauri-api";
import type { Segment, TargetLang } from "../shared/types";
import { hotkeyKeys } from "../shared/hotkey";
import { IconAlert, IconClose, IconGlobe, IconReply, IconSettings } from "../shared/icons";
import { useTranslation } from "./use-translation";
import { FuriganaText } from "./furigana-text";
import { LangSwitcher } from "./lang-switcher";
import { ResultActions } from "./result-actions";
import { ReplyBox } from "./reply-box";
import "../styles/popup.css";

// phase 06 loads the persisted default; until then Vietnamese (the product default).
const DEFAULT_LANG: TargetLang = "vi";
const POPUP_WIDTH = 460;
const MIN_HEIGHT = 120;

const ERROR_MESSAGES: Record<string, string> = {
  "not-configured": "Chưa có dịch vụ dịch nào — thêm OpenAI API key trong Cài đặt (hoặc đăng nhập Claude).",
  "invalid-key": "OpenAI không chấp nhận API key này. Kiểm tra lại trong Cài đặt.",
  "rate-limited": "Đang gửi quá nhiều yêu cầu — thử lại sau giây lát.",
  "quota-exhausted": "Đã hết credit Claude — đổi dịch vụ trong Cài đặt.",
  unavailable: "Dịch vụ dịch đang không phản hồi.",
  "bad-response": "Không đọc được kết quả dịch.",
  timeout: "Hết thời gian chờ dịch.",
};
/** Errors only fixable in Settings get a shortcut there instead of Retry. */
const SETTINGS_ERRORS = new Set(["not-configured", "invalid-key", "quota-exhausted"]);

const REPLY_PROMPT: Record<TargetLang, string> = {
  vi: "Trả lời bằng tiếng Việt…",
  ja: "日本語で返信…",
  en: "Reply in English…",
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
  const [hotkey, setHotkey] = useState<string | null>(null);
  const [captureError, setCaptureError] = useState<CaptureErrorCode | null>(null);
  const { state, run } = useTranslation();
  const [replies, setReplies] = useState<ReplyTurn[]>([]);
  const [replyOpen, setReplyOpen] = useState(false);
  const cardRef = useRef<HTMLDivElement>(null);
  const bodyRef = useRef<HTMLDivElement>(null);
  const contentRef = useRef<HTMLDivElement>(null);
  const lastHeight = useRef(0);

  // Keep latest target lang in a ref so the (subscribe-once) capture listener
  // translates into the current language without re-subscribing.
  const targetLangRef = useRef(targetLang);
  useEffect(() => {
    targetLangRef.current = targetLang;
  }, [targetLang]);

  // Load the persisted default target language (and the hotkey for hints).
  useEffect(() => {
    void getSettings()
      .then((s) => {
        setTargetLang(s.target_lang);
        setHotkey(s.hotkey ?? null);
      })
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
          let lang = targetLangRef.current;
          try {
            const s = await getSettings();
            lang = s.target_lang;
            setHotkey(s.hotkey ?? null);
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

  // Auto-size the window to the content so the card fills it exactly (no window
  // backing showing as a faint layer behind it). A ResizeObserver catches every
  // height change — streamed text wrapping, the reply box opening, fonts loading
  // — not just React state changes. Everything around the content (header,
  // padding, border) is measured rather than hard-coded so CSS edits cannot
  // desync the two.
  useLayoutEffect(() => {
    const card = cardRef.current;
    const body = bodyRef.current;
    const content = contentRef.current;
    if (!card || !body || !content) return;
    const fit = () => {
      const chrome = card.offsetHeight - body.clientHeight;
      const maxH = Math.floor(window.screen.availHeight * 0.85);
      const needed = Math.ceil(content.getBoundingClientRect().height) + chrome;
      const height = Math.max(MIN_HEIGHT, Math.min(needed, maxH));
      if (height === lastHeight.current) return;
      lastHeight.current = height;
      void resizePopup(POPUP_WIDTH, height);
    };
    fit();
    const observer = new ResizeObserver(fit);
    observer.observe(content);
    return () => observer.disconnect();
  }, []);

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
    const onPartial = (partial: string) =>
      setReplies((rs) =>
        rs.map((r, i) => (i === idx && r.status === "loading" ? { ...r, output: partial } : r)),
      );
    try {
      const res = await translateReply(trimmed, source, original, targetLang, onPartial);
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

  const retry = () => {
    if (source) void run(source, targetLang);
  };

  const sourceSegments = state.status === "result" ? state.result.source_segments : [];

  return (
    // .popup fills the window (opaque rounded card): a fixed header, then
    // .popup-body, which scrolls only past the max window height and wraps the
    // measured .popup-content. The window is frameless, so the header (minus its
    // buttons) and any empty card space act as the drag handle; bare drag
    // regions only fire on the element itself, so text stays selectable.
    <div className="popup" ref={cardRef} data-tauri-drag-region>
      <header className="popup-header" data-tauri-drag-region="deep">
        {captureError ? (
          <span className="brand">
            <IconGlobe />
            Translate On Air
          </span>
        ) : (
          <LangSwitcher value={targetLang} onChange={onLangChange} />
        )}
        <HeaderActions />
      </header>

      <div className="popup-body" ref={bodyRef} data-tauri-drag-region>
        <div className="popup-content" ref={contentRef} data-tauri-drag-region>
          {captureError ? (
            <CaptureErrorView code={captureError} hotkey={hotkey} />
          ) : (
            <>
              {/* Translation first — the primary content the user reached for. */}
              <section className="panel translation-panel">
                <TranslationBody state={state} onRetry={retry} />
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
                  <button className="reply-trigger" onClick={() => setReplyOpen(true)}>
                    <IconReply />
                    {REPLY_PROMPT[targetLang]}
                  </button>
                ))}
            </>
          )}
        </div>
      </div>
    </div>
  );
}

/** A single reply turn: what the user wrote, then its translation into the
 * source language (the text to send back). */
function ReplyTurnView({ turn }: { turn: ReplyTurn }) {
  return (
    <div className="reply-turn">
      <div className="divider" />
      <p className="reply-in">
        <IconReply />
        {turn.input}
      </p>
      {turn.status === "loading" ? (
        turn.output ? (
          <p className="translation streaming">{turn.output}</p>
        ) : (
          <Skeleton />
        )
      ) : turn.status === "error" ? (
        <ErrorNotice message={ERROR_MESSAGES[turn.code ?? ""] ?? "Không dịch được câu trả lời."} />
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
    </div>
  );
}

/** Settings + close buttons, top-right of the popup. ESC also closes. */
function HeaderActions() {
  return (
    <div className="header-actions">
      <button
        className="icon-btn"
        onClick={() => void showSettings()}
        title="Cài đặt"
        aria-label="Cài đặt"
      >
        <IconSettings />
      </button>
      <button
        className="icon-btn"
        onClick={() => void hidePopup()}
        title="Đóng (esc)"
        aria-label="Đóng"
      >
        <IconClose />
      </button>
    </div>
  );
}

/** Placeholder lines shown until the first streamed words arrive. */
function Skeleton() {
  return (
    <div className="skeleton" role="status" aria-label="Đang dịch…">
      <span />
      <span />
    </div>
  );
}

function ErrorNotice({ message, onRetry }: { message: string; onRetry?: () => void }) {
  return (
    <div className="notice">
      <IconAlert />
      <div className="notice-body">
        <p>{message}</p>
        {onRetry && (
          <div className="notice-actions">
            <button className="chip" onClick={onRetry}>
              Thử lại
            </button>
          </div>
        )}
      </div>
    </div>
  );
}

function Keys({ accel }: { accel: string }) {
  return (
    <>
      {hotkeyKeys(accel).map((k) => (
        <kbd key={k}>{k}</kbd>
      ))}
    </>
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
      return state.partial ? <p className="translation streaming">{state.partial}</p> : <Skeleton />;
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
      return SETTINGS_ERRORS.has(state.code) ? (
        <div className="notice">
          <IconAlert />
          <div className="notice-body">
            <p>{ERROR_MESSAGES[state.code]}</p>
            <div className="notice-actions">
              <button className="chip chip-primary" onClick={() => void showSettings()}>
                Mở Cài đặt
              </button>
            </div>
          </div>
        </div>
      ) : (
        <ErrorNotice message={ERROR_MESSAGES[state.code] ?? "Dịch không thành công."} onRetry={onRetry} />
      );
  }
}

function CaptureErrorView({ code, hotkey }: { code: CaptureErrorCode; hotkey: string | null }) {
  if (code === "ax-missing") {
    return (
      <div className="notice">
        <IconAlert />
        <div className="notice-body">
          <p>Cần quyền Trợ năng (Accessibility) để đọc đoạn text đang được bôi đen.</p>
          <p className="hint">
            Đã bật trong danh sách mà vẫn báo lỗi? Xoá app bằng nút “−” rồi thêm lại — mỗi lần
            cập nhật app, quyền cũ sẽ mất hiệu lực.
          </p>
          <div className="notice-actions">
            <button className="chip chip-primary" onClick={() => void requestAccessibility()}>
              Cấp quyền
            </button>
            <button className="chip" onClick={() => void openAccessibilitySettings()}>
              Mở Cài đặt hệ thống
            </button>
          </div>
        </div>
      </div>
    );
  }
  if (code === "no-selection") {
    return (
      <div className="notice info">
        <IconAlert />
        <div className="notice-body">
          <p>Chưa có đoạn text nào được bôi đen.</p>
          <p className="hint">
            Bôi đen đoạn cần dịch rồi bấm {hotkey ? <Keys accel={hotkey} /> : "phím tắt"}, hoặc
            chuột phải → Services → Translate with Translate On Air.
          </p>
        </div>
      </div>
    );
  }
  return <ErrorNotice message="Không lấy được đoạn text đang bôi đen. Thử lại nhé." />;
}
