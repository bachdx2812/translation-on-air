import { describe, it, expect, vi, afterEach } from "vitest";
import { render, cleanup, waitFor, act } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { PopupView } from "./popup-view";

// Popup pulls settings via invoke("get_settings") and subscribes to capture
// events; stub both so the component mounts in jsdom.
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(() => Promise.resolve({ target_lang: "vi" })),
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));

// jsdom has no ResizeObserver; keep the latest callback so tests can fire it.
let resizeCallback: () => void = () => {};
vi.stubGlobal(
  "ResizeObserver",
  class {
    constructor(cb: () => void) {
      resizeCallback = cb;
    }
    observe() {}
    disconnect() {}
  },
);

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
    expect(getByLabelText("Cài đặt")).toBeTruthy();
  });
});

describe("PopupView window sizing", () => {
  it("sizes the window to the content plus the card's measured chrome", async () => {
    // jsdom reports a 0px screen, which would clamp every height to the minimum.
    Object.defineProperty(window.screen, "availHeight", { value: 1000, configurable: true });
    const { container } = render(<PopupView />);
    const card = container.querySelector(".popup") as HTMLElement;
    const body = container.querySelector(".popup-body") as HTMLElement;
    const content = container.querySelector(".popup-content") as HTMLElement;
    // Header + padding + border = 300 - 240 = 60px around the content.
    Object.defineProperty(card, "offsetHeight", { value: 300 });
    Object.defineProperty(body, "clientHeight", { value: 240 });
    content.getBoundingClientRect = () => ({ height: 250 }) as DOMRect;

    act(() => resizeCallback());

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("resize_popup", { width: 460, height: 310 }),
    );
  });
});
