# Phase 04 — Translation Providers + Secure Key Storage

## Context Links

- Parent plan: [plan.md](plan.md)
- Depends on: [phase-01](phase-01-scaffold-activation-policy-tray.md) (scaffold); independent of 02/03 except final `lib.rs` wiring
- Research: [researcher-02 §1 (providers), §2a–2b (furigana schema + validation)](research/researcher-02-llm-providers-furigana.md); [researcher-01 §6 (keyring)](research/researcher-01-tauri-macos-system.md)

## Overview

- **Date:** 2026-06-11
- **Description:** Rust translation core: `Provider` enum (ClaudeCli | OpenAi), Auto resolution (OpenAI key → Claude binary+creds → NotConfigured), Claude CLI subprocess provider, OpenAI reqwest provider with strict json_schema, Keychain key storage (keyring v3), shared furigana JSON schema + Rust validation, single `translate` Tauri command. Largest phase — secrets never leave Rust.
- **Priority:** P1 (core value)
- **Implementation status:** ✅ done — cargo check + tsc clean; 8 unit tests pass (furigana validation ×4, resolver ×4). Provider enum + Auto resolver (OpenAI-first), Claude CLI subprocess, OpenAI reqwest strict json_schema, keyring v3 Keychain, furigana validate, all commands wired. Live LLM call = manual (no key set; Claude quota not burned unprompted).
- **Review status:** not started

## Key Insights

- **User decision (revised)**: Auto prefers **OpenAI first** (≈1–2s, fits instant-popup UX), Claude CLI is zero-config **fallback** for subscribers. Original "Claude-first" idea dropped after latency trade-off (≈5s + 67k tok/call). Settings still lets user force Claude.
- Claude headless: `claude -p --output-format json --json-schema <schema> --model haiku --system-prompt <short>`; prompt via **stdin** (no ARG_MAX/escaping). Parse `.structured_output` (with schema) else `.result` (strip code fences). **Never `--bare`** — it skips OAuth/keychain reads, breaking subscription auth.
- Detection: GUI apps get minimal PATH — probe known paths: `/opt/homebrew/bin/claude`, `/usr/local/bin/claude`, `~/.local/bin/claude`, `/Applications/cmux.app/Contents/Resources/bin/claude` (verified locally), + user override setting. Creds: `~/.claude/.credentials.json` OR Keychain item `Claude Code-credentials`.
- Measured: Claude call ≈4.7s wall, ~67k overhead tokens; `--system-prompt` replacement should shrink — measure during impl. From **2026-06-15** subscription `-p` usage draws capped Agent SDK credit pool → map quota errors gracefully.
- OpenAI: chat completions, `gpt-4o-mini` default, `response_format: {type:"json_schema", json_schema:{strict:true,...}}` guarantees schema-valid output. Errors: 401→InvalidKey, 429→RateLimited, 5xx/timeout→Unavailable, parse→BadResponse.
- KISS: enum dispatch over `dyn Trait` (2 providers, avoids async_trait friction). keyring v3 pinned (v4 split = adoption risk).
- Furigana validation entirely in Rust BEFORE UI: fence strip → parse → reconcat check → katakana→hiragana → kana-segment filter → degrade to plain text on any failure. Never block display on segments.

## Requirements

Functional:
- `translate(text, target_lang) -> Translated {translation, segments}` Tauri command; segments populated only for `ja`, validated; vi/en → empty segments.
- Provider modes: Auto | Claude | OpenAI (setting read from store, default Auto). Auto resolution cached; invalidated on settings change (`invalidate_provider_cache` hook for phase 06).
- Keychain commands: `set_openai_key(key)`, `delete_openai_key()`, `has_openai_key() -> bool`. Key NEVER returned to frontend.
- `detect_providers()` command → `{claude_detected, claude_path, claude_supports_json_schema, has_openai_key, resolved}` for Settings UI.
- Errors surfaced as stable string codes: `not-configured | invalid-key | rate-limited | quota-exhausted | unavailable | bad-response | timeout`.

Non-functional: Claude timeout 60s, OpenAI 30s; one reused `reqwest::Client`; modules <200 LOC each; comprehensive error mapping; no panics on malformed LLM output.

## Architecture

