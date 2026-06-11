---
title: "Translate On Air — macOS Background Translation App"
description: "Tauri v2 menubar agent: global hotkey captures selected text via synthetic Cmd+C, frameless popup auto-translates via local Claude CLI or OpenAI, with Japanese furigana rendering."
status: pending
priority: P2
effort: 30h
branch: "none (not a git repo)"
tags: [tauri, rust, react, macos, translation, openai, claude]
created: 2026-06-11
---

# Translate On Air — Implementation Plan

Native macOS background translator. Hotkey (default `Cmd+Shift+T`) in any app → selected text captured via synthetic Cmd+C → frameless always-on-top popup auto-translates → result shown. No dock icon; minimal menubar tray (Settings + Quit). Target langs: vi (default), ja (with furigana), en.

**Model directive:** Implementation to be executed by **Opus 4.8 (claude-opus-4-8)**. Planning/research done by Fable 5. No code implemented yet.

## Key Architecture Decisions

- **Background agent**: `app.set_activation_policy(ActivationPolicy::Accessory)` in `setup()` (works dev+bundle); `LSUIElement` in Info.plist for bundle. Tray via core `tray-icon` feature (no plugin), `icon_as_template(true)`.
- **Windows**: popup + settings pre-declared in `tauri.conf.json` (`visible:false`; popup frameless, `alwaysOnTop`); `show()/hide()` only — never destroy. ESC hides popup. Capture runs BEFORE popup focus (synth Cmd+C must hit the frontmost app).
- **Text capture**: core-graphics 0.25 `CGEvent` synth Cmd+C (`kVK_ANSI_C=8` + Command flag); clipboard save → poll-read → restore (text-only restore v1). Needs Accessibility permission: detect via `AXIsProcessTrusted`, deep-link guidance to System Settings pane.
- **Provider abstraction**: Rust `Provider` enum (`ClaudeCli | OpenAi`) — KISS enum dispatch, no dyn trait. Auto-resolve (user-confirmed order): Keychain OpenAI key present → OpenAi; else Claude binary+creds detected → ClaudeCli; else `NotConfigured` (Settings prompts user). OpenAI-first chosen for instant-popup UX; Claude is zero-config fallback. All LLM calls + secrets stay in Rust; API key never reaches React.
- **Furigana (ja only)**: shared JSON schema `{translation, segments:[{surface, reading}]}`; Rust validation (reconcat check, katakana→hiragana normalize, kana-segment filter, graceful degrade to plain text); React `<ruby><rt>` rendering. vi/en = plain text.

## ⚠ Claude vs OpenAI Trade-off (surface to users)

Claude CLI headless ≈ **5s wall + ~67k overhead tokens/call** (measured) vs OpenAI gpt-4o-mini ≈ **1–2s**. From **2026-06-15**, `claude -p` on subscription plans draws from a separate capped Agent SDK credit pool. OpenAI fits the "instant popup" UX better; Claude is zero-config for subscribers. **DECIDED:** Auto prefers **OpenAI first**, Claude CLI as fallback (latency-driven); Settings allows manual override either way. Document in README + Settings UI hint.

## Phases

| # | Phase | Status | Est | Description |
|---|-------|--------|-----|-------------|
| 01 | [Scaffold, activation policy, tray](phase-01-scaffold-activation-policy-tray.md) | ✅ done | 3h | Tauri v2 + React/TS scaffold, Accessory policy (no dock), tray with Settings/Quit |
| 02 | [Windows + frontend shell](phase-02-windows-frontend-shell.md) | ✅ done | 3h | Hidden pre-declared popup/settings windows, per-window React entry, show/hide commands |
| 03 | [Hotkey, text capture, accessibility](phase-03-global-shortcut-text-capture-accessibility.md) | ✅ done | 4h | Global shortcut + dynamic rebind, CGEvent Cmd+C synth, clipboard save/restore, AX guidance |
| 04 | [Translation providers + secure key](phase-04-translation-providers-secure-key.md) | ✅ done | 6h | Provider enum (Claude CLI / OpenAI), Auto resolver, Keychain key, furigana schema + validation |
| 05 | [Popup UI + ruby rendering](phase-05-react-popup-ui-ruby-rendering.md) | ✅ done | 4h | Source/result UI, `<ruby>` furigana, lang switcher re-translate, copy, loading/error, ESC |
| 06 | [Settings + persistence + rebind](phase-06-settings-window-persistence-rebind.md) | ✅ done | 4h | Hotkey recorder, lang/provider/model prefs, masked key input, live shortcut rebind |
| 07 | [Testing](phase-07-testing.md) | ✅ done | 4h | Rust unit (resolver/furigana/accelerator), frontend (ruby/state), manual flow checklist |
| 08 | [Packaging + signing](phase-08-packaging-signing.md) | ◐ config done | 2h | LSUIElement + min macOS set; bundle/sign/notarize **deferred** (needs Apple Developer ID) |

