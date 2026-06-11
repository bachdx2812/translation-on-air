# Research: Tauri v2 macOS System Integration (background translate app)

Date: 2026-06-11 | Target: tauri 2.11.2 (latest stable, crates.io). All versions verified via crates.io API 2026-06-11.

## 1. No dock icon + keep running

Canonical for v2 = Rust API `ActivationPolicy::Accessory` in `setup()` — works in dev AND bundle. `LSUIElement` via custom `src-tauri/Info.plist` (auto-merged at build) only affects the bundled .app, not `tauri dev` → use Rust API. `AppHandle::set_activation_policy()` also exists for runtime toggling (e.g. show dock while settings open). macOS-only; guard with cfg.

```rust
.setup(|app| {
    #[cfg(target_os = "macos")]
    app.set_activation_policy(tauri::ActivationPolicy::Accessory);
    Ok(())
})
```

Cite: https://docs.rs/tauri/latest/tauri/struct.App.html#method.set_activation_policy , https://docs.rs/tauri/latest/tauri/enum.ActivationPolicy.html

## 2. Tray icon (core, no plugin)

Built into tauri core behind `tray-icon` Cargo feature — no plugin. Menu via `tauri::menu`. Icon: reuse `app.default_window_icon()` or `Image::from_path` (needs `image-png` feature). Use `.icon_as_template(true)` on macOS so menu-bar icon adapts to dark/light.

```toml
tauri = { version = "2", features = ["tray-icon", "image-png"] }
```
```rust
use tauri::{menu::{Menu, MenuItem}, tray::TrayIconBuilder};
let settings_i = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
let menu = Menu::with_items(app, &[&settings_i, &quit_i])?;
TrayIconBuilder::new()
    .icon(app.default_window_icon().unwrap().clone())
    .icon_as_template(true)
    .menu(&menu)
    .show_menu_on_left_click(true)   // NOT deprecated menu_on_left_click
    .on_menu_event(|app, e| match e.id.as_ref() {
        "quit" => app.exit(0),
        "settings" => { if let Some(w) = app.get_webview_window("settings") { let _ = w.show(); let _ = w.set_focus(); } }
        _ => {}
    })
    .build(app)?;
```

Cite: https://v2.tauri.app/learn/system-tray/ (verified: `show_menu_on_left_click`, `on_menu_event`, `on_tray_icon_event` with `TrayIconEvent::Click{button: MouseButton::Left, ..}`).

## 3. Multi-window: popup + settings

Recommended: declare both windows in `tauri.conf.json` with `"visible": false` (pre-created at startup → instant `show()`), toggle via show/hide, never destroy. `WebviewWindowBuilder` equivalent for lazy creation. `skipTaskbar` is Windows/Linux-only — on macOS dock hiding comes from Accessory policy (sec 1), so omit. Transparency (if wanted later) needs `"macOSPrivateApi": true`.

```json
"app": { "windows": [
  { "label": "popup", "url": "index.html#/popup", "visible": false, "decorations": false,
    "alwaysOnTop": true, "center": true, "resizable": false, "width": 460, "height": 300, "focus": false },
  { "label": "settings", "url": "index.html#/settings", "visible": false, "title": "Settings",
    "width": 480, "height": 420, "center": true }
]}
```
```rust
// show/hide cycle (hotkey handler); Manager trait for get_webview_window
let w = app.get_webview_window("popup").unwrap();
w.show()?; w.set_focus()?;          // fast re-show
w.hide()?;                           // on dismiss (Esc/blur) — keep alive
// dynamic alternative:
tauri::WebviewWindowBuilder::new(app, "popup", tauri::WebviewUrl::App("index.html#/popup".into()))
    .decorations(false).always_on_top(true).visible(false).center().build()?;
```

Cite: https://docs.rs/tauri/latest/tauri/webview/struct.WebviewWindowBuilder.html , https://v2.tauri.app/reference/config/#windowconfig

## 4. tauri-plugin-global-shortcut v2.3.2

`cargo add tauri-plugin-global-shortcut` (desktop-only target ok). Rust-side use via `GlobalShortcutExt` needs NO capability entries (capabilities gate JS/IPC only; add `global-shortcut:allow-register` etc. only if calling from JS). Accelerator strings parse via `Shortcut::from_str`: `"CmdOrCtrl+Shift+T"`, `"Cmd+Shift+T"`, `"Alt+Space"`. Uses Carbon hotkey API → no Accessibility permission needed. Dynamic rebind = unregister old, register new; single global handler dispatches.

