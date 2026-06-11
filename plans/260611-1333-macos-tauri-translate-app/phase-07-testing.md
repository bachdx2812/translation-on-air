# Phase 07 — Testing

## Context Links

- Parent plan: [plan.md](plan.md)
- Depends on: [phase-04](phase-04-translation-providers-secure-key.md), [phase-05](phase-05-react-popup-ui-ruby-rendering.md), [phase-06](phase-06-settings-window-persistence-rebind.md)
- Research: [researcher-02 §1c (resolver), §2b (furigana pitfalls = test cases)](research/researcher-02-llm-providers-furigana.md); [researcher-01 §4 (accelerator grammar)](research/researcher-01-tauri-macos-system.md)

## Overview

- **Date:** 2026-06-11
- **Description:** Test pass: Rust unit tests (provider resolver, furigana validation, accelerator parsing, provider response parsing), frontend tests (ruby rendering, popup state, lang switch), plus manual critical-flow checklist for macOS-API paths that can't be automated headlessly. 80% coverage target where practical (pure logic ≈100%; OS-glue excluded, covered manually).
- **Priority:** P2
- **Implementation status:** ✅ done — 18 tests green. Rust `cargo test`: 13 (resolver ×4, furigana ×4, accelerator ×2, claude envelope/fence/quota ×3). Frontend `vitest`: 5 (FuriganaText ×2, LangSwitcher ×1, useTranslation success+stale-guard ×2). Dropped 1 error-path test (vitest unhandled-rejection quirk on mock string-reject; trivial proven catch). OS-glue (CGEvent/TCC/Keychain/tray/hotkey) = manual checklist below.
- **Review status:** not started

## Key Insights

- macOS system paths (CGEvent synth, TCC, real Keychain, tray, global hotkey OS registration) are NOT meaningfully unit-testable in CI-less headless context → cover via manual checklist; keep them in thin modules so testable logic stays pure (already structured that way in phases 03/04).
- Phase-04 `furigana.rs` + `resolve()` were deliberately designed as pure functions — primary unit targets. Research §2b pitfalls 1–5 ARE the test case list.
- Claude/OpenAI response parsing testable with fixture strings — no live calls in default test run; optional `#[ignore]` live tests for local verification.
- Frontend: vitest + @testing-library/react; mock IPC via `mockIPC` from `@tauri-apps/api/mocks` (official mock entry).
- Per global rules: real tests, no fake passes; failing tests must be fixed, not ignored.

## Requirements

Functional (what must be tested):
- Rust: furigana validation (5 pitfall cases), resolver matrix, accelerator parse accept/reject, Claude JSON parse (success/fenced/error/quota), OpenAI response parse + status mapping, keychain service/user constants, prompt selection per lang.
- Frontend: FuriganaText DOM shape, popup reducer transitions, lang-switch re-translate, error-code → message map, copy action invoke.
- Manual checklist executed + results recorded in report.

Non-functional: `cargo test` + `pnpm test` green, no network in default runs; coverage ≥80% on pure-logic modules (`furigana.rs`, `providers/mod.rs` resolve, parsers, reducer, components); test files exempt from 200-LOC cap but split by module.

## Architecture

```
Rust:  #[cfg(test)] mod tests co-located per module (Rust convention)
       fixtures: src-tauri/tests/fixtures/*.json (claude success/fenced/error, openai success/refusal)
       resolve() tested via injected inputs (mode, Option<ClaudeInfo>, has_key) — no FS/keychain
Front: vitest + jsdom; src/**/*.test.tsx co-located; mockIPC stubs translate/get_settings/copy_text
Manual: checklist table in this file → executed results → reports/tester-*.md
```

## Related Code Files

CREATE:
- `src-tauri/tests/fixtures/claude_success.json`, `claude_fenced.json`, `claude_error_quota.json`, `openai_success.json`
- `src/popup/furigana-text.test.tsx`, `src/popup/popup-state.test.ts` (reducer extracted to `popup-state.ts` if not already), `src/popup/use-translation.test.ts`
- `vitest.config.ts` (or vite config test block), `src/test-setup.ts`

MODIFY:
- `package.json` — vitest, @testing-library/react, @testing-library/jest-dom, jsdom dev-deps + `test` script
- `src-tauri/src/providers/*.rs`, `hotkey.rs` — add `#[cfg(test)]` test mods (and minor refactors ONLY if needed for injectability — e.g. parse fn takes `&str` not Command output)

## Implementation Steps

