import type { TargetLang } from "../shared/types";

const LANGS: { code: TargetLang; label: string }[] = [
  { code: "vi", label: "Tiếng Việt" },
  { code: "ja", label: "日本語" },
  { code: "en", label: "English" },
];

/** Target-language buttons. Switching re-translates the stored source (no recapture). */
export function LangSwitcher({
  value,
  onChange,
}: {
  value: TargetLang;
  onChange: (lang: TargetLang) => void;
}) {
  return (
    <div className="lang-switcher">
      {LANGS.map((l) => (
        <button
          key={l.code}
          className={l.code === value ? "active" : ""}
          onClick={() => onChange(l.code)}
        >
          {l.label}
        </button>
      ))}
    </div>
  );
}
