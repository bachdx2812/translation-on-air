import { describe, it, expect, vi, afterEach } from "vitest";
import { render, cleanup, fireEvent } from "@testing-library/react";
import { BubbleView } from "./bubble-view";
import * as api from "../shared/tauri-api";

vi.mock("../shared/tauri-api", () => ({
  bubbleTranslate: vi.fn(() => Promise.resolve()),
  hideBubble: vi.fn(() => Promise.resolve()),
}));

afterEach(cleanup);

describe("BubbleView", () => {
  it("runs the translate pipeline when the button is clicked", () => {
    const { getByRole } = render(<BubbleView />);
    fireEvent.click(getByRole("button"));
    expect(api.bubbleTranslate).toHaveBeenCalledOnce();
  });

  it("dismisses on Escape", () => {
    render(<BubbleView />);
    fireEvent.keyDown(window, { key: "Escape" });
    expect(api.hideBubble).toHaveBeenCalledOnce();
  });
});
