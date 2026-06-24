import { useState } from "react";
import { setSettings } from "../shared/tauri-api";
import type { Settings, TargetLang } from "../shared/types";

/** Default target language + selection-bubble toggle. Saved on change (no global
 * Save button — KISS). */
export function GeneralSection({ initial }: { initial: Settings }) {
  const [lang, setLang] = useState<TargetLang>(initial.target_lang);
  const [bubble, setBubble] = useState<boolean>(initial.selection_bubble);

  const onLang = (l: TargetLang) => {
    setLang(l);
    void setSettings({ target_lang: l });
  };

  const onBubble = (on: boolean) => {
    setBubble(on);
    void setSettings({ selection_bubble: on });
  };

  return (
    <section className="setting-section">
      <label htmlFor="lang">Default target language</label>
      <select id="lang" value={lang} onChange={(e) => onLang(e.target.value as TargetLang)}>
        <option value="vi">Tiếng Việt</option>
        <option value="ja">日本語 (furigana)</option>
        <option value="en">English</option>
      </select>

      <label className="setting-check">
        <input
          type="checkbox"
          checked={bubble}
          onChange={(e) => onBubble(e.target.checked)}
        />
        Hiện nút “Dịch” khi bôi đen text
      </label>
      <p className="setting-hint">
        Bôi đen text ở bất kỳ app nào để hiện nút dịch nổi cạnh con trỏ (cần quyền Accessibility).
      </p>
    </section>
  );
}
