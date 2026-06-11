# Research: LLM Providers (Claude CLI + OpenAI), Furigana, macOS Distribution

Date: 2026-06-11 | Stack: Tauri v2 + Rust + React/TS, macOS | Verified locally: claude CLI v2.1.173 at `/Applications/cmux.app/Contents/Resources/bin/claude`, subscription-authed, headless works.

## 1. Provider Abstraction (local Claude first, OpenAI fallback)

### 1a. Claude CLI headless — empirically verified + docs

- Invoke: `claude -p "<prompt>" --output-format json --model haiku`. Prompt can also be piped via **stdin** (10MB cap) — prefer stdin from Rust (no ARG_MAX/escaping issues). [Headless docs](https://code.claude.com/docs/en/headless)
- Flags (verified in `--help` v2.1.173): `-p/--print`, `--output-format text|json|stream-json`, `--system-prompt <p>` (replace default), `--append-system-prompt`, `--model <alias|full>` (`haiku` worked → claude-haiku-4-5), `--fallback-model`, `--allowedTools/--disallowedTools`, `--json-schema <schema>`.
- **JSON shape** (verified): `{"type":"result","subtype":"success","is_error":false,"result":"<text>","stop_reason":"end_turn","session_id":"…","total_cost_usd":0.13,"usage":{…},"permission_denials":[],…}`. Text in `.result`; with `--json-schema`, parsed object in `.structured_output`. Guard parser — full schema not contractually stable ([Introl ref](https://introl.com/blog/claude-code-cli-comprehensive-guide-2025), [ClaudeLog](https://claudelog.com/faqs/what-is-output-format-in-claude-code/)).
- **Detection + auth** (no API key needed on Pro/Max): (1) binary exists — probe candidates `/opt/homebrew/bin/claude`, `/usr/local/bin/claude`, `~/.local/bin/claude`, login-shell `which claude`, + user-overridable path setting (GUI apps get minimal PATH — must probe, don't rely on `$PATH`); (2) creds exist — `~/.claude/.credentials.json` or Keychain item `Claude Code-credentials` (`security find-generic-password -s "Claude Code-credentials"`), both verified present here; (3) optional one-time live probe `claude -p "OK" --output-format json`, check `is_error==false`, cache result. Errors surface as `is_error:true` / nonzero exit.
- **GOTCHA — do NOT use `--bare`**: docs recommend `--bare` for scripts (skips hooks/skills/CLAUDE.md → faster), but bare mode *skips OAuth/keychain reads* — auth then requires `ANTHROPIC_API_KEY`, defeating the subscription use case. Use plain `-p` + `--system-prompt "<short>"` to cut default-context overhead instead. [Headless docs](https://code.claude.com/docs/en/headless)
- **Overhead measured**: trivial haiku call = 4.7s wall, ~67k cache-creation input tokens (Claude Code system prompt/tools), reported `total_cost_usd` $0.13 (subscription users aren't billed; field is API-equivalent). Expect ~3–6s/translation. `--system-prompt` replacement should shrink this; verify during impl.
- **POLICY (4 days out)**: from **2026-06-15**, `claude -p` on subscription plans draws from a separate monthly "Agent SDK credit", not interactive limits ([support article](https://support.claude.com/en/articles/15036540-use-the-claude-agent-sdk-with-your-claude-plan)). Still "free with plan" but capped — surface errors gracefully.
- **Rust spawn**: `tokio::process::Command` (Tauri runs tokio), `.kill_on_drop(true)`, write prompt to stdin, `wait_with_output()` wrapped in `tokio::time::timeout(Duration::from_secs(60))`, `serde_json::from_slice` stdout.

### 1b. OpenAI fallback (reqwest)

- `POST https://api.openai.com/v1/chat/completions`, `Authorization: Bearer <key>`, default model `gpt-4o-mini` (cheap: $0.15/M in, $0.60/M out; supports Structured Outputs since `gpt-4o-mini-2024-07-18`). [Structured Outputs guide](https://platform.openai.com/docs/guides/structured-outputs), [API ref](https://platform.openai.com/docs/api-reference/chat)
- Enforce JSON: `"response_format": {"type":"json_schema","json_schema":{"name":"translation","strict":true,"schema":{…}}}` — guarantees schema-valid output (vs `json_object` mode which only guarantees *some* JSON).
- Key storage: **`keyring` crate v3** (macOS Security framework/Keychain) — mature, standard. Alternative `tauri-plugin-keychain` adds a dependency for no gain. KISS: keyring.
- Client: `reqwest::Client::builder().timeout(Duration::from_secs(30)).build()`, reuse one client. Map errors: 401 → InvalidKey, 429 → RateLimited, 5xx/timeout → Unavailable, parse fail → BadResponse.

### 1c. Recommended design (KISS — enum over dyn trait)

Only 2 providers; enum dispatch avoids `async_trait`/dyn-compat friction:

```rust
pub enum Provider { ClaudeCli(ClaudeCli), OpenAi(OpenAi) }
impl Provider {
    pub async fn translate(&self, req: &TranslateRequest) -> Result<Translated, ProviderError> {
        match self { Self::ClaudeCli(c) => c.translate(req).await, Self::OpenAi(o) => o.translate(req).await }
    }
}
// resolve(Auto): claude binary found + creds present → ClaudeCli
//   else keychain has OpenAI key → OpenAi
//   else Err(NotConfigured) → UI prompts for key. Cache at startup; settings override (Auto|Claude|OpenAI).
```

Both providers return the same `Translated { translation: String, segments: Vec<Segment> }`; provider-specific prompt/schema plumbing stays internal. **Ranking**: ship OpenAI path first (simpler, predictable 1–2s latency), Claude-CLI second (zero-config for subscribers but slower + flag-churn risk — pin a min-version check via `claude --version`).

## 2. Furigana generation + rendering

### 2a. Schema + prompt (works for both providers)

```json
{"type":"object","additionalProperties":false,"required":["translation","segments"],
 "properties":{"translation":{"type":"string"},
  "segments":{"type":"array","items":{"type":"object","additionalProperties":false,
   "required":["surface","reading"],
   "properties":{"surface":{"type":"string"},"reading":{"type":"string"}}}}}}
```

System prompt (shared): *"Translate the user's text into natural Japanese. Then split the translation into consecutive segments whose surfaces concatenate EXACTLY to the translation. For each segment containing kanji, set `reading` to its pronunciation in hiragana only. For segments of pure kana, punctuation, latin, or digits, set `reading` to "". Split okurigana: 食べた → [{"surface":"食","reading":"た"},{"surface":"べた","reading":""}]. Keep non-decomposable readings whole: [{"surface":"今日","reading":"きょう"}]. Output JSON only, no code fences."*

- Claude: pass schema via `--json-schema '<schema>'`, read `.structured_output`. Fallback (older CLI): JSON-only instruction, parse `.result`, strip ```` ```json ```` fences.
- OpenAI: same schema in `response_format.json_schema` with `strict:true`.

### 2b. Pitfalls + mitigation (do all in Rust, before UI)

1. **Malformed/fenced JSON** (Claude text path): strip fences → `serde_json` parse → on fail, degrade to plain `translation` without ruby. Never block display on segments.
2. **Surfaces don't reconcat to translation**: validate `concat(surfaces) == translation`; on mismatch drop segments (plain text). Cheap, deterministic.
3. **Katakana readings**: normalize katakana→hiragana (codepoint −0x60 for U+30A1..U+30F6).
4. **Okurigana lumped** (reading covers whole word incl. kana): cosmetic only — ruby still renders; prompt rule above minimizes it. Don't over-engineer (YAGNI: no MeCab/kuromoji unless quality proves bad — note dictionary tokenizers exist as deterministic upgrade path).
5. **Reading present on kana-only segment**: post-filter `if !surface.chars().any(is_kanji) { reading = "" }`.

### 2c. React rendering ([MDN ruby](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/ruby))

```tsx
const Furigana = ({ segments }: { segments: Segment[] }) => (
  <p lang="ja" className="furigana">
    {segments.map((s, i) => s.reading
      ? <ruby key={i}>{s.surface}<rt>{s.reading}</rt></ruby>
      : <span key={i}>{s.surface}</span>)}
  </p>
);
```

```css
.furigana { line-height: 2.2; font-size: 1.25rem; }   /* headroom so rt doesn't clip */
.furigana rt { font-size: 0.5em; user-select: none; }
ruby { ruby-position: over; -webkit-ruby-position: over; } /* Tauri macOS = WKWebView; prefix for older Safari */
```

Index keys fine — list replaced wholesale per translation, never reordered.

## 3. macOS distribution (later phase — note for planning)

- **Bundle/sign/notarize**: `tauri build` → `.app`/`.dmg`. Sign with Developer ID Application cert via `tauri.conf.json > bundle > macOS > signingIdentity` or `APPLE_SIGNING_IDENTITY` env; notarize via `APPLE_API_KEY`+`APPLE_API_ISSUER` (or `APPLE_ID`+`APPLE_PASSWORD`+`APPLE_TEAM_ID`) — Tauri CLI runs codesign/notarytool/staple automatically. Requires $99/yr Apple Developer for distribution. [Tauri macOS signing](https://v2.tauri.app/distribute/sign/macos/)
- **Accessibility/TCC persistence — the key risk**: TCC keys grants to the app's *stable code-signing identity* + bundle ID + path. Apple DTS confirms ad-hoc signatures (Tauri default identity `"-"`) produce a *new identity every build* → macOS forgets/breaks the Accessibility grant each rebuild (often shows enabled but dead; needs toggle or stale-entry cleanup). Developer ID (or Apple Development cert) gives stable identity → grants persist across versions. [Apple DTS thread](https://developer.apple.com/forums/thread/730043), [Tauri #11085](https://github.com/tauri-apps/tauri/issues/11085) (re-grant after every update, closed not-planned — caused by unstable signing), [Eclectic Light on TCC](https://eclecticlight.co/2023/02/09/should-you-reset-its-database-or-delete-it-the-woes-of-tcc/)
- **Dev-mode caveat confirmed**: `tauri dev` runs unsigned `target/debug/<app>` → expect re-granting Accessibility after rebuilds. Workarounds, ranked: (1) grant Accessibility to the **terminal/IDE** launching `tauri dev` — child process inherits TCC "responsible process" attribution, survives rebuilds; (2) sign dev builds with a free **Apple Development** cert (stable identity, per Apple DTS); (3) cleanup when stuck: `sudo tccutil reset Accessibility <bundle.id>`, remove stale System Settings entry, re-grant ([Macworld fix guide](https://www.macworld.com/article/347452/how-to-fix-macos-accessibility-permission-when-an-app-cant-be-enabled.html)). Keep bundle ID + app path stable always.
- In-app check/request: [`tauri-plugin-macos-permissions`](https://github.com/ayangweb/tauri-plugin-macos-permissions) (checkAccessibilityPermission/request…) — small, fits Tauri v2; or 10-line direct `AXIsProcessTrustedWithOptions` FFI (KISS if it's the only permission needed).
- Personal/no-cert distribution: unsigned/ad-hoc app works via right-click→Open or `xattr -cr`, but Accessibility re-grant pain applies — acceptable for dev only.

## Recommendation (ranked)

1. **Provider**: enum `Provider` + `resolve(Auto)` (Claude-if-binary+creds → OpenAI-if-key → NotConfigured). OpenAI `gpt-4o-mini` + strict `json_schema`; Claude `-p --output-format json --json-schema --model haiku --system-prompt <short>`, stdin prompt, tokio spawn, 60s timeout. No `--bare`.
2. **Furigana**: one shared schema/prompt, Rust-side validation (reconcat check, kana normalize, fence strip), graceful degrade to plain text; `<ruby>` rendering as above. No tokenizer dependency now.
3. **Distribution**: defer; when Accessibility ships, immediately adopt stable signing (Apple Development for dev, Developer ID + notarization for release) and document terminal-grant dev workaround.

## Unresolved questions

1. How much does `--system-prompt` replacement actually cut the ~67k-token/4.7s Claude headless overhead? Measure in impl spike; if still >5s, OpenAI default ranking strengthens.
2. Minimum claude CLI version for `--json-schema` (works on 2.1.173 per docs/help; older user installs may lack it) — feature-detect via `--help` grep or fall back to prompt-JSON.
3. Agent SDK credit quota size post-2026-06-15 (support article paywalled detail) — affects heavy-usage subscribers; handle quota-exhausted `is_error` path.
4. Does cmux-bundled claude binary behave identically to standalone install (creds shared via `~/.claude`)? Verified creds exist + call succeeded here; re-verify on clean machine.
