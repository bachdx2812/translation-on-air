import { useEffect, useState } from "react";
import { getSettings } from "../shared/tauri-api";
import type { Settings } from "../shared/types";
import { GeneralSection } from "./general-section";
import { ProviderSection } from "./provider-section";
import "../styles/settings.css";

export function SettingsView() {
  const [settings, setSettings] = useState<Settings | null>(null);

  useEffect(() => {
    void getSettings().then(setSettings);
  }, []);

  return (
    <main className="settings">
      {settings ? (
        <>
          <GeneralSection initial={settings} />
          <ProviderSection initial={settings} />
        </>
      ) : (
        <p className="settings-loading">Đang tải…</p>
      )}
    </main>
  );
}
