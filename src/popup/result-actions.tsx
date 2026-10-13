import { useEffect, useRef, useState } from "react";
import { copyText } from "../shared/tauri-api";
import { IconCheck, IconCopy } from "../shared/icons";

/** Copy button with brief "copied" feedback. Copies the plain translation text. */
export function ResultActions({ text }: { text: string }) {
  const [copied, setCopied] = useState(false);
  const timer = useRef<ReturnType<typeof setTimeout>>(undefined);
  useEffect(() => () => clearTimeout(timer.current), []);

  const onCopy = async () => {
    try {
      await copyText(text);
      setCopied(true);
      clearTimeout(timer.current);
      timer.current = setTimeout(() => setCopied(false), 1500);
    } catch {
      // Clipboard write failures are non-fatal; leave the button label unchanged.
    }
  };

  return (
    <div className="result-actions">
      <button className={copied ? "chip done" : "chip"} onClick={() => void onCopy()}>
        {copied ? <IconCheck /> : <IconCopy />}
        {copied ? "Đã chép" : "Sao chép"}
      </button>
    </div>
  );
}
