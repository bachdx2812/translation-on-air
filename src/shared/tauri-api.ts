import { invoke } from "@tauri-apps/api/core";
import type { Translated, TargetLang, ProviderStatus, Settings, SettingsPatch } from "./types";

// Typed wrappers around Rust commands. Keeping invoke calls in one module means
// view components never reference raw command-name strings.

// --- Window commands ---
export const showPopup = (): Promise<void> => invoke("show_popup");
export const hidePopup = (): Promise<void> => invoke("hide_popup");
export const showSettings = (): Promise<void> => invoke("show_settings");
export const resizePopup = (width: number, height: number): Promise<void> =>
  invoke("resize_popup", { width, height });

// --- Accessibility (macOS permission for synthetic Cmd+C) ---
export const checkAccessibility = (): Promise<boolean> => invoke("check_accessibility");
export const openAccessibilitySettings = (): Promise<void> =>
  invoke("open_accessibility_settings");

// --- Translation + providers ---
export const translate = (text: string, targetLang: TargetLang): Promise<Translated> =>
  invoke("translate", { text, targetLang });
export const detectProviders = (): Promise<ProviderStatus> => invoke("detect_providers");
export const setOpenAiKey = (key: string): Promise<void> => invoke("set_openai_key", { key });
export const deleteOpenAiKey = (): Promise<void> => invoke("delete_openai_key");
export const hasOpenAiKey = (): Promise<boolean> => invoke("has_openai_key");
export const copyText = (text: string): Promise<void> => invoke("copy_text", { text });

// --- Settings (persisted prefs) ---
export const getSettings = (): Promise<Settings> => invoke("get_settings");
export const setSettings = (patch: SettingsPatch): Promise<void> =>
  invoke("set_settings", { patch });
export const setHotkey = (accel: string): Promise<void> => invoke("set_hotkey", { accel });

// --- Events emitted by the capture pipeline to the popup window ---
export type CaptureDonePayload = { text: string };
export type CaptureErrorCode = "no-selection" | "ax-missing" | "capture-failed";
export type CaptureErrorPayload = { code: CaptureErrorCode };
