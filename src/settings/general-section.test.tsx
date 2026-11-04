import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, fireEvent, cleanup, waitFor } from "@testing-library/react";
import { GeneralSection } from "./general-section";
import { setSettings } from "../shared/tauri-api";
import type { Settings } from "../shared/types";

vi.mock("../shared/tauri-api", () => ({
  setSettings: vi.fn().mockResolvedValue(undefined),
  setHotkey: vi.fn().mockResolvedValue(undefined),
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
  launch_at_login: false,
};

describe("GeneralSection", () => {
  beforeEach(() => {
    vi.mocked(setSettings).mockClear();
  });

  // No vitest `globals` → RTL auto-cleanup is off; clean up renders explicitly.
  afterEach(cleanup);

  it("persists turning source furigana on", () => {
    const { getByLabelText } = render(<GeneralSection initial={initial} />);
    const box = getByLabelText("Furigana cho câu gốc tiếng Nhật") as HTMLInputElement;
    expect(box.checked).toBe(false);

    fireEvent.click(box);

    expect(box.checked).toBe(true);
    expect(setSettings).toHaveBeenCalledWith({ source_furigana: true });
  });

  it("persists turning launch at login on", () => {
    const { getByLabelText } = render(<GeneralSection initial={initial} />);
    const box = getByLabelText("Mở cùng macOS") as HTMLInputElement;

    fireEvent.click(box);

    expect(box.checked).toBe(true);
    expect(setSettings).toHaveBeenCalledWith({ launch_at_login: true });
  });

  it("switches launch at login back off when saving fails", async () => {
    vi.mocked(setSettings).mockRejectedValueOnce("Operation not permitted");
    const { getByLabelText } = render(<GeneralSection initial={initial} />);
    const box = getByLabelText("Mở cùng macOS") as HTMLInputElement;

    fireEvent.click(box);

    await waitFor(() => expect(box.checked).toBe(false));
  });
});
