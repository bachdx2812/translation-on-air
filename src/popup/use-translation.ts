import { useCallback, useRef, useState } from "react";
import { translate as invokeTranslate } from "../shared/tauri-api";
import type { Translated, TargetLang } from "../shared/types";

export type TranslationState =
  | { status: "idle" }
  | { status: "loading" }
  | { status: "result"; result: Translated }
  | { status: "error"; code: string };

/**
 * Translation runner with a stale-response guard: if the user switches language
 * mid-flight, only the latest request's result is applied (older ones ignored).
 */
export function useTranslation() {
  const [state, setState] = useState<TranslationState>({ status: "idle" });
  const reqId = useRef(0);

  const run = useCallback(async (text: string, lang: TargetLang) => {
    const id = ++reqId.current;
    setState({ status: "loading" });
    try {
      const result = await invokeTranslate(text, lang);
      if (id === reqId.current) setState({ status: "result", result });
    } catch (e) {
      // Rust returns the stable error code as the rejection value.
      if (id === reqId.current) setState({ status: "error", code: String(e) });
    }
  }, []);

  return { state, run };
}
