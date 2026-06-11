import { describe, it, expect } from "vitest";
import { render } from "@testing-library/react";
import { FuriganaText } from "./furigana-text";

describe("FuriganaText", () => {
  it("uses <ruby><rt> for kanji segments and plain text for kana", () => {
    const { container } = render(
      <FuriganaText
        segments={[
          { surface: "日本語", reading: "にほんご" },
          { surface: "です", reading: "" },
        ]}
      />,
    );
    expect(container.querySelectorAll("ruby")).toHaveLength(1);
    expect(container.querySelector("rt")?.textContent).toBe("にほんご");
    expect(container.querySelectorAll("span")).toHaveLength(1);
    expect(container.textContent).toContain("です");
  });

  it("renders nothing special for an empty segment list", () => {
    const { container } = render(<FuriganaText segments={[]} />);
    expect(container.querySelectorAll("ruby")).toHaveLength(0);
  });
});