```rust
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
app.handle().plugin(
    tauri_plugin_global_shortcut::Builder::new()
        .with_handler(|app, _shortcut, event| {
            if event.state() == ShortcutState::Pressed { /* trigger capture+translate */ }
        }).build(),
)?;
app.global_shortcut().register("Cmd+Shift+T".parse::<Shortcut>().unwrap())?;
// rebind at runtime:
let gs = app.global_shortcut();
gs.unregister(old.parse::<Shortcut>()?)?;   // or gs.unregister_all()
gs.register(new.parse::<Shortcut>()?)?;
```

Cite: https://v2.tauri.app/plugin/global-shortcut/ , https://docs.rs/tauri-plugin-global-shortcut/2.3.2 (crates.io: 2.3.2)

## 5. Text capture: synth Cmd+C + clipboard + AX permission

**Keystroke synthesis — ranked:** (1) `core-graphics` 0.25.0 CGEvent: minimal deps (already in tauri's tree), deterministic, ~15 LOC. (2) `enigo` 0.6.1: nicer API but extra dep tree + broader surface for a single keystroke → YAGNI. Both require Accessibility permission. Pick core-graphics.

```rust
use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
const KVK_ANSI_C: u16 = 8;
let src = CGEventSource::new(CGEventSourceStateID::CombinedSessionState).unwrap();
for down in [true, false] {
    let ev = CGEvent::new_keyboard_event(src.clone(), KVK_ANSI_C, down).unwrap();
    ev.set_flags(CGEventFlags::CGEventFlagCommand);
    ev.post(CGEventTapLocation::HID);
}
```

**Clipboard:** `tauri-plugin-clipboard-manager` 2.3.2, Rust `ClipboardExt`. Save→copy→wait ~150–300ms (poll for change)→read→restore. Limitation: restore is text-only; non-text prior clipboard (image/files) is lost — acceptable v1 trade-off.

```rust
use tauri_plugin_clipboard_manager::ClipboardExt;
let prev = app.clipboard().read_text().ok();        // save
send_cmd_c();                                        // synth
tokio::time::sleep(std::time::Duration::from_millis(200)).await;
let captured = app.clipboard().read_text()?;         // read selection
if let Some(p) = prev { app.clipboard().write_text(p)?; }  // restore
```

**AX permission:** `macos-accessibility-client` 0.0.2 (thin wrapper over `AXIsProcessTrustedWithOptions` w/ prompt; tiny but stable, used by tauri ecosystem apps) or raw FFI via `accessibility-sys` 0.2.0. Open the pane directly:

```rust
let trusted = macos_accessibility_client::accessibility::application_is_trusted_with_prompt(); // shows system dialog once
// deep-link to settings pane:
std::process::Command::new("open")
    .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
    .spawn()?;
```

Cite: https://v2.tauri.app/plugin/clipboard/ , https://docs.rs/core-graphics/0.25.0 , https://docs.rs/macos-accessibility-client , https://crates.io/crates/enigo (0.6.1)

## 6. Secure storage: keyring + tauri-plugin-store

**Recommend pin `keyring = "3"` (3.x line), NOT 4.0.1.** v4 (released post-Jan-2026) split into `keyring-core` + per-platform store crates (`apple-native-keyring-store`) and requires explicit `use_apple_keychain_store()` init — brand-new major, sparse migration docs = adoption risk. v3 is battle-tested, single feature flag. Revisit v4 once ecosystem settles.

```toml
keyring = { version = "3", features = ["apple-native"] }
tauri-plugin-store = "2"   # 2.4.3
```
```rust
let entry = keyring::Entry::new("translate-on-air", "openai_api_key")?;
entry.set_password(&key)?;            // store in macOS Keychain
let key = entry.get_password()?;      // read
entry.delete_credential()?;           // remove (v3 name; was delete_password in v2)

use tauri_plugin_store::StoreExt;     // non-secret prefs
let store = app.store("settings.json")?;
store.set("hotkey", serde_json::json!("Cmd+Shift+T"));
let lang = store.get("target_lang");  // Option<JsonValue>
store.save()?;
```

Cite: https://docs.rs/keyring/3 , https://docs.rs/keyring/latest (v4 store-split verified) , https://v2.tauri.app/plugin/store/ (2.4.3)

## Unresolved questions

1. keyring v4.0.1 exact `Entry` API unverified (docs.rs page lacked samples) — pinning v3 sidesteps this; confirm latest 3.x patch at `cargo add` time.
2. Clipboard change-detection: fixed sleep vs polling `read_text` diff vs NSPasteboard `changeCount` (needs objc2-app-kit). Start with poll-loop (50ms x 10); upgrade only if flaky.
3. `icon_as_template` requires a monochrome PNG w/ alpha — asset must be authored accordingly (not verified against final icon asset).
4. Dev-mode AX permission attaches to the running binary (terminal/IDE may need granting during dev); re-grant needed after bundle signing changes.
