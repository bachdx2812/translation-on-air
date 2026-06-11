# Phase 01 — Scaffold, Activation Policy, Tray Icon

## Context Links

- Parent plan: [plan.md](plan.md)
- Dependencies: none (first phase)
- Research: [researcher-01 §1 (no dock icon)](research/researcher-01-tauri-macos-system.md), [§2 (tray icon)](research/researcher-01-tauri-macos-system.md)

## Overview

- **Date:** 2026-06-11
- **Description:** Greenfield Tauri v2 + React/TS scaffold. App runs as background agent: no dock icon (`ActivationPolicy::Accessory`), minimal menubar tray with Settings + Quit. Establish project structure all later phases build on.
- **Priority:** P1 (blocks everything)
- **Implementation status:** ✅ done — `cargo check` clean (no warnings); `tauri dev` boots (vite ready, bin runs, no panic → tray + Accessory policy execute). Menubar visual = human glance.
- **Review status:** not started

## Key Insights

- `ActivationPolicy::Accessory` via Rust API in `setup()` is canonical for v2 — works in `tauri dev` AND bundle. `LSUIElement` plist only affects bundle (phase 08). Guard with `#[cfg(target_os = "macos")]`.
- Tray is built into tauri core behind `tray-icon` Cargo feature — NO plugin needed. `image-png` feature required for PNG icon loading.
- `icon_as_template(true)` makes menubar icon adapt to dark/light — requires monochrome PNG with alpha.
- Use `show_menu_on_left_click(true)` (current API; `menu_on_left_click` is deprecated).
- Bundle identifier must be chosen NOW and stay stable forever — macOS TCC keys Accessibility grants to it (researcher-02 §3).

## Requirements

Functional:
- App launches with no dock icon, no main window; tray icon appears in menubar.
- Tray menu: "Settings" (no-op until phase 02 wires window), "Quit" (exits app).
- App keeps running with zero windows visible.

Non-functional:
- Rust modules snake_case, <200 LOC each. `main.rs` stays minimal (delegates to `lib.rs`).
- Compiles clean: `cargo check` + `pnpm tauri dev` boots.

## Architecture

```
main.rs → lib.rs run()
            ├─ .plugin(...)            (later phases)
            ├─ .setup(|app| {
            │     set_activation_policy(Accessory)   // no dock
            │     tray::create(app)                  // menubar icon + menu
            │  })
            └─ .run()
tray.rs   → TrayIconBuilder + Menu(Settings, Quit) + on_menu_event dispatch
```

Data flow: tray menu event → `on_menu_event` → match id → `app.exit(0)` | show settings (stub now, real in phase 02).

## Related Code Files

CREATE (all <200 LOC):
- `package.json`, `vite.config.ts`, `tsconfig.json`, `index.html`, `src/main.tsx` — via `create-tauri-app` react-ts template (reworked in phase 02)
- `src-tauri/Cargo.toml` — deps + features
- `src-tauri/tauri.conf.json` — identifier, build config (windows array added phase 02)
- `src-tauri/src/main.rs` — entry, calls `translate_on_air_lib::run()`
- `src-tauri/src/lib.rs` — builder, setup, activation policy
- `src-tauri/src/tray.rs` — tray creation module
- `src-tauri/icons/` — generated app icons + `tray-icon.png` (monochrome template, alpha channel)

## Implementation Steps

1. Scaffold: `pnpm create tauri-app@latest translate-on-air --template react-ts` content into project root `/Users/macos/apps/self/translate-on-air` (greenfield — adjust paths if generator demands empty dir: generate in temp, move contents).
2. Set identifier in `tauri.conf.json`: `"identifier": "com.bachdx.translateonair"` (no hyphens; NEVER change later — TCC).
3. `src-tauri/Cargo.toml`:
   ```toml
   tauri = { version = "2.11", features = ["tray-icon", "image-png"] }
   ```
4. `lib.rs` setup:
   ```rust
   .setup(|app| {
       #[cfg(target_os = "macos")]
       app.set_activation_policy(tauri::ActivationPolicy::Accessory);
       tray::create(app.handle())?;
       Ok(())
   })
   ```
5. `tray.rs` per research snippet:
   ```rust
   let settings_i = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
   let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
   let menu = Menu::with_items(app, &[&settings_i, &quit_i])?;
   TrayIconBuilder::new()
       .icon(app.default_window_icon().unwrap().clone())  // swap to template tray-icon.png when authored
       .icon_as_template(true)
       .menu(&menu)
       .show_menu_on_left_click(true)
       .on_menu_event(|app, e| match e.id.as_ref() {
           "quit" => app.exit(0),
           "settings" => { /* phase 02: show settings window */ }
           _ => {}
       })
       .build(app)?;
   ```
6. Author/generate monochrome template `tray-icon.png` (black + alpha, ~22x22@2x); load via `Image::from_path` or embed with `include_bytes!`. Default window icon acceptable fallback for this phase.
7. Remove scaffold's default visible-window assumption: delete default `windows` entry or set `"visible": false` (phase 02 replaces array wholesale).
8. Verify: `pnpm install && pnpm tauri dev` → no dock icon, tray visible, Quit works, app survives with no window.

## Todo List

- [ ] Scaffold create-tauri-app react-ts into project root
- [ ] Set stable bundle identifier `com.bachdx.translateonair`
- [ ] Add `tray-icon` + `image-png` features to Cargo.toml
- [ ] `lib.rs`: Accessory activation policy in setup (cfg macos)
- [ ] `tray.rs`: menu (Settings/Quit) + `icon_as_template(true)` + event handler
- [ ] Tray template icon asset (or default-icon fallback)
- [ ] Neutralize default visible window from scaffold
- [ ] `cargo check` clean + `pnpm tauri dev` smoke test

## Success Criteria

- `pnpm tauri dev` runs: NO dock icon, NO visible window, tray icon present in menubar.
- Tray → Quit terminates process. Tray menu opens on left click.
- `cargo check` zero errors; project tree matches Related Code Files.

Verify: manual run + screenshot of menubar; `ps` confirms process alive with no window.

## Risk Assessment

- **Template icon renders blank/black** (likelihood M, impact L): non-monochrome asset breaks template mode → fallback `icon_as_template(false)` or default window icon until asset authored.
- **Scaffold generator refuses non-empty dir** (M, L): plans/ dir exists → generate in temp dir, move files in.
- **Accessory policy hides app from Cmd+Tab making dev debugging odd** (expected behavior, L): use tray Quit or `kill` during dev.

## Security Considerations

- None substantive this phase. No secrets, no network, no permissions. Identifier choice has downstream TCC security implications (documented above).

## Next Steps

- Phase 02 (windows + frontend shell) — depends on this scaffold; wires the tray "settings" stub to a real window.
- Optional: `git init` if version control wanted (currently not a repo — user decision).
