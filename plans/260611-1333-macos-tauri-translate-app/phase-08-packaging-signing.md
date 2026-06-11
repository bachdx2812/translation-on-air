# Phase 08 — Packaging, Signing, Notarization (Later / Optional)

## Context Links

- Parent plan: [plan.md](plan.md)
- Depends on: phases 01–07 complete (ship-ready app)
- Research: [researcher-02 §3 (distribution, TCC persistence)](research/researcher-02-llm-providers-furigana.md); [researcher-01 §1 (LSUIElement)](research/researcher-01-tauri-macos-system.md)

## Overview

- **Date:** 2026-06-11
- **Description:** Production bundle: `.app`/`.dmg` via `tauri build`, `LSUIElement` plist for bundle-level dock hiding, stable code signing (Apple Development for dev, Developer ID + notarization for release), document TCC/Accessibility-across-rebuild caveats. **Marked later/optional** — app fully usable locally without it; do this when distributing.
- **Priority:** P3 (deferred)
- **Implementation status:** ◐ config done, rest deferred — `src-tauri/Info.plist` (LSUIElement=true) added + `bundle.macOS.minimumSystemVersion="12.0"` set; cargo check clean. Production bundle (`tauri build`), code signing, notarization, and `docs/deployment-guide.md` DEFERRED: need an Apple Developer ID ($99/yr) + the user's certs/credentials. App is fully usable now via `pnpm tauri dev`.
- **Review status:** not started

## Key Insights

- **TCC is the key risk**: macOS keys Accessibility grants to stable code-signing identity + bundle ID + path. Tauri default ad-hoc identity (`"-"`) = NEW identity every build → grant silently breaks each rebuild (often shows enabled but dead). Stable cert (Apple Development or Developer ID) fixes it.
- Dev workarounds ranked: (1) grant Accessibility to the terminal/IDE launching `tauri dev` — child inherits "responsible process" attribution, survives rebuilds; (2) sign dev builds with free Apple Development cert; (3) stuck-state cleanup: `sudo tccutil reset Accessibility com.bachdx.translateonair` + remove stale System Settings row + re-grant.
- `LSUIElement` via custom `src-tauri/Info.plist` auto-merged at build — bundle-only effect; dev already covered by Accessory policy (phase 01). Both kept (belt and suspenders).
- Tauri CLI automates codesign → notarytool → staple given env vars: `APPLE_SIGNING_IDENTITY` (or conf `signingIdentity`), notarize via `APPLE_API_KEY`+`APPLE_API_ISSUER` or `APPLE_ID`+`APPLE_PASSWORD`+`APPLE_TEAM_ID`. Distribution requires $99/yr Apple Developer Program.
- Bundle ID + app install path must stay stable forever (TCC).

## Requirements

Functional:
- `pnpm tauri build` produces signed `.app` + `.dmg`; bundled app: no dock icon, tray works, Accessibility grant persists across app updates.
- Unsigned local-use fallback documented (right-click→Open / `xattr -cr`) with its re-grant caveat.

Non-functional: notarization passes Gatekeeper (`spctl -a -vv` accepted); no secrets (cert passwords, API keys) committed — env vars only.

## Architecture

```
src-tauri/Info.plist        LSUIElement=true (merged into bundle plist)
tauri.conf.json > bundle    targets ["app","dmg"], macOS { signingIdentity, minimumSystemVersion }
env (release build)         APPLE_SIGNING_IDENTITY / APPLE_API_KEY + APPLE_API_ISSUER (CI-ready)
docs/deployment-guide.md    build+sign+notarize steps, TCC caveats, dev workarounds
```

## Related Code Files

CREATE:
- `src-tauri/Info.plist` — `<key>LSUIElement</key><true/>`
- `docs/deployment-guide.md` — build/sign/notarize runbook + TCC troubleshooting

MODIFY:
- `src-tauri/tauri.conf.json` — `bundle` section: targets, icons, macOS `signingIdentity` (env-overridable), `minimumSystemVersion`
- `README.md` (if exists by then) — dev AX-grant workaround note

## Implementation Steps

1. Add `src-tauri/Info.plist` with `LSUIElement=true`; verify merged into built bundle's Info.plist.
2. Configure `bundle`: `"targets": ["app", "dmg"]`, final icon set (`tauri icon` from master asset), `"macOS": {"minimumSystemVersion": "12.0"}`.
3. Dev signing: create free Apple Development cert in Xcode → set `signingIdentity` for dev-profile builds → confirm AX grant survives consecutive builds.
4. Release: obtain Developer ID Application cert; export env `APPLE_SIGNING_IDENTITY="Developer ID Application: <name> (<team>)"`; notarization creds via App Store Connect API key envs; `pnpm tauri build` → CLI signs, notarizes, staples automatically.
5. Validate: fresh user account / clean machine: `spctl -a -vv <app>` accepted; first-run AX prompt → grant → hotkey works; update build (bump version, rebuild, replace) → grant persists.
6. Write `docs/deployment-guide.md`: steps above + troubleshooting (`tccutil reset`, stale-entry removal, terminal-grant dev mode) + unsigned distribution caveat.
7. Optional follow-up (explicitly out of scope now, YAGNI): auto-update plugin, launch-at-login.

## Todo List

- [ ] Info.plist LSUIElement + merge verification
- [ ] bundle config (targets, icons, min macOS)
- [ ] Apple Development cert for dev builds; AX persistence verified across rebuilds
- [ ] Developer ID + notarization env setup; release build signed/stapled
- [ ] Clean-machine Gatekeeper + AX-grant-persistence validation
- [ ] deployment-guide.md runbook with TCC troubleshooting

## Success Criteria

- Notarized `.dmg` installs on a clean Mac without Gatekeeper override; app runs as agent (no dock), tray present.
- Accessibility granted once persists across at least 2 subsequent version installs.
- Runbook reproducible by someone else (no tribal knowledge).

## Risk Assessment

- **No Apple Developer membership yet** (?, blocks release path): dev cert path (free) still de-risks TCC; release deferred until membership — why phase is optional.
- **Notarization rejection (hardened runtime/entitlements)** (M, M): Tauri defaults usually pass; if rejected inspect `notarytool log`; CGEvent/AX need no special entitlement (TCC-prompted, not entitlement-gated) — verify empirically.
- **Users with broken stale TCC entries after switching from ad-hoc dev builds** (M, L): documented `tccutil reset` cleanup in guide.

## Security Considerations

- Signing keys/API keys via env or CI secrets only — never in repo/conf committed values.
- Notarization = malware scan gate; do not ship `xattr -cr` advice to end users for release builds (dev-only escape hatch).
- Keep bundle ID stable — changing it orphans Keychain entries (keyring service stays `translate-on-air` — unaffected) and TCC grants.

## Next Steps

- Post-ship backlog (separate plan when needed): auto-update, launch-at-login, cursor-positioned popup, NSPasteboard full clipboard restore.
