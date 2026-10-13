import { useState } from "react";
import { setSettings } from "../shared/tauri-api";
import type { Settings, TargetLang } from "../shared/types";
import { HotkeyRecorder } from "./hotkey-recorder";
import { SettingGroup, SettingRow } from "./setting-row";

/** Target language, hotkey and the selection-bubble / source-furigana toggles.
 * Saved on change (no global Save button — KISS). */
export function GeneralSection({ initial }: { initial: Settings }) {
  const [lang, setLang] = useState<TargetLang>(initial.target_lang);
  const [bubble, setBubble] = useState<boolean>(initial.selection_bubble);
  const [sourceFurigana, setSourceFurigana] = useState<boolean>(initial.source_furigana);

  const onLang = (l: TargetLang) => {
    setLang(l);
    void setSettings({ target_lang: l });
  };

  const onBubble = (on: boolean) => {
    setBubble(on);
    void setSettings({ selection_bubble: on });
  };

  const onSourceFurigana = (on: boolean) => {
    setSourceFurigana(on);
    void setSettings({ source_furigana: on });
  };

  return (
    <SettingGroup title="Chung">
      <SettingRow
        label="Dịch sang"
        htmlFor="lang"
        hint="Ngôn ngữ mặc định; đổi nhanh ngay trên popup."
      >
        <select id="lang" value={lang} onChange={(e) => onLang(e.target.value as TargetLang)}>
          <option value="vi">Tiếng Việt</option>
          <option value="ja">日本語 (kèm furigana)</option>
          <option value="en">English</option>
        </select>
      </SettingRow>

      <SettingRow label="Phím tắt dịch" hint="Bôi đen text ở bất kỳ app nào rồi bấm phím tắt.">
        <HotkeyRecorder initial={initial.hotkey} />
      </SettingRow>

      <SettingRow
        label="Nút “Dịch” khi bôi đen text"
        htmlFor="bubble"
        hint="Hiện nút dịch nổi cạnh con trỏ (cần quyền Trợ năng)."
      >
        <input
          id="bubble"
          type="checkbox"
          role="switch"
          className="switch"
          checked={bubble}
          onChange={(e) => onBubble(e.target.checked)}
        />
      </SettingRow>

      <SettingRow
        label="Furigana cho câu gốc tiếng Nhật"
        htmlFor="source-furigana"
        hint="Tắt để dịch nhanh hơn: model không phải sinh cách đọc cho từng chữ kanji."
      >
        <input
          id="source-furigana"
          type="checkbox"
          role="switch"
          className="switch"
          checked={sourceFurigana}
          onChange={(e) => onSourceFurigana(e.target.checked)}
        />
      </SettingRow>
    </SettingGroup>
  );
}
