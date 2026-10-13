import { useEffect, useState } from "react";
import {
  deleteOpenAiKey,
  detectProviders,
  setOpenAiKey,
  setSettings,
} from "../shared/tauri-api";
import type {
  ProviderMode,
  ProviderStatus,
  ReasoningEffort,
  Settings,
} from "../shared/types";
import { IconCheck, IconRefresh } from "../shared/icons";
import { SettingGroup, SettingRow } from "./setting-row";

const MODES: ProviderMode[] = ["auto", "openai", "claude"];
const MODE_LABELS: Record<ProviderMode, string> = {
  auto: "Tự động",
  openai: "OpenAI",
  claude: "Claude",
};

const EFFORTS: ReasoningEffort[] = ["none", "low", "medium", "high", "xhigh", "default"];
const EFFORT_LABELS: Record<ReasoningEffort, string> = {
  none: "Không (nhanh nhất)",
  low: "Thấp",
  medium: "Trung bình",
  high: "Cao",
  xhigh: "Rất cao",
  default: "Mặc định của model",
};

function Status({ status, onRedetect }: { status: ProviderStatus | null; onRedetect: () => void }) {
  const [tone, text] = !status
    ? ["", "Đang kiểm tra…"]
    : status.resolved === "openai"
      ? ["ok", "Đang dùng OpenAI"]
      : status.resolved === "claude"
        ? ["ok", "Đang dùng Claude CLI (gói cá nhân)"]
        : ["bad", "Chưa có dịch vụ nào dùng được"];
  return (
    <span className="status-line">
      <span className={`status ${tone}`}>
        <i className="dot" />
        {text}
      </span>
      <button type="button" className="link-btn" onClick={onRedetect}>
        <IconRefresh />
        Kiểm tra lại
      </button>
    </span>
  );
}

/** Provider mode + OpenAI key management + model + reasoning effort + detection status. */
export function ProviderSection({ initial }: { initial: Settings }) {
  const [mode, setMode] = useState<ProviderMode>(initial.provider_mode);
  const [model, setModel] = useState(initial.openai_model);
  const [effort, setEffort] = useState<ReasoningEffort>(initial.openai_reasoning_effort);
  const [hasKey, setHasKey] = useState(initial.has_openai_key);
  const [keyInput, setKeyInput] = useState("");
  const [status, setStatus] = useState<ProviderStatus | null>(null);

  const redetect = () => void detectProviders().then(setStatus);
  useEffect(() => {
    redetect();
  }, []);

  const onMode = (m: ProviderMode) => {
    setMode(m);
    void setSettings({ provider_mode: m }).then(redetect);
  };
  const onEffort = (e: ReasoningEffort) => {
    setEffort(e);
    void setSettings({ openai_reasoning_effort: e });
  };
  const saveKey = async () => {
    if (!keyInput) return;
    await setOpenAiKey(keyInput);
    setKeyInput("");
    setHasKey(true);
    redetect();
  };
  const clearKey = async () => {
    await deleteOpenAiKey();
    setHasKey(false);
    redetect();
  };

  return (
    <SettingGroup
      title="Dịch vụ dịch"
      footnote="OpenAI dịch mất khoảng 1–2 giây, Claude CLI khoảng 5 giây. Từ 15/06/2026, dùng gói Claude ở chế độ headless sẽ trừ vào credit Agent SDK."
    >
      <SettingRow label="Dịch vụ" hint={<Status status={status} onRedetect={redetect} />}>
        <div className="segmented" role="radiogroup" aria-label="Dịch vụ">
          {MODES.map((m) => (
            <label key={m} className={mode === m ? "active" : ""}>
              <input
                type="radio"
                name="provider"
                checked={mode === m}
                onChange={() => onMode(m)}
              />
              {MODE_LABELS[m]}
            </label>
          ))}
        </div>
      </SettingRow>

      {/* The OpenAI options only matter when OpenAI can be picked. */}
      {mode !== "claude" && (
        <>
          {hasKey ? (
            <SettingRow label="OpenAI API key">
              <div className="key-row">
                <span className="key-saved">
                  <IconCheck />
                  Đã lưu trong Keychain
                </span>
                <button type="button" className="btn" onClick={() => void clearKey()}>
                  Xoá
                </button>
              </div>
            </SettingRow>
          ) : (
            <SettingRow
              label="OpenAI API key"
              htmlFor="openai-key"
              hint="Lưu trong Keychain của macOS, không lưu vào file cài đặt."
              stacked
            >
              <div className="key-row">
                <input
                  id="openai-key"
                  type="password"
                  value={keyInput}
                  placeholder="sk-…"
                  onChange={(e) => setKeyInput(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") void saveKey();
                  }}
                />
                <button
                  type="button"
                  className="btn btn-primary"
                  disabled={!keyInput}
                  onClick={() => void saveKey()}
                >
                  Lưu
                </button>
              </div>
            </SettingRow>
          )}

          <SettingRow label="Model" htmlFor="model">
            <input
              id="model"
              className="model-input"
              value={model}
              spellCheck={false}
              onChange={(e) => {
                // Saved per keystroke: closing destroys the window, which may
                // skip a final blur.
                setModel(e.target.value);
                void setSettings({ openai_model: e.target.value });
              }}
            />
          </SettingRow>

          <SettingRow
            label="Mức suy luận"
            htmlFor="effort"
            hint="Mức càng cao thì model nghĩ càng lâu, dịch càng chậm. Model không hỗ trợ mức đã chọn thì dùng mặc định của model."
          >
            <select
              id="effort"
              value={effort}
              onChange={(e) => onEffort(e.target.value as ReasoningEffort)}
            >
              {EFFORTS.map((e) => (
                <option key={e} value={e}>
                  {EFFORT_LABELS[e]}
                </option>
              ))}
            </select>
          </SettingRow>
        </>
      )}
    </SettingGroup>
  );
}
