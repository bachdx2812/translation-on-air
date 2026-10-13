import { describe, it, expect, vi, afterEach } from "vitest";
import { render, cleanup, fireEvent } from "@testing-library/react";
import { ReplyBox } from "./reply-box";

afterEach(cleanup);

describe("ReplyBox", () => {
  it("sends the text on Enter, but not when empty", () => {
    const onSend = vi.fn();
    const { getByRole } = render(<ReplyBox lang="vi" onSend={onSend} onCancel={() => {}} />);
    const ta = getByRole("textbox");

    fireEvent.keyDown(ta, { key: "Enter" });
    expect(onSend).not.toHaveBeenCalled();

    fireEvent.change(ta, { target: { value: "xin chào" } });
    fireEvent.keyDown(ta, { key: "Enter" });
    expect(onSend).toHaveBeenCalledWith("xin chào");
  });

  it("Shift+Enter inserts a newline instead of sending", () => {
    const onSend = vi.fn();
    const { getByRole } = render(<ReplyBox lang="vi" onSend={onSend} onCancel={() => {}} />);
    const ta = getByRole("textbox");
    fireEvent.change(ta, { target: { value: "hi" } });
    fireEvent.keyDown(ta, { key: "Enter", shiftKey: true });
    expect(onSend).not.toHaveBeenCalled();
  });

  it("Enter that confirms an IME composition does not send", () => {
    const onSend = vi.fn();
    const { getByRole } = render(<ReplyBox lang="ja" onSend={onSend} onCancel={() => {}} />);
    const ta = getByRole("textbox");
    fireEvent.change(ta, { target: { value: "へんしん" } });
    fireEvent.keyDown(ta, { key: "Enter", isComposing: true });
    expect(onSend).not.toHaveBeenCalled();
  });

  it("Esc closes the composer without reaching the popup's own Esc handler", () => {
    const onCancel = vi.fn();
    const onWindowKey = vi.fn();
    window.addEventListener("keydown", onWindowKey);
    const { getByRole } = render(<ReplyBox lang="vi" onSend={() => {}} onCancel={onCancel} />);

    fireEvent.keyDown(getByRole("textbox"), { key: "Escape" });

    expect(onCancel).toHaveBeenCalledOnce();
    expect(onWindowKey).not.toHaveBeenCalled();
    window.removeEventListener("keydown", onWindowKey);
  });
});
