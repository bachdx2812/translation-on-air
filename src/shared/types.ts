// Mirror of the Rust translation types (src-tauri/src/providers/types.rs).

export type Segment = { surface: string; reading: string };

export type Translated = {
  translation: string;
  segments: Segment[];
  source_segments: Segment[];
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

export type Settings = {
  hotkey: string;
  target_lang: TargetLang;
  provider_mode: ProviderMode;
  openai_model: string;
  has_openai_key: boolean;
};

export type SettingsPatch = Partial<{
  target_lang: TargetLang;
  provider_mode: ProviderMode;
  openai_model: string;
}>;
