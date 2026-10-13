import React from "react";
import ReactDOM from "react-dom/client";
import { PopupView } from "./popup/popup-view";
import { SettingsView } from "./settings/settings-view";
import { BubbleView } from "./bubble/bubble-view";
import "./styles/global.css";

// A single Vite bundle serves every Tauri window; the URL hash selects the view.
// popup → `#/popup`, settings → `#/settings`, bubble → `#/bubble`.
const hash = window.location.hash;
const viewName = hash.includes("settings") ? "settings" : hash.includes("bubble") ? "bubble" : "popup";
// Lets the shared stylesheet scope root-level rules (e.g. the popup's
// non-scrolling page) to one window.
document.documentElement.dataset.view = viewName;
const view = viewName === "settings" ? (
  <SettingsView />
) : viewName === "bubble" ? (
  <BubbleView />
) : (
  <PopupView />
);

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>{view}</React.StrictMode>,
);
