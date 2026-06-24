import { useState } from "react";
import type { TargetLang } from "../shared/types";

// The user types the reply in the current target language; it is translated back
// into the source language (see translate_reply).
const PLACEHOLDER: Record<TargetLang, string> = {
  vi: "Nhập câu trả lời (tiếng Việt)…",
  ja: "返信を入力（日本語）…",
  en: "Type your reply (English)…",
};

/** Inline reply composer. Enter sends, Shift+Enter inserts a newline. */
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
          if (e.key === "Enter" && !e.shiftKey) {
            e.preventDefault();
            submit();
          }
        }}
      />
      <div className="reply-actions">
        <button className="reply-cancel" onClick={onCancel}>
          Cancel
        </button>
        <button className="reply-send" onClick={submit} disabled={!text.trim()}>
          Send
        </button>
      </div>
    </div>
  );
}
