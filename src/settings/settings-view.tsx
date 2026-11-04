import { useEffect, useState } from "react";
import { getSettings } from "../shared/tauri-api";
import type { Settings } from "../shared/types";
import { AboutSection, UpdateBanner, useUpdateStatus } from "./about-section";
import { GeneralSection } from "./general-section";
import { ProviderSection } from "./provider-section";
import "../styles/settings.css";

export function SettingsView() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const update = useUpdateStatus();

  useEffect(() => {
    void getSettings().then(setSettings);
  }, []);

  return (
    <main className="settings">
      {settings ? (
        <>
          {update.status?.available && <UpdateBanner status={update.status} />}
          <GeneralSection initial={settings} />
          <ProviderSection initial={settings} />
          <AboutSection
            status={update.status}
            checking={update.checking}
            onCheck={() => void update.check()}
          />
        </>
      ) : (
        <p className="settings-loading">Đang tải…</p>
      )}
    </main>
  );
}
