import React from "react";
import ReactDOM from "react-dom/client";
import { PopupView } from "./popup/popup-view";
import { SettingsView } from "./settings/settings-view";
import "./styles/global.css";

// A single Vite bundle serves both Tauri windows; the URL hash selects the view.
// popup window loads `index.html#/popup`, settings loads `index.html#/settings`.
const isSettings = window.location.hash.includes("settings");
const view = isSettings ? <SettingsView /> : <PopupView />;

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>{view}</React.StrictMode>,
);
