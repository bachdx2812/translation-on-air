import { useCallback, useRef, useState } from "react";
import { translate as invokeTranslate } from "../shared/tauri-api";
import type { Translated, TargetLang } from "../shared/types";

export type TranslationState =
  | { status: "idle" }
  // `partial` is the translation streamed so far ("" until the first chunk).
  | { status: "loading"; partial: string }
  | { status: "result"; result: Translated }
  | { status: "error"; code: string };

/**
 * Translation runner with a stale-response guard: if the user switches language
 * mid-flight, only the latest request's partials and result are applied.
 */
export function useTranslation() {
  const [state, setState] = useState<TranslationState>({ status: "idle" });
  const reqId = useRef(0);

  const run = useCallback(async (text: string, lang: TargetLang) => {
    const id = ++reqId.current;
    setState({ status: "loading", partial: "" });
    // Partials arrive on a separate IPC path from the result, so one can land
    // after it; only apply while still loading.
    const onPartial = (partial: string) => {
      if (id !== reqId.current) return;
      setState((s) => (s.status === "loading" ? { status: "loading", partial } : s));
    };
    try {
      const result = await invokeTranslate(text, lang, onPartial);
      if (id === reqId.current) setState({ status: "result", result });
    } catch (e) {
      // Rust returns the stable error code as the rejection value.
      if (id === reqId.current) setState({ status: "error", code: String(e) });
    }
  }, []);

  return { state, run };
}
