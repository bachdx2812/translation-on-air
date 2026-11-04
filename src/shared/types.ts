// Mirror of the Rust translation types (src-tauri/src/providers/types.rs).

export type Segment = { surface: string; reading: string };

export type Translated = {
  translation: string;
  segments: Segment[];
  source_segments: Segment[];
  /** Furigana was wanted but skipped because the text is too long. */
  furigana_skipped?: boolean;
};

export type TargetLang = "vi" | "ja" | "en";

export type TranslateErrorCode =
  | "not-configured"
  | "invalid-key"
  | "rate-limited"
  | "quota-exhausted"
  | "unavailable"
  | "bad-response"
  | "timeout";

export type ProviderStatus = {
  claude_detected: boolean;
  claude_path: string | null;
  claude_supports_json_schema: boolean;
  has_openai_key: boolean;
  resolved: "openai" | "claude" | "none";
};

export type ProviderMode = "auto" | "openai" | "claude";

/** "default" omits `reasoning_effort` so the model uses its own default. */
export type ReasoningEffort = "none" | "low" | "medium" | "high" | "xhigh" | "default";

export type Settings = {
  hotkey: string;
  target_lang: TargetLang;
  provider_mode: ProviderMode;
  openai_model: string;
  openai_reasoning_effort: ReasoningEffort;
  has_openai_key: boolean;
  selection_bubble: boolean;
  source_furigana: boolean;
  launch_at_login: boolean;
};

export type SettingsPatch = Partial<{
  target_lang: TargetLang;
  provider_mode: ProviderMode;
  openai_model: string;
  openai_reasoning_effort: ReasoningEffort;
  selection_bubble: boolean;
  source_furigana: boolean;
  launch_at_login: boolean;
}>;

/** Last update check (see src-tauri/src/updates.rs). */
export type UpdateStatus = {
  current: string;
  /** Newest released version; null until a check succeeded. */
  latest: string | null;
  available: boolean;
  /** "network", "http-<code>" or "bad-response" when the last check failed. */
  error: string | null;
};
