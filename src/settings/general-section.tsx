import { useState } from "react";
import { setSettings } from "../shared/tauri-api";
import type { Settings, TargetLang } from "../shared/types";

/** Default target language. Saved on change (no global Save button — KISS). */
export function GeneralSection({ initial }: { initial: Settings }) {
  const [lang, setLang] = useState<TargetLang>(initial.target_lang);

  const onChange = (l: TargetLang) => {
    setLang(l);
    void setSettings({ target_lang: l });
  };

  return (
    <section className="setting-section">
      <label htmlFor="lang">Default target language</label>
      <select id="lang" value={lang} onChange={(e) => onChange(e.target.value as TargetLang)}>
        <option value="vi">Tiếng Việt</option>
        <option value="ja">日本語 (furigana)</option>
        <option value="en">English</option>
      </select>
    </section>
  );
}
