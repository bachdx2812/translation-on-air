# Phase 06 — Settings Window, Persistence, Live Rebind

## Context Links

- Parent plan: [plan.md](plan.md)
- Depends on: [phase-02](phase-02-windows-frontend-shell.md) (settings window), [phase-03](phase-03-global-shortcut-text-capture-accessibility.md) (`rebind`), [phase-04](phase-04-translation-providers-secure-key.md) (key/detect commands, cache invalidation)
- Research: [researcher-01 §4 (rebind), §6 (store + keyring)](research/researcher-01-tauri-macos-system.md); [researcher-02 §1c (provider modes)](research/researcher-02-llm-providers-furigana.md)

## Overview

- **Date:** 2026-06-11
- **Description:** Settings UI + persistence: hotkey recorder with live rebind, default target lang, provider selector (Auto/Claude/OpenAI) with detection status, masked OpenAI key input (Keychain), model name field, optional claude path override. Prefs via tauri-plugin-store; startup reads persisted values.
- **Priority:** P2
- **Implementation status:** ✅ done — tsc + cargo check clean; dev boots with store plugin. settings.rs (store-backed get/set/set_hotkey, defaults, cache invalidation), register_from_settings reads persisted accel w/ fallback, hotkey-recorder + provider-section (key Save/Clear, model, latency hint) + general-section, popup reads persisted lang. Dropped claude_path_override (YAGNI). Live rebind/Keychain/persist = manual verify.
- **Review status:** not started

## Key Insights

- Store (`settings.json` via tauri-plugin-store 2.4.3) for NON-secrets only: `hotkey`, `target_lang`, `provider_mode`, `openai_model`, `claude_path_override`. Secret (OpenAI key) → Keychain only (phase 04 commands).
- Rebind = `unregister(old) + register(new)` (phase 03 `rebind` fn with rollback). Validate accelerator by `parse::<Shortcut>()` server-side — single source of truth.
- Settings UI shows resolved provider status from `detect_providers` (claude path, json-schema support, key presence) — helps user understand Auto.
- Surface the latency trade-off: Claude ≈5s vs OpenAI ≈1–2s; post-2026-06-15 Agent SDK credit pool (plan.md ⚠ section) — one hint line in provider section.
- After any provider-affecting change: invalidate phase-04 `ProviderCache`.

## Requirements

Functional:
- `get_settings() -> Settings` / `set_settings(partial)` commands; Settings = `{hotkey, target_lang, provider_mode, openai_model, claude_path_override, has_openai_key, claude_detected...}` (read model merges store + detection; write model excludes derived fields).
- Hotkey recorder: focus field → press combo → display accelerator (e.g. `Cmd+Shift+T`) → Save calls `set_hotkey(accel)`: parse-validate → rebind live → persist. Invalid/conflict → error shown, old binding kept (rollback).
- Target lang select (vi/ja/en) → persisted default used by popup auto-translate.
- Provider radio Auto/Claude/OpenAI → persisted; status line: "Auto → Claude CLI (≈5s/translation)" or "Auto → OpenAI gpt-4o-mini". Re-detect button.
- OpenAI key: masked input (never prefilled — show "Key saved ✓" when `has_openai_key`), Save → Keychain, Clear → delete. Model text input default `gpt-4o-mini`.
- Startup: `hotkey::register_from_settings` reads persisted accelerator (phase 03 prepared); popup reads persisted target lang (replaces phase 05 constant).

Non-functional: settings apply WITHOUT app restart; files <200 LOC; immutable form state; key never round-trips to UI.

## Architecture

```
settings-view.tsx
 ├─ hotkey-recorder.tsx      keydown capture → accelerator string → set_hotkey
 ├─ provider-section.tsx     radio + detection status + key input + model field + re-detect
 └─ general-section.tsx      target-lang select
src-tauri/src/settings.rs    Settings struct, get_settings/set_settings/set_hotkey commands
                             store helpers (defaults on first run)
flow: UI change → invoke → settings.rs → store.save() (+ keychain / rebind / cache.invalidate())
startup: lib.rs setup → settings::load → hotkey::register_from_settings(accel)
```

Keydown→accelerator mapping (recorder): collect modifiers (`metaKey→Cmd`, `ctrlKey→Ctrl`, `altKey→Alt`, `shiftKey→Shift`) + non-modifier `e.code` key (letters/digits/F-keys); require ≥1 modifier + 1 key; final validation in Rust parse.

## Related Code Files

