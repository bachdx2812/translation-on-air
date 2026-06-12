import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, fireEvent, waitFor, cleanup } from "@testing-library/react";
import { HotkeyRecorder } from "./hotkey-recorder";
import { setHotkey } from "../shared/tauri-api";

vi.mock("../shared/tauri-api", () => ({
  setHotkey: vi.fn().mockResolvedValue(undefined),
}));

describe("HotkeyRecorder", () => {
  beforeEach(() => {
    vi.mocked(setHotkey).mockClear();
    vi.mocked(setHotkey).mockResolvedValue(undefined);
  });

  // No vitest `globals` → RTL auto-cleanup is off; clean up renders explicitly.
  afterEach(cleanup);

  // WKWebView (all Tauri windows on macOS) does NOT focus a <button> on click,
  // so key events land on <body>, never on the button. Recording must therefore
  // listen at the window level.
  it("records a combo even though the button never receives focus", async () => {
    const { getByRole } = render(<HotkeyRecorder initial="Cmd+Shift+T" />);
    fireEvent.click(getByRole("button"));

    fireEvent.keyDown(window, { code: "KeyY", metaKey: true, shiftKey: true });

    await waitFor(() => expect(setHotkey).toHaveBeenCalledWith("Cmd+Shift+Y"));
    expect(getByRole("button").textContent).toBe("Cmd+Shift+Y");
  });

  it("Escape cancels recording without saving", () => {
    const { getByRole } = render(<HotkeyRecorder initial="Cmd+Shift+T" />);
    fireEvent.click(getByRole("button"));

    fireEvent.keyDown(window, { key: "Escape", code: "Escape" });
    expect(setHotkey).not.toHaveBeenCalled();
    expect(getByRole("button").textContent).toBe("Cmd+Shift+T");
  });

  it("ignores keys pressed without a modifier and keeps recording", async () => {
    const { getByRole } = render(<HotkeyRecorder initial="Cmd+Shift+T" />);
    fireEvent.click(getByRole("button"));

    fireEvent.keyDown(window, { code: "KeyY" });
    expect(setHotkey).not.toHaveBeenCalled();

    fireEvent.keyDown(window, { code: "KeyP", metaKey: true });
    await waitFor(() => expect(setHotkey).toHaveBeenCalledWith("Cmd+P"));
  });

  it("shows the rejection message when Rust reports register-failed", async () => {
    vi.mocked(setHotkey).mockRejectedValue("register-failed");
    const { getByRole, getByText } = render(<HotkeyRecorder initial="Cmd+Shift+T" />);
    fireEvent.click(getByRole("button"));

    fireEvent.keyDown(window, { code: "KeyY", metaKey: true });

    await waitFor(() => expect(getByText(/Couldn't register/)).toBeTruthy());
    expect(getByRole("button").textContent).toBe("Cmd+Shift+T");
  });
});
