import type { Segment } from "../shared/types";

/**
 * Render Japanese segments with furigana. Tokens that carry a hiragana `reading`
 * become <ruby>surface<rt>reading</rt></ruby>; everything else is plain text.
 * Index keys are fine — the list is replaced wholesale on each translation.
 */
export function FuriganaText({ segments }: { segments: Segment[] }) {
  return (
    <p lang="ja" className="furigana">
      {segments.map((s, i) =>
        s.reading ? (
          <ruby key={i}>
            {s.surface}
            <rt>{s.reading}</rt>
          </ruby>
        ) : (
          <span key={i}>{s.surface}</span>
        ),
      )}
    </p>
  );
}
