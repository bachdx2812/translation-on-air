import { useEffect, useState } from "react";
import { getSettings } from "../shared/tauri-api";
import type { Settings } from "../shared/types";
import { GeneralSection } from "./general-section";
import { HotkeyRecorder } from "./hotkey-recorder";
import { ProviderSection } from "./provider-section";
import "../styles/settings.css";

export function SettingsView() {
  const [settings, setSettings] = useState<Settings | null>(null);

  useEffect(() => {
    void getSettings().then(setSettings);
  }, []);

  if (!settings) {
    return (
      <main className="settings">
        <p className="hint">Loading…</p>
      </main>
    );
  }

  return (
    <main className="settings">
      <h1>Settings</h1>
      <GeneralSection initial={settings} />
      <HotkeyRecorder initial={settings.hotkey} />
      <ProviderSection initial={settings} />
    </main>
  );
}
