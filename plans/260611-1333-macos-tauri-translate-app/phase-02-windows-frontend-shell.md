# Phase 02 — Windows + Frontend Shell

## Context Links

- Parent plan: [plan.md](plan.md)
- Depends on: [phase-01](phase-01-scaffold-activation-policy-tray.md)
- Research: [researcher-01 §3 (multi-window)](research/researcher-01-tauri-macos-system.md)

## Overview

- **Date:** 2026-06-11
- **Description:** Pre-declare popup (frameless, always-on-top) + settings windows hidden in config; React shell with per-window entry (hash routing, no router lib); Rust show/hide commands; ESC hides popup; tray Settings opens settings window.
- **Priority:** P1
- **Implementation status:** ✅ done — tsc + cargo check clean; dev boots, both windows instantiate hidden, no panic. Tray Settings wired to `show_settings_inner`; CloseRequested→hide for popup+settings.
- **Review status:** not started

## Key Insights

- Declaring windows in `tauri.conf.json` with `"visible": false` pre-creates them at startup → instant `show()` later (no webview cold start on hotkey press). Never destroy; only show/hide.
- `skipTaskbar` is Windows/Linux-only; on macOS dock-hiding already comes from Accessory policy — omit.
- Popup `"focus": false` in config = doesn't steal focus at creation. Phase 03 relies on capture-before-focus ordering.
- Hiding window from JS would need `core:window:allow-hide` capability; custom Rust commands keep capability surface minimal (KISS).
- Hash URLs (`index.html#/popup`) route both windows from one Vite bundle.

## Requirements

Functional:
- Popup window: frameless (`decorations:false`), `alwaysOnTop:true`, 460x300, centered, non-resizable, hidden at start.
- Settings window: titled "Settings", 480x420, centered, hidden at start.
- React renders PopupView or SettingsView based on URL hash.
- Commands: `show_popup`, `hide_popup`, `show_settings`. ESC in popup → hides it.
- Tray "Settings" menu item shows + focuses settings window. Settings close button hides (not destroys) window.

Non-functional: TS files kebab-case <200 LOC; immutable state patterns; no router dependency (YAGNI).

## Architecture

```
tauri.conf.json windows[]:  popup(#/popup, hidden, frameless, onTop) | settings(#/settings, hidden)
src/main.tsx ── hash switch ──> popup/popup-view.tsx | settings/settings-view.tsx
Rust windows.rs: show_popup/hide_popup/show_settings (get_webview_window → show/set_focus/hide)
tray.rs "settings" ──> windows::show_settings(app)
PopupView keydown(Escape) ──invoke──> hide_popup
```

Data flow: tray event / hotkey (phase 03) → Rust window command → webview shows. Frontend → `invoke('hide_popup')` on ESC.

## Related Code Files

CREATE (all <200 LOC):
- `src/popup/popup-view.tsx` — placeholder shell (real UI phase 05) + ESC handler
- `src/settings/settings-view.tsx` — placeholder shell (real UI phase 06)
- `src/shared/tauri-api.ts` — typed `invoke` wrappers (`hidePopup()`, etc.)
- `src-tauri/src/windows.rs` — show/hide commands

MODIFY:
- `src-tauri/tauri.conf.json` — add `app.windows` array (2 entries)
- `src/main.tsx` — hash-based window entry switch
- `src-tauri/src/lib.rs` — register `windows` module + `invoke_handler` commands
- `src-tauri/src/tray.rs` — wire "settings" menu id to `show_settings`

## Implementation Steps

1. `tauri.conf.json` windows array (per research §3):
   ```json
   "app": { "windows": [
     { "label": "popup", "url": "index.html#/popup", "visible": false, "decorations": false,
       "alwaysOnTop": true, "center": true, "resizable": false, "width": 460, "height": 300, "focus": false },
     { "label": "settings", "url": "index.html#/settings", "visible": false, "title": "Settings",
       "width": 480, "height": 420, "center": true }
   ]}
   ```
2. `windows.rs` commands (use `Manager` trait):
   ```rust
   #[tauri::command]
   pub fn hide_popup(app: tauri::AppHandle) -> Result<(), String> {
       app.get_webview_window("popup").ok_or("no popup")?.hide().map_err(|e| e.to_string())
   }
   // show_popup: show() + set_focus(); show_settings: show() + set_focus()
   ```
   Note: `show_popup` exists for completeness/tests; phase 03 hotkey path calls show AFTER capture.
3. Register in `lib.rs`: `.invoke_handler(tauri::generate_handler![windows::show_popup, windows::hide_popup, windows::show_settings])`.
4. Tray: replace phase-01 stub — `"settings" => { let _ = windows::show_settings_inner(app); }` (shared inner fn, DRY).
5. `src/main.tsx`: `const view = window.location.hash.includes('settings') ? <SettingsView/> : <PopupView/>` → render.
6. `popup-view.tsx`: placeholder text + `useEffect` keydown listener → `if (e.key === 'Escape') hidePopup()`. Cleanup listener on unmount.
7. `settings-view.tsx`: placeholder. Intercept close: settings window `onCloseRequested` → `preventDefault` + hide (Rust `on_window_event(WindowEvent::CloseRequested)` for label "settings" → `api.prevent_close(); window.hide()`) so window object survives for re-show.
8. Capabilities: keep `capabilities/default.json` minimal — `"permissions": ["core:default"]`, both window labels listed.
9. Verify: dev run → tray Settings opens window; close button hides it; re-open instant. Temporarily call `show_popup` (dev button or command line) → frameless on-top popup; ESC hides.

## Todo List

- [ ] Add popup + settings entries to tauri.conf.json
- [ ] `windows.rs` show/hide commands + register in invoke_handler
- [ ] Wire tray Settings menu → show_settings
- [ ] CloseRequested → prevent_close + hide for settings (and popup)
- [ ] `main.tsx` hash switch; popup/settings placeholder views
- [ ] ESC-to-hide in popup view via `hidePopup()` wrapper
- [ ] Minimal capabilities file covering both windows
- [ ] Smoke test show/hide cycles (no window destruction)

## Success Criteria

- Both windows hidden at launch; settings opens from tray <100ms (pre-created); popup shows frameless + stays above full-screen-less apps; ESC hides popup; closing settings hides (re-openable).
- No webview re-creation in logs across repeated show/hide. `cargo check` + TS build clean.

## Risk Assessment

- **ESC not received when popup lacks focus** (M, M): hotkey flow focuses popup after show (phase 03); ESC handler only matters when focused — acceptable. Mitigation: also hide on window blur later (follow-up, YAGNI now).
- **Window close destroys webview → next show fails** (M, H): mitigated by CloseRequested prevent_close+hide (step 7).
- **alwaysOnTop popup floats over everything during dev annoyance** (L, L): ESC/hide available.

## Security Considerations

- Capabilities minimal (`core:default` only) — no window/clipboard/shortcut JS permissions; all privileged ops behind explicit Rust commands.

## Next Steps

- Phase 03 hotkey pipeline calls capture → then `show_popup` + emits event to popup webview.
- Phase 05/06 replace placeholder views with real UI.
