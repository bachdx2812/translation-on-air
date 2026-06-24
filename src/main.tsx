import React from "react";
import ReactDOM from "react-dom/client";
import { PopupView } from "./popup/popup-view";
import { SettingsView } from "./settings/settings-view";
import { BubbleView } from "./bubble/bubble-view";
import "./styles/global.css";

// A single Vite bundle serves every Tauri window; the URL hash selects the view.
// popup → `#/popup`, settings → `#/settings`, bubble → `#/bubble`.
const hash = window.location.hash;
const view = hash.includes("settings") ? (
  <SettingsView />
) : hash.includes("bubble") ? (
  <BubbleView />
) : (
  <PopupView />
);

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>{view}</React.StrictMode>,
);
