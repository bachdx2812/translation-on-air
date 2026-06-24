# Popup redesign + selection “Dịch” bubble — design

Date: 2026-06-24
Branch: `feat/popup-redesign-and-rightclick-autoenable`

## Background

Two user goals:
1. Redesign the translation popup UI.
2. “Bôi đen text → chuột phải → option Dịch”. Focus: macOS.

### Right-click investigation (root cause)

The v0.1.2 macOS Service (right-click → Services → “Translate with Translate On Air”) was
implemented correctly but the user never saw it. Live debugging on macOS 26.4.1 confirmed the cause:
**macOS defaults third-party Services to DISABLED**; the item was listed in System Settings →
Keyboard → Services with its checkbox unchecked. Ticking it made it appear and work end-to-end.
Falsified along the way: legacy `NSStringPboardType` send type, and app quarantine/Gatekeeper —
neither was the cause. (See memory `macos-services-default-disabled`.)

**macOS gives no API to inject a top-level item into another app’s right-click menu.** The Services
submenu is the only sanctioned cross-app context-menu hook, and it is always nested under “Services >”.

### Decision

Build a **PopClip/DeepL-style floating “Dịch” bubble**: when the user selects text in any app, a
small non-activating button appears near the cursor; clicking it runs the existing translate
pipeline and shows the popup. This is the only way to get an “outside the Services menu” UX. The
existing Service stays as a secondary path; we fix its `NSSendTypes` to the modern UTI and add a
`⌘⌃T` key equivalent, but it is no longer the primary mechanism.

## Part A — Popup redesign (stacked, translation-first)

Replace the side-by-side `panels` layout with a vertical stack; width `680 → 440`.

```
┌─────────────────────────────────────┐
│ [Tiếng Việt][日本語][English]      ⚙ │  header: segmented switcher + gear
├─────────────────────────────────────┤
│  <translation>                       │  primary: ~1.25rem, strong weight
│                              ⧉ Copy   │  copy: floats top-right, on hover
├──────────────── divider ────────────┤
│  <source>                            │  secondary: muted, ~0.9rem, no label
└─────────────────────────────────────┘
                                   ESC
```

- Translation on top (largest, primary), source below (muted, **no “Gốc” label** — user choice).
- Furigana still renders via `FuriganaText` in whichever panel holds Japanese.
- All states (loading / error / capture-error) re-laid for the narrow column.
- Touch only `popup-view.tsx` + rewrite `popup.css`; `POPUP_WIDTH = 440`. No change to
  `use-translation.ts`, providers, or capture pipeline. Keep `light-dark()` auto dark mode.

## Part B — Selection bubble (new feature, macOS-only)

### Components & data flow

```
[global mouse-up monitor] ──drag/dbl-click selection?──▶ [read AX selected text + bounds]
        (Rust, NSEvent)                                          (Rust, AX API)
                                                                     │ non-empty
                                                                     ▼
                                              position + show [bubble window] (non-activating)
                                                                     │ user clicks "Dịch"
                                                                     ▼
                                       invoke translate cmd ▶ run_text_pipeline(text) ▶ [popup]
                                                                     │
                                                              hide bubble
```

### Rust (`src-tauri/src/`)

- **`selection.rs`** (new):
  - Global monitor via `NSEvent::addGlobalMonitorForEventsMatchingMask` for `leftMouseUp`
    (+ track `leftMouseDown`/`leftMouseDragged` to distinguish a drag-select; also accept
    double-click word selection). Debounced.
  - On qualifying mouse-up, read the focused element’s `kAXSelectedTextAttribute` (clean — no
    clipboard disturbance, unlike the hotkey path’s synthetic Cmd+C). If empty → do nothing.
    Best-effort selection bounds via `kAXBoundsForRangeParameterizedAttribute` for positioning;
    fall back to the mouse-up location.
  - Emit/Show the bubble window at the computed screen point. Carry the captured text to the
    bubble (window state or event payload).
- **`bubble` window**: a tiny `WebviewWindow` (label `"bubble"`), borderless, transparent,
  always-on-top, `skipTaskbar`, created warm at setup and hidden. Made a **non-activating panel**
  natively (objc2: `NSWindowStyleMask::NonActivatingPanel`-equivalent + `level = popUpMenu` +
  `collectionBehavior` for all-spaces / over-fullscreen) so showing it does **not** steal focus
  from the user’s app.
- **Commands**: `bubble_translate()` (runs `run_text_pipeline` with the stored selection, hides
  bubble), `hide_bubble()`.
- **Settings flag** `selection_bubble` (bool, default **true**): gates the whole monitor. Toggle
  in settings.
- Keep `services.rs`; update `Info.plist` `NSSendTypes → public.utf8-plain-text` (+ keep legacy)
  and add `NSKeyEquivalent @^t`.

### Frontend (`src/`)

- **`src/bubble/bubble-view.tsx`** + `src/styles/bubble.css`: one pill button “🌐 Dịch”. On click
  → `bubble_translate()`. Esc / blur → `hide_bubble()`. Mounted via a new HTML entry
  (`bubble.html`) like popup/settings.
- **Settings**: add a toggle “Hiện nút Dịch khi bôi đen text” bound to the `selection_bubble` flag.

### Dismissal rules

Hide the bubble on: click elsewhere (next `leftMouseDown` outside it), Esc, a new empty selection,
or a ~4s timeout.

### Permissions

Reading AX selected text requires Accessibility — already granted/managed by `accessibility.rs`.
The global mouse monitor itself does not need extra grants. If AX is missing, the bubble simply
never shows (same fallback as today).

## Error handling

- AX read failure / non-AX app → no bubble (silent; Service + hotkey remain).
- Pipeline errors surface in the popup exactly as today (shared `run_text_pipeline`).
- Bubble window create failure → log; feature degrades to Service/hotkey.

## Testing

- Frontend (vitest + RTL): `bubble-view` renders the button and calls `bubble_translate` on click;
  popup redesign — translation-primary ordering, copy action, state rendering (reuse existing tests,
  add layout assertions). Remember explicit `cleanup` (no `globals:true`).
- Rust: unit-test the AX selected-text parsing / positioning math where isol able; the native
  monitor + panel are verified by manual test on the installed `.app`.
- Manual matrix: TextEdit, Safari, Notes, VS Code (Electron AX), Slack — note where AX selection
  works vs not.

## Risks / open questions

- **AX coverage**: some apps (Terminal, certain Electron/Chromium) don’t expose `AXSelectedText`.
  Bubble won’t show there → Service/hotkey remain the fallback. Acceptable for v1.
- **Focus stealing**: must verify the non-activating panel truly keeps the host app first responder
  (else the selection or subsequent copy breaks). Highest-risk item — build/verify first.
- **Performance**: global mouse monitor must be cheap; only read AX on mouse-up after a real
  selection gesture, never on every move.
- **Multi-monitor / coordinate flip**: AppKit screen coords are bottom-left origin; convert care.

## Build sequence

1. Part A popup redesign (self-contained, low risk) + tests.
2. Bubble window plumbing: warm window + non-activating panel + show/hide commands; verify no focus
   steal (spike).
3. Selection detection: mouse monitor + AX selected text + positioning.
4. Wire bubble click → `run_text_pipeline`; dismissal rules.
5. Settings toggle + `Info.plist` Service cleanup.
6. Build `.app`, manual-test matrix.
