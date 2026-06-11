# Phase 05 — React Popup UI + Ruby (Furigana) Rendering

## Context Links

- Parent plan: [plan.md](plan.md)
- Depends on: [phase-02](phase-02-windows-frontend-shell.md) (shell/ESC), [phase-03](phase-03-global-shortcut-text-capture-accessibility.md) (events), [phase-04](phase-04-translation-providers-secure-key.md) (`translate` command)
- Research: [researcher-02 §2c (React ruby rendering + CSS)](research/researcher-02-llm-providers-furigana.md)

## Overview

- **Date:** 2026-06-11
- **Description:** Real popup UI: shows captured source text, auto-translates immediately on `capture-done`, renders result (ja = `<ruby><rt>` furigana; vi/en plain), target-lang switcher with re-translate, copy button, loading/error/no-selection/ax-missing states, ESC hide.
- **Priority:** P1 (the product surface)
- **Implementation status:** ✅ done — tsc + cargo check clean. Popup: capture-done/error listeners (subscribe-once + lang ref), auto-translate, FuriganaText <ruby><rt>, lang switcher re-translate (stale-guard), copy via copy_text command, all 8 states, ESC hide. Visual ruby in WKWebView + reducer transitions verified in phase 07.
- **Review status:** not started

## Key Insights

- Ruby rendering: only segments with non-empty `reading` get `<ruby>surface<rt>reading</rt></ruby>`; others plain `<span>`. Index keys fine — list replaced wholesale each translation, never reordered.
- CSS: `.furigana { line-height: 2.2 }` headroom so `<rt>` doesn't clip; `rt { font-size: .5em; user-select: none }`; `ruby-position: over` + `-webkit-ruby-position` (Tauri macOS = WKWebView).
- Auto-translate = popup listens for `capture-done`, immediately invokes `translate` with current default target lang (read once via `get_settings`).
- Re-translate on lang switch reuses stored source text — NO recapture.
- Rust validation already guarantees segments are safe/consistent; UI never re-validates (DRY) but must handle `segments: []` (plain text path).

## Requirements