CREATE (all <200 LOC):
- `src-tauri/src/settings.rs` — Settings struct + store helpers + commands (`get_settings`, `set_settings`, `set_hotkey`)
- `src/settings/hotkey-recorder.tsx`
- `src/settings/provider-section.tsx`
- `src/settings/general-section.tsx`

MODIFY:
- `src-tauri/Cargo.toml` + `lib.rs` — add `tauri-plugin-store = "2"` (2.4.3), register plugin, commands; startup reads store for hotkey registration (touch phase-03 `hotkey.rs` `register_from_settings` to read store)
- `src/settings/settings-view.tsx` — compose sections, load via `get_settings`
- `src/shared/tauri-api.ts` + `src/shared/types.ts` — settings wrappers/types
- `src/popup/popup-view.tsx` — replace constant default lang with `get_settings` value

## Implementation Steps

1. Add store plugin; `settings.rs` defaults: `hotkey="Cmd+Shift+T"`, `target_lang="vi"`, `provider_mode="auto"`, `openai_model="gpt-4o-mini"`, `claude_path_override=null`. `get_settings` merges store values + `keychain::has_key()` + cached detection (cheap).
2. `set_settings(partial)`: write provided keys → `store.save()`; if `provider_mode|openai_model|claude_path_override` changed → `ProviderCache::invalidate()`.
3. `set_hotkey(accel)`: `accel.parse::<Shortcut>()` err → `"invalid-accelerator"`; `hotkey::rebind(app, old, new)` err → `"register-failed"` (old kept via phase-03 rollback); ok → persist + return.
4. Update `hotkey::register_from_settings` to read persisted accelerator at startup (fallback default + log on parse failure — never crash startup on corrupt store).
5. `hotkey-recorder.tsx`: controlled component; on keydown `preventDefault`, build accelerator string, ignore bare modifiers; Esc cancels recording. Show current binding + "Recording…" state. Save → `set_hotkey`, error text on failure.
6. `provider-section.tsx`: radio (Auto/Claude/OpenAI); `detect_providers` on mount + Re-detect button; status line incl. latency hint ("Claude CLI ≈5s per translation; OpenAI ≈1–2s. From 2026-06-15 Claude subscription headless usage draws Agent SDK credits."); masked key input + Save/Clear (calls phase-04 commands; on save → re-detect); model field.
7. `general-section.tsx`: lang select bound to settings.
8. `settings-view.tsx`: load settings on mount, optimistic immutable updates, per-field save (KISS: save-on-change, no global Save button).
9. Popup: read persisted `target_lang` on mount (and on each `capture-done` — cheap invoke — so changed default applies without popup restart).
10. Verify: change hotkey → old combo dead, new combo fires immediately; restart app → all prefs persist; key visible in Keychain Access app under service `translate-on-air`; switching provider mode changes which backend handles next translation (log line).

## Todo List

- [ ] Store plugin + settings.rs defaults/get/set + cache invalidation hooks
- [ ] set_hotkey with parse-validate + live rebind + rollback error codes
- [ ] register_from_settings reads persisted accelerator at startup
- [ ] hotkey-recorder component (record/cancel/error states)
- [ ] provider-section: radio, detection status, latency hint, masked key Save/Clear, model field
- [ ] general-section lang select; settings-view composition
- [ ] Popup uses persisted default lang
- [ ] Persistence + live-rebind + provider-switch manual verify

## Success Criteria

- All prefs survive restart (store file `settings.json` in app data dir; key in Keychain only — confirm `settings.json` contains no key material).
- Rebind applies live; invalid combo ("Cmd" alone, garbage) rejected with message; failed register keeps old binding working.
- Provider switch affects very next translation without restart. `has_openai_key` drives "Key saved ✓" without exposing value.

## Risk Assessment

- **Recorder accelerator string ≠ plugin grammar** (M, M): Rust parse is the validator; recorder restricted to letters/digits/F-keys + modifiers; reject otherwise.
- **Hotkey conflict with system/other app** (M, L): register Err → surfaced `"register-failed"`, rollback to old.
- **Corrupt/partial store file** (L, M): defaults fallback per key; never panic at startup (step 4).
- **Settings drift between open settings window and popup defaults** (M, L): popup re-reads on each capture (step 9).

## Security Considerations

- Key write-only from UI: masked input, never prefilled, never returned by `get_settings` (only boolean). Keychain ACL ties entry to app.
- `claude_path_override` executed as subprocess — validate path exists + is file before use (phase 04 detect already does); document that overriding to a malicious binary is user's own machine risk (no privilege escalation beyond user).

## Next Steps

- Phase 07 tests recorder mapping, settings round-trip, rebind rollback logic.
