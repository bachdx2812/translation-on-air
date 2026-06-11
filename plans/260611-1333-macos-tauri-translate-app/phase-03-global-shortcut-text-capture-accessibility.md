# Phase 03 — Global Shortcut, Text Capture, Accessibility

## Context Links

- Parent plan: [plan.md](plan.md)
- Depends on: [phase-01](phase-01-scaffold-activation-policy-tray.md), [phase-02](phase-02-windows-frontend-shell.md)
- Research: [researcher-01 §4 (global-shortcut), §5 (CGEvent + clipboard + AX)](research/researcher-01-tauri-macos-system.md); [researcher-02 §3 (TCC dev caveats)](research/researcher-02-llm-providers-furigana.md)

## Overview

- **Date:** 2026-06-11
- **Description:** Register global hotkey (default Cmd+Shift+T) with dynamic-rebind support; on press: verify Accessibility permission, synth Cmd+C via CGEvent, capture selection through clipboard (save→poll→restore), emit captured text to popup, show popup. Missing-permission guidance UX with deep-link.
- **Priority:** P1 (core interaction)
- **Implementation status:** ✅ done — cargo check clean; dev boots, hotkey Cmd+Shift+T registers (no setup error). capture.rs uses clear→synth→poll-nonempty→restore (improved over plan: clears clipboard first to avoid stale-text false positives). Full capture flow (AX grant + real selection) = manual test.
- **Review status:** not started

## Key Insights

- tauri-plugin-global-shortcut 2.3.2 Rust-side (`GlobalShortcutExt`) needs NO capability entries (capabilities gate JS only). Carbon hotkey API → registering the hotkey itself needs no AX permission; only CGEvent posting does.
- Accelerator parse: `"Cmd+Shift+T".parse::<Shortcut>()`. Rebind = `unregister(old)` + `register(new)`; ONE `with_handler` dispatcher serves all (handler set once at plugin build).
- CGEvent synth: `kVK_ANSI_C = 8` + `CGEventFlagCommand`, post down+up to `CGEventTapLocation::HID`. ~15 LOC, core-graphics already in tauri's dep tree.
- **Ordering critical**: capture MUST run before popup `show()+set_focus()` — else Cmd+C targets our popup, not the frontmost app.
- Clipboard read/restore via `ClipboardExt` (Rust side). Poll for change (50ms × 10) beats fixed sleep for slow apps. Restore text-only v1.
- AX check: `macos-accessibility-client` `application_is_trusted_with_prompt()` shows system dialog once; deep-link `x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility`.
- Dev mode: TCC grant attaches to running binary — grant Accessibility to the launching terminal/IDE so it survives rebuilds (researcher-02 §3).

## Requirements

Functional:
- Default `Cmd+Shift+T` registered at startup (read persisted accelerator from store if present, else default — store written by phase 06).
- Hotkey press → AX check → capture selection → emit `capture-done {text}` to popup window → show+focus popup. Empty/unchanged selection → emit `capture-error {code:"no-selection"}` but still show popup (hint UI).
- AX missing → emit `capture-error {code:"ax-missing"}` + show popup; popup offers "Open System Settings" button → `open_accessibility_settings` command.
- Prior clipboard text restored after capture; `rebind_shortcut(old, new)` function reusable by phase 06.

Non-functional: capture pipeline async (no main-thread blocking); total capture latency target <600ms; modules <200 LOC.

## Architecture

```
hotkey press ─> with_handler(Pressed) ─> tauri::async_runtime::spawn(pipeline)
 pipeline: accessibility::is_trusted()? ──no──> emit ax-missing ─> show popup
   └─yes─> capture::capture_selection(app):
            prev = clipboard.read_text().ok()
            cg_synth_cmd_c()                       // frontmost app still focused
            poll 50ms x10 until clipboard != prev
            captured = read_text(); restore prev
   ──> emit_to("popup", "capture-done", {text}) ─> windows::show_popup (show + set_focus)
```

Modules: `hotkey.rs` (register/rebind/dispatch), `capture.rs` (CGEvent + clipboard dance), `accessibility.rs` (trust check, prompt, deep-link command).

## Related Code Files

CREATE (all <200 LOC):
- `src-tauri/src/hotkey.rs` — plugin setup, `register_from_settings(app)`, `rebind(app, old, new)`, pressed-handler → pipeline spawn
- `src-tauri/src/capture.rs` — `synth_cmd_c()`, `capture_selection(app) -> Result<String, CaptureError>`
- `src-tauri/src/accessibility.rs` — `is_trusted(prompt: bool) -> bool`, `#[tauri::command] open_accessibility_settings`, `#[tauri::command] check_accessibility`

MODIFY:
- `src-tauri/Cargo.toml` — add tauri-plugin-global-shortcut 2.3.2, tauri-plugin-clipboard-manager 2.3.2, core-graphics 0.25, macos-accessibility-client 0.0.2
- `src-tauri/src/lib.rs` — register plugins (global-shortcut Builder `with_handler`, clipboard), commands, call `hotkey::register_from_settings`
- `src/shared/tauri-api.ts` — add `checkAccessibility()`, `openAccessibilitySettings()` wrappers + event payload types

