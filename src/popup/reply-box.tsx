import { useState } from "react";
import type { TargetLang } from "../shared/types";

// The user types the reply in the current target language; it is translated back
// into the source language (see translate_reply).
const PLACEHOLDER: Record<TargetLang, string> = {
  vi: "Nhập câu trả lời bằng tiếng Việt…",
  ja: "返信を日本語で入力…",
  en: "Type your reply in English…",
};

/** Inline reply composer. Enter sends, Shift+Enter inserts a newline, Esc
 * closes the composer (not the popup). */
export function ReplyBox({
  lang,
  onSend,
  onCancel,
}: {
  lang: TargetLang;
  onSend: (text: string) => void;
  onCancel: () => void;
}) {
  const [text, setText] = useState("");
  const submit = () => {
    if (text.trim()) onSend(text);
  };

  return (
    <div className="reply-box">
      <textarea
        className="reply-input"
        autoFocus
        rows={2}
        value={text}
        placeholder={PLACEHOLDER[lang]}
        onChange={(e) => setText(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter" && !e.shiftKey && !e.nativeEvent.isComposing) {
            e.preventDefault();
            submit();
          } else if (e.key === "Escape") {
            // Keep the popup's own Esc handler from closing the whole window.
            e.stopPropagation();
            onCancel();
          }
        }}
      />
      <div className="reply-actions">
        <span className="kbd-hint">↵ dịch · ⇧↵ xuống dòng · esc huỷ</span>
        <button className="chip" onClick={onCancel}>
          Huỷ
        </button>
        <button className="chip chip-primary" onClick={submit} disabled={!text.trim()}>
          Dịch câu trả lời
        </button>
      </div>
    </div>
  );
}
