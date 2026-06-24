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
});