```
src-tauri/src/providers/
  types.rs      TranslateRequest{text,target_lang} | Translated{translation,segments} |
                Segment{surface,reading} | ProviderError (thiserror, → code strings)
  prompt.rs     system prompt builder per lang + shared JSON schema const
  claude_cli.rs detect() -> Option<ClaudeInfo{path, supports_json_schema}>; translate(req)
  openai.rs     translate(req, key, model)  [reqwest, strict json_schema]
  furigana.rs   validate_segments(translation, segments) -> Vec<Segment>  (pure fns)
  mod.rs        Provider enum + resolve(mode, deps) + ProviderCache (Mutex<Option<..>>)
src-tauri/src/keychain.rs   keyring v3 Entry("translate-on-air","openai_api_key")
src-tauri/src/commands.rs   #[command] translate / detect_providers / set|delete|has_openai_key

flow: popup invoke(translate) ─> resolve(cached) ─> ClaudeCli: tokio Command stdin─prompt → JSON
                                                  └> OpenAi:  reqwest POST → JSON
        ─> ja? furigana::validate_segments ─> Translated → popup
```

## Related Code Files

CREATE (all <200 LOC):
- `src-tauri/src/providers/mod.rs`, `types.rs`, `prompt.rs`, `claude_cli.rs`, `openai.rs`, `furigana.rs`
- `src-tauri/src/keychain.rs`
- `src-tauri/src/commands.rs` — translate + key + detect commands

MODIFY:
- `src-tauri/Cargo.toml` — add `keyring = {version="3", features=["apple-native"]}`, `reqwest = {version="0.12", features=["json"]}`, `thiserror`, `serde`/`serde_json` (present), tokio features (`process`, `time`)
- `src-tauri/src/lib.rs` — `mod` declarations, manage `ProviderCache` state, register commands
- `src/shared/types.ts` — mirror `Translated`, `Segment`, error codes

## Implementation Steps

1. `types.rs`: serde structs + `ProviderError` (thiserror) with `code() -> &'static str` for the 7 codes above. Command layer returns `Err(error.code().to_string())` (KISS; payload msg optional later).
2. `prompt.rs`: const `FURIGANA_SCHEMA: &str` (research §2a JSON schema verbatim); `system_prompt(lang)`:
   - ja → research §2a segmentation prompt (reconcat rule, okurigana split example, hiragana-only, `""` for kana/punct/latin/digit, JSON only no fences)
   - vi/en → "Translate into natural {Vietnamese|English}. Return JSON {translation, segments:[]} with segments always empty. JSON only."
   Same schema both cases (strict-mode friendly).
3. `keychain.rs`:
   ```rust
   const SERVICE: &str = "translate-on-air"; const USER: &str = "openai_api_key";
   pub fn set_key(k: &str) -> Result<...> { keyring::Entry::new(SERVICE, USER)?.set_password(k) }
   // get_key, delete_key (v3: delete_credential), has_key = get_key().is_ok()
   ```
