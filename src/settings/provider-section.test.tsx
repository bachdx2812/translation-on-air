import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, fireEvent, cleanup } from "@testing-library/react";
import { ProviderSection } from "./provider-section";
import { setSettings } from "../shared/tauri-api";
import type { Settings } from "../shared/types";

vi.mock("../shared/tauri-api", () => ({
  detectProviders: vi.fn(() => new Promise(() => {})),
  setSettings: vi.fn().mockResolvedValue(undefined),
  setOpenAiKey: vi.fn(),
  deleteOpenAiKey: vi.fn(),
}));

const initial: Settings = {
  hotkey: "Cmd+Shift+T",
  target_lang: "vi",
  provider_mode: "openai",
  openai_model: "gpt-6-luna",
  openai_reasoning_effort: "none",
  has_openai_key: true,
  selection_bubble: true,
  source_furigana: false,
};

describe("ProviderSection reasoning effort", () => {
  beforeEach(() => {
    vi.mocked(setSettings).mockClear();
  });

  // No vitest `globals` → RTL auto-cleanup is off; clean up renders explicitly.
  afterEach(cleanup);

  it("shows the saved effort", () => {
    const { getByLabelText } = render(<ProviderSection initial={initial} />);
    expect((getByLabelText("OpenAI reasoning effort") as HTMLSelectElement).value).toBe("none");
  });

  it("persists the chosen effort", () => {
    const { getByLabelText } = render(<ProviderSection initial={initial} />);
    const select = getByLabelText("OpenAI reasoning effort") as HTMLSelectElement;

    fireEvent.change(select, { target: { value: "low" } });

    expect(select.value).toBe("low");
    expect(setSettings).toHaveBeenCalledWith({ openai_reasoning_effort: "low" });
  });
});
