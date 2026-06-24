import { describe, it, expect, vi, afterEach } from "vitest";
import { render, cleanup, waitFor } from "@testing-library/react";
import { PopupView } from "./popup-view";

// Popup pulls settings via invoke("get_settings") and subscribes to capture
// events; stub both so the component mounts in jsdom.
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(() => Promise.resolve({ target_lang: "vi" })),
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));

afterEach(cleanup);

describe("PopupView layout", () => {
  it("renders the translation panel above the source panel", async () => {
    const { container } = render(<PopupView />);

    await waitFor(() => {
      expect(container.querySelector(".lang-switcher")).toBeTruthy();
    });

    const panels = container.querySelectorAll(".panel");
    // First panel is the translation (primary), second is the muted source.
    expect(panels[0].className).toContain("translation-panel");
    expect(panels[1].className).toContain("source-panel");
  });

  it("shows the language switcher and gear in the header", async () => {
    const { container, getByLabelText } = render(<PopupView />);

    await waitFor(() => {
      expect(container.querySelector(".popup-header .lang-switcher")).toBeTruthy();
    });
    expect(getByLabelText("Settings")).toBeTruthy();
  });
});