Execution: sequential 01→07; 08 deferred. 03 ∥ 04 possible (distinct modules) IF `lib.rs` wiring coordinated — default sequential (KISS).

## Dependencies / Versions

- Rust: tauri 2.11.x (`tray-icon`, `image-png`), tauri-plugin-global-shortcut 2.3.2, tauri-plugin-clipboard-manager 2.3.2, tauri-plugin-store 2.4.3, core-graphics 0.25, macos-accessibility-client 0.0.2, keyring 3 (`apple-native`), reqwest (json), tokio, serde/serde_json.
- Frontend: React 18 + TypeScript + Vite; vitest + @testing-library/react (phase 07).
- Conventions: Rust modules snake_case, TS files kebab-case, code files <200 LOC, immutable TS patterns.

## Unresolved Questions

1. **Claude latency/credit pool** (provider order RESOLVED → Auto prefers OpenAI, Claude fallback): still handle Claude path's post-2026-06-15 Agent SDK quota-exhausted `is_error` gracefully + surface in UI; `--system-prompt` overhead-trim worth measuring only if user manually selects Claude.
2. **keyring v3 vs v4**: v4 split into keyring-core + platform stores, sparse docs — pinned v3 (`apple-native`); revisit v4 when ecosystem settles.
3. **Clipboard restore is text-only**: prior image/file clipboard contents lost on capture — accepted v1 trade-off; NSPasteboard full-content restore = future upgrade.
4. **Dev-mode Accessibility re-grant**: unsigned `tauri dev` binaries break TCC grant each rebuild — workaround: grant AX to launching terminal, or free Apple Development cert (phase 08).
5. **`claude --json-schema` min version**: works on v2.1.173; older installs may lack flag — feature-detect via `--help` grep, fall back to prompt-JSON + fence strip.
6. **Popup position**: v1 = centered (config). Cursor-position spawn nicer but needs mouse-location query + multi-monitor math — decide post-v1 usage feedback.

## Validation Summary

**Validated:** 2026-06-11 · **Questions asked:** 4 (+ provider-order decided pre-validation)

### Confirmed Decisions
- **Provider Auto order:** OpenAI-first → Claude fallback (latency-driven; plan.md + phase-04 updated).
- **Furigana scope:** REVISED 2026-06-11 post-demo → **both** source + output. Furigana on the SOURCE whenever it's Japanese (reading JA→VI/EN) AND on the OUTPUT when target=ja. One LLM call returns `segments` (output) + `source_segments` (source); Rust validates each. (Originally "target=ja only"; user changed after seeing real reading flow.)
- **Settings access:** added ⚙ gear in popup (top-right) in addition to the menubar tray.
- **OpenAI default model:** `gpt-4o-mini` — confirms plan.
- **Dev Accessibility:** unsigned dev build + grant AX to launching terminal (free, KISS) — confirms phase 03/08.
- **No-selection:** show "select text first" hint, no manual-input box — confirms plan.

### Action Items
- [x] Flip provider Auto order → OpenAI-first (plan.md + phase-04) — DONE.
- [ ] No further phase-file changes; all other answers confirm existing defaults.

**Status:** validated — ready for implementation (Opus 4.8).