Functional:
- States (discriminated union): `idle | loading | result | error(code) | no-selection | ax-missing`.
- On `capture-done {text}`: store source, set loading, invoke `translate(text, targetLang)` → result state.
- Source text shown (truncated ~3 lines, scrollable); result below; ja result with furigana when segments present.
- Lang switcher (vi | ja | en) inside popup → sets session target + re-invokes translate (does NOT change persisted default — that's Settings).
- Copy button → copies plain `translation` via Rust command; brief "Copied" feedback.
- Error state: human message per code (e.g. `not-configured` → "No provider configured — open Settings" with button), Retry button.
- `ax-missing`: guidance text + "Open System Settings" (invoke `open_accessibility_settings`).
- ESC hides (phase 02); hiding resets transient feedback but keeps last result (instant re-show OK).

Non-functional: immutable state updates (single `useReducer` or discriminated `useState` object); components <200 LOC; no `dangerouslySetInnerHTML`; loading shows within 50ms of hotkey (perceived responsiveness vs 1–5s provider latency).

## Architecture

```
popup-view.tsx        state machine + event listeners (capture-done / capture-error)
 ├─ use-translation.ts  hook: translate(text, lang) wraps invoke, maps error codes, abort-stale guard
 ├─ furigana-text.tsx   segments → <ruby>/<span>; fallback plain <p> when segments empty
 ├─ lang-switcher.tsx   3 buttons, active highlight, onChange(lang)
 └─ result-actions.tsx  copy button (+ retry inside error block)
styles: src/styles/popup.css (furigana CSS + frameless window chrome, drag region optional)
```

Data flow: Rust event → reducer dispatch → invoke('translate') → reducer result/error → render. Stale-response guard: increment request id; ignore responses with old id (user switched lang mid-flight).

## Related Code Files

CREATE (all <200 LOC):
- `src/popup/use-translation.ts` — hook (invoke wrapper, request-id stale guard, error-code → message map)
- `src/popup/furigana-text.tsx` — ruby renderer (research §2c snippet)
- `src/popup/lang-switcher.tsx`
- `src/popup/result-actions.tsx` — copy (+ feedback)
- `src/styles/popup.css` — furigana + popup chrome

MODIFY:
- `src/popup/popup-view.tsx` — replace placeholder: reducer, event subscriptions (cleanup on unmount), state rendering
- `src/shared/tauri-api.ts` — `translate()`, `copyText()` wrappers
- `src-tauri/src/commands.rs` — add `copy_text(text)` command (reuses ClipboardExt; avoids JS clipboard capability)

## Implementation Steps

1. `furigana-text.tsx` (research §2c):
   ```tsx
   const FuriganaText = ({ segments }: { segments: Segment[] }) => (
     <p lang="ja" className="furigana">
       {segments.map((s, i) => s.reading
         ? <ruby key={i}>{s.surface}<rt>{s.reading}</rt></ruby>
         : <span key={i}>{s.surface}</span>)}
     </p>
   );
   ```
   Caller: `segments.length > 0 ? <FuriganaText/> : <p>{translation}</p>`.
2. `popup.css`: research §2c rules verbatim (`line-height: 2.2`, `rt .5em user-select:none`, `ruby-position: over` + webkit prefix) + minimal frameless chrome (rounded corners, padding, system font).
3. `use-translation.ts`: `translate(text, lang)` → `invoke<Translated>('translate', {text, targetLang: lang})`; catch → map code string to `{code, message}`; `useRef` request counter for stale guard. Immutable returns only.
4. `popup-view.tsx`: `useReducer` with actions `CAPTURE_DONE | CAPTURE_ERROR | TRANSLATE_START | TRANSLATE_OK | TRANSLATE_FAIL | SET_LANG | COPIED`. `useEffect` mount: `listen('capture-done')`, `listen('capture-error')`, load default lang via `get_settings` (phase 06 command; until then constant 'vi'). Cleanup unlisten on unmount.
5. Auto-translate: `CAPTURE_DONE` effect → `TRANSLATE_START` + hook call. `SET_LANG` → re-call with stored source.
6. `result-actions.tsx`: copy → `invoke('copy_text', {text: translation})` → transient "Copied" (setTimeout 1.5s, cleared on unmount/hide).
7. Error rendering map: `not-configured` (+ "Open Settings" → `show_settings` invoke), `quota-exhausted` ("Claude credit exhausted — switch provider in Settings"), `rate-limited`, `timeout`, `unavailable`, `bad-response`, `no-selection` ("Select text first, then press hotkey"), `ax-missing` (guidance + settings deep-link button).
8. Add `copy_text` command in Rust; register.
9. Verify: full flow hotkey→popup→auto-translation for vi/en/ja; furigana ruby visually above kanji; switch lang re-translates; copy puts plain text on clipboard; every error code renders distinct message; ESC hides.

## Todo List

- [ ] FuriganaText component + popup.css ruby rules
- [ ] use-translation hook with stale-response guard + error map
- [ ] popup-view reducer + event subscriptions + auto-translate
- [ ] lang-switcher (session-only target change, re-translate)
- [ ] result-actions copy via Rust `copy_text` + feedback
- [ ] All 8 error/edge states rendered distinctly
- [ ] ESC retained; transient state reset on hide
- [ ] Visual check ja ruby in WKWebView (clipping, dark mode)

## Success Criteria

- Select 日本語 target: kanji shows hiragana above via real `<ruby>` elements (inspect DOM); kana/punct segments have no `<rt>`.
- vi/en: plain text, no ruby markup. Lang switch re-translates without recapture (<1 IPC capture event).
- Copy button → clipboard contains plain translation. Loading indicator visible during provider latency. All error codes human-readable. No mutation patterns (`reducer` pure).

## Risk Assessment

- **rt clipping/overlap on long text** (M, L): line-height 2.2 + popup scroll on overflow; visual check todo.
- **Stale translation overwrites newer (lang switched mid-flight)** (M, M): request-id guard (step 3).
- **Event listener leak across hide/show cycles** (M, M): window persists (never destroyed) → mount once, cleanup only on unmount; verify no duplicate listeners after 10 show/hide cycles.
- **`get_settings` not ready until phase 06** (certain, L): constant default 'vi' + TODO marker; phase 06 swaps in.

## Security Considerations

- LLM output rendered ONLY as React text nodes — no HTML injection path even if model returns markup.
- Popup invokes limited command set (`translate`, `copy_text`, `hide_popup`, `show_settings`, `open_accessibility_settings`); no key/secret commands reachable from popup.

## Next Steps

- Phase 06 supplies persisted default lang (`get_settings`) — replace the constant.
- Phase 07 unit-tests FuriganaText + reducer transitions.