## Implementation Steps

1. Add crates (versions above). Register clipboard plugin: `.plugin(tauri_plugin_clipboard_manager::init())`.
2. `hotkey.rs` — plugin with single dispatcher (research §4):
   ```rust
   app.handle().plugin(
       tauri_plugin_global_shortcut::Builder::new()
           .with_handler(|app, _shortcut, event| {
               if event.state() == ShortcutState::Pressed {
                   let app = app.clone();
                   tauri::async_runtime::spawn(async move { run_capture_pipeline(app).await; });
               }
           }).build(),
   )?;
   app.global_shortcut().register("Cmd+Shift+T".parse::<Shortcut>().unwrap())?;
   ```
   `rebind(app, old, new)`: parse both → `unregister(old)` → `register(new)`; on register failure re-register old (rollback) and return Err.
3. `capture.rs` synth (research §5): CGEventSource `CombinedSessionState`, keyboard event keycode 8, `set_flags(CGEventFlagCommand)`, post down+up to HID.
4. `capture_selection`:
   ```rust
   let prev = app.clipboard().read_text().ok();
   synth_cmd_c()?;
   let captured = poll_clipboard_change(&app, prev.as_deref()).await; // 50ms x 10
   if let Some(p) = &prev { let _ = app.clipboard().write_text(p.clone()); }
   captured.ok_or(CaptureError::NoSelection)
   ```
   Edge cases: selection identical to prior clipboard → poll sees "no change" → treat as captured-if-nonempty after timeout (use prev value), else NoSelection. Document in code comment.
5. Pipeline: AX gate first — `accessibility::is_trusted(false)`; on false emit `capture-error {code:"ax-missing"}` (and call `is_trusted(true)` ONCE per app run to surface system prompt). Then capture → `app.emit_to("popup", "capture-done", payload)` → show+focus popup. Errors → `capture-error` + still show popup.
6. `accessibility.rs`: wrap `application_is_trusted_with_prompt()` / `application_is_trusted()`; `open_accessibility_settings` = `open` cmd with `x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility`.
7. Register new commands in `invoke_handler`; extend `tauri-api.ts` types (`CaptureDonePayload {text: string}`, `CaptureErrorPayload {code: 'no-selection'|'ax-missing'|'capture-failed'}`).
8. Dev setup note (README/plan): grant Accessibility to the terminal running `tauri dev`.
9. Verify manually: select text in Safari/Notes → hotkey → popup shows captured text event (placeholder UI logs it); clipboard restored; no-selection and AX-revoked paths show correct error codes.

## Todo List

- [ ] Add 4 crates; register clipboard + global-shortcut plugins
- [ ] `hotkey.rs` dispatcher + default registration + `rebind` with rollback
- [ ] `capture.rs` CGEvent synth Cmd+C
- [ ] Clipboard save → poll-change → read → restore sequence
- [ ] `accessibility.rs` trust check + prompt-once + deep-link command
- [ ] Pipeline wiring: AX gate → capture → emit → show popup
- [ ] Event payload types in `tauri-api.ts`
- [ ] Manual verify: 3 apps, no-selection, AX-revoked, clipboard restore

## Success Criteria

- Hotkey fires in any frontmost app; selected text arrives in popup webview console <600ms; prior clipboard text intact afterward.
- AX revoked (System Settings toggle off) → popup shows ax-missing code; deep-link opens correct pane.
- Rebind function swaps accelerator live (unit-testable parse/rollback logic; live test in phase 06).

## Risk Assessment

- **Cmd+C synth races slow apps (Electron, web apps)** (M, M): poll-loop ×10 (500ms cap); if flaky, raise to ×16 or NSPasteboard changeCount upgrade (noted, YAGNI).
- **Popup steals focus before synth** (L, H): pipeline ordering capture→show enforced; popup config `focus:false` belt-and-suspenders.
- **Hotkey collision (already registered by other app)** (M, L): `register` returns Err → log + tray still works; phase 06 surfaces error in UI.
- **AX prompt loop annoyance** (M, L): prompt-once-per-run flag; popup guidance otherwise.
- **Dev rebuild loses AX grant** (H, M in dev only): terminal-grant workaround documented; phase 08 stable signing fixes properly.

## Security Considerations

- Synthetic keystrokes scoped to single Cmd+C — no arbitrary event injection API exposed to frontend.
- Captured text + clipboard contents stay in Rust/popup event; not logged. No clipboard history retained beyond single restore variable.
- Commands exposed to JS: only `check_accessibility`, `open_accessibility_settings`, window show/hide — no capture trigger from JS (hotkey-only, prevents webview-driven clipboard sniffing).

## Next Steps

- Phase 04 provides `translate` command the popup calls after `capture-done`.
- Phase 05 builds real popup UI consuming these events; phase 06 calls `rebind` + persists accelerator.