4. `claude_cli.rs`:
   - `detect()`: iterate candidate paths (incl. store override `claude_path_override`) → first existing+executable; creds check = `~/.claude/.credentials.json` exists OR `security find-generic-password -s "Claude Code-credentials"` exit 0; `supports_json_schema` = run `<path> --help`, grep `--json-schema`; cache result in ProviderCache.
   - `translate(req)`: `tokio::process::Command::new(path)` args `["-p","--output-format","json","--model","haiku","--system-prompt",sys]` + `["--json-schema", FURIGANA_SCHEMA]` when supported; `.kill_on_drop(true)`, stdin=piped write user text, `tokio::time::timeout(60s, wait_with_output())`.
   - Parse stdout JSON: check `is_error==false` else map (quota/limit message → QuotaExhausted, else Unavailable); payload from `.structured_output` else `.result` → strip ```` ```json ```` fences → `serde_json::from_str::<Translated>` → BadResponse on fail.
5. `openai.rs`: lazy `OnceLock<reqwest::Client>` (30s timeout). POST `/v1/chat/completions`: `{model, messages:[{role:"system",content:sys},{role:"user",content:text}], response_format:{type:"json_schema", json_schema:{name:"translation", strict:true, schema: FURIGANA_SCHEMA-as-value}}}`. Map status codes per Key Insights; parse `choices[0].message.content` → `Translated`.
6. `furigana.rs` (pure, unit-test target):
   - `is_kanji(c)`: `0x4E00..=0x9FFF | 0x3400..=0x4DBF`
   - `kata_to_hira(s)`: codepoint −0x60 for U+30A1..=U+30F6
   - `validate_segments(translation, segs) -> Vec<Segment>`: if `concat(surfaces) != translation` → `vec![]`; map readings via kata_to_hira; `if !surface.contains(is_kanji) → reading=""`. Return cleaned vec.
7. `mod.rs`: `enum ProviderMode {Auto, Claude, OpenAi}` (from store string); `resolve(mode, claude_info, has_key)` pure fn → `Result<Provider, ProviderError::NotConfigured>` (Auto: openai-key→claude→err; forced modes err if their dep missing). `ProviderCache: Mutex<Option<ClaudeInfo>>` managed state + `invalidate()`.
8. `commands.rs`: `translate` reads mode from store + key from keychain (OpenAI path), calls provider, runs furigana validation when `target_lang=="ja"`, returns `Translated`. Plus `detect_providers`, key commands. Register all in `lib.rs`; `app.manage(ProviderCache::default())`.
9. `src/shared/types.ts`: `interface Segment {surface: string; reading: string}`, `interface Translated {translation: string; segments: Segment[]}`, `type TranslateErrorCode = ...` (7 codes).
10. Verify: `cargo check`; manual `translate` invocations (temp dev button or `cargo test -- --ignored` live tests) against both providers; confirm key absent from any frontend payloads/logs.

## Todo List

- [ ] types.rs structs + ProviderError with stable codes
- [ ] prompt.rs schema const + per-lang system prompts
- [ ] keychain.rs set/get/delete/has (keyring v3 apple-native)
- [ ] claude_cli.rs detect (paths+creds+--help feature probe) + subprocess translate
- [ ] openai.rs reqwest strict-json_schema translate + status mapping
- [ ] furigana.rs pure validation fns
- [ ] mod.rs resolve() + ProviderCache state + invalidate
- [ ] commands.rs translate/detect_providers/key commands; lib.rs wiring
- [ ] types.ts mirror; manual both-provider smoke test
- [ ] Measure Claude latency with --system-prompt; record in report

## Success Criteria

- `translate("hello", "ja")` via Claude AND via OpenAI returns valid `Translated` with reconcat-passing segments; vi/en return empty segments.
- Auto resolution: with OpenAI key present → OpenAi; key absent + claude detected → ClaudeCli; neither → `not-configured` code.
- Malformed/fenced/mismatched LLM output degrades to plain translation (no crash, no empty popup).
- Grep frontend bundle/IPC: OpenAI key string never present. `cargo check` clean.

## Risk Assessment

- **Claude latency >5s hurts UX** (H, M): document in Settings hint; loading state (phase 05); if `--system-prompt` doesn't help, recommend users pick OpenAI — note in unresolved Qs.
- **2026-06-15 Agent SDK credit cap** (H, M): map quota `is_error` → `quota-exhausted` code; popup suggests switching provider.
- **Older claude CLI lacks `--json-schema`** (M, L): `--help` feature-detect → prompt-JSON + fence-strip fallback path (step 4).
- **Claude output JSON shape drifts** (M, M): defensive parser, only rely on `is_error`/`result`/`structured_output`; BadResponse degrade.
- **Strict schema rejection / OpenAI refusal field** (L, L): treat missing content as BadResponse.
- **cmux-bundled binary behaves differently on other machines** (L, L): path override setting + detect() probes standard locations first.

## Security Considerations

- API key only in Keychain via keyring; read inside Rust at call time; never in store JSON, never in IPC responses, never logged.
- Claude subprocess: fixed argv, prompt via stdin (no shell interpolation → no injection); `kill_on_drop` prevents orphan processes.
- `security find-generic-password` called WITHOUT `-w` (presence check only — never read Claude's token).
- Translated text treated as untrusted: React renders as text nodes (no dangerouslySetInnerHTML) — enforced phase 05.

## Next Steps

- Phase 05 popup consumes `translate` + error codes; phase 06 Settings consumes `detect_providers` + key commands + cache invalidation.
