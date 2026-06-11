import { useState } from "react";
import { copyText } from "../shared/tauri-api";

/** Copy button with brief "Copied" feedback. Copies the plain translation text. */
export function ResultActions({ text }: { text: string }) {
  const [copied, setCopied] = useState(false);

  const onCopy = async () => {
    try {
      await copyText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      // Clipboard write failures are non-fatal; leave the button label unchanged.
    }
  };

  return (
    <button className="copy-btn" onClick={onCopy}>
      {copied ? "Copied" : "Copy"}
    </button>
  );
}
