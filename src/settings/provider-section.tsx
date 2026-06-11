import { useEffect, useState } from "react";
import {
  deleteOpenAiKey,
  detectProviders,
  setOpenAiKey,
  setSettings,
} from "../shared/tauri-api";
import type { ProviderMode, ProviderStatus, Settings } from "../shared/types";

const MODES: ProviderMode[] = ["auto", "openai", "claude"];
const MODE_LABELS: Record<ProviderMode, string> = {
  auto: "Auto",
  openai: "OpenAI",
  claude: "Claude",
};

function statusLine(s: ProviderStatus | null): string {
  if (!s) return "Detecting…";
  if (s.resolved === "openai") return "Active: OpenAI";
  if (s.resolved === "claude") return "Active: Claude CLI (local subscription)";
  return "No provider configured.";
}

/** Provider mode + OpenAI key management + model + detection status. */
export function ProviderSection({ initial }: { initial: Settings }) {
  const [mode, setMode] = useState<ProviderMode>(initial.provider_mode);
  const [model, setModel] = useState(initial.openai_model);
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
    <section className="setting-section">
      <label>Provider</label>
      <div className="radios">
        {MODES.map((m) => (
          <label key={m}>
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
      <p className="status">{statusLine(status)}</p>
      <p className="hint">
        Claude CLI ≈5s per translation; OpenAI ≈1–2s. From 2026-06-15 Claude subscription
        headless usage draws Agent SDK credits.
      </p>
      <button type="button" onClick={redetect}>
        Re-detect
      </button>

      <label>OpenAI API key</label>
      {hasKey ? (
        <div className="key-row">
          <span>Key saved ✓</span>
          <button type="button" onClick={() => void clearKey()}>
            Clear
          </button>
        </div>
      ) : (
        <div className="key-row">
          <input
            type="password"
            value={keyInput}
            placeholder="sk-..."
            onChange={(e) => setKeyInput(e.target.value)}
          />
          <button type="button" onClick={() => void saveKey()}>
            Save
          </button>
        </div>
      )}

      <label htmlFor="model">OpenAI model</label>
      <input
        id="model"
        value={model}
        onChange={(e) => setModel(e.target.value)}
        onBlur={() => void setSettings({ openai_model: model })}
      />
    </section>
  );
}
