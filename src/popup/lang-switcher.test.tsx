import { describe, it, expect, vi } from "vitest";
import { render, fireEvent } from "@testing-library/react";
import { LangSwitcher } from "./lang-switcher";

describe("LangSwitcher", () => {
  it("marks the active language and emits the selected one", () => {
    const onChange = vi.fn();
    const { getByText } = render(<LangSwitcher value="vi" onChange={onChange} />);

    expect(getByText("Tiếng Việt").className).toContain("active");
    expect(getByText("日本語").className).not.toContain("active");

    fireEvent.click(getByText("日本語"));
    expect(onChange).toHaveBeenCalledWith("ja");
  });
});