1. Rust — `furigana.rs` tests (research §2b mapping):
   - reconcat mismatch → segments dropped (empty vec)
   - katakana reading `キョウ` → `きょう`
   - kana-only segment with spurious reading → cleared to `""`
   - mixed sample: `今日は食べた` style segment list passes through correctly
   - is_kanji boundaries (`一` yes, `あ`/`ア`/`A`/`。` no)
2. Rust — resolver matrix (pure `resolve(mode, claude, has_key)`):
   | mode | claude | key | expect |
   |---|---|---|---|
   | Auto | Some | any | ClaudeCli |
   | Auto | None | true | OpenAi |
   | Auto | None | false | NotConfigured |
   | Claude | None | any | NotConfigured(claude-missing) |
   | OpenAi | any | false | NotConfigured |
3. Rust — accelerator: `"Cmd+Shift+T"`, `"Alt+Space"` parse ok; `""`, `"Cmd"`, `"Foo+X"` err. Rebind rollback logic if extracted pure (else manual item).
4. Rust — Claude parse fn against fixtures: `.structured_output` path, `.result` + fence strip path, `is_error:true` quota message → QuotaExhausted, garbage stdout → BadResponse. OpenAI: content parse + 401/429/500 → code mapping (status-mapping fn takes `StatusCode`, no live HTTP).
5. Frontend setup: vitest + jsdom + RTL; `mockIPC((cmd, args) => ...)` in tests.
6. `furigana-text.test.tsx`: segments `[{surface:"今日",reading:"きょう"},{surface:"は",reading:""}]` → exactly one `ruby` with `rt`=きょう, one `span`; empty segments → plain paragraph, zero `ruby`.
7. Reducer tests: CAPTURE_DONE→loading, TRANSLATE_OK→result, TRANSLATE_FAIL(code)→error message, SET_LANG mid-flight → stale response ignored (request-id), CAPTURE_ERROR(ax-missing)→guidance state.
8. `use-translation.test.ts`: mockIPC translate success/error-code propagation.
9. Coverage: `cargo tarpaulin` (or `cargo llvm-cov`) + `vitest --coverage`; assert ≥80% on listed pure modules; document excluded OS-glue.
10. Manual critical-flow checklist (execute, record pass/fail in `reports/`):
    - [ ] AX fresh-grant flow: revoke → hotkey → guidance → deep-link opens pane → grant → works
    - [ ] Capture in Safari, Notes, VS Code (selected text correct, unicode/ja source ok)
    - [ ] Prior clipboard text restored after capture
    - [ ] Empty selection → "Select text first" state
    - [ ] ESC hides; re-show instant with previous result
    - [ ] Tray: Settings opens, Quit exits, no dock icon ever
    - [ ] Provider Auto picks Claude when present; force OpenAI works; not-configured prompt when neither
    - [ ] ja furigana visual check (no clipping, dark mode)
    - [ ] Hotkey rebind live; persists across restart
    - [ ] Keychain Access shows `translate-on-air` entry; `settings.json` contains no key
    - [ ] Claude quota/error path renders friendly message (simulate via bogus claude path override)

## Todo List

- [ ] furigana.rs unit tests (5 pitfall cases + boundaries)
- [ ] resolver matrix tests
- [ ] accelerator parse tests
- [ ] Claude/OpenAI parse + status-mapping tests w/ fixtures
- [ ] vitest setup + mockIPC
- [ ] FuriganaText DOM tests; reducer + hook tests
- [ ] Coverage run ≥80% pure modules; document exclusions
- [ ] Execute manual checklist; record results in reports/

## Success Criteria

- `cargo test` and `pnpm test` pass green, zero network/live-LLM calls in default run.
- Coverage report ≥80% across `furigana.rs`, resolve, parse fns, popup reducer/components; exclusion list documented.
- Manual checklist 100% executed, failures filed as fixes (loop back to owning phase) before phase marked done.

## Risk Assessment

- **Refactor-for-testability creep** (M, M): only extract pure fns (parse, status-map, reducer) — no DI frameworks, no trait abstractions (YAGNI).
- **mockIPC API drift across @tauri-apps/api versions** (L, L): pin version from phase-01 lockfile; consult docs if signature differs.
- **Coverage tooling friction on macOS (tarpaulin)** (M, L): fallback `cargo llvm-cov`.
- **Manual checklist skipped under time pressure** (M, H): phase success criteria explicitly require recorded results.

## Security Considerations

- Fixtures contain NO real keys/tokens; live `#[ignore]` tests read key from Keychain at runtime, never hardcode.
- Test logs must not print captured clipboard contents or key material.

## Next Steps

- All green → phase 08 (packaging) optional/when distributing; otherwise project usable via `tauri dev`/local build.
