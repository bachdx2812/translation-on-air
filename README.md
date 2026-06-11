# Translate On Air

macOS menubar app: select text in any app, press a global hotkey, get an instant
translation. Japanese renders with **furigana** (hiragana over kanji). No dock
icon — it lives in the menubar.

Built with **Tauri v2 + Rust + React**.

## Features

- Global hotkey (default `⌘⇧T`) translates the current selection.
- Target languages: Vietnamese (default), Japanese, English — switch inline.
- Furigana on both the source and the translation whenever Japanese is involved.
- Providers: **OpenAI** (fast, needs an API key) or the local **Claude CLI**
  (zero-config if you're signed in). Auto-resolves, OpenAI preferred for speed.
- Source ⟷ translation shown side by side; window auto-sizes to the content.
- Secrets in the macOS Keychain; settings via the in-popup ⚙ or the menubar tray.

## Install (Homebrew)

```sh
brew install --cask bachdx2812/tap/translation-on-air
```

Then grant Accessibility:
**System Settings → Privacy & Security → Accessibility →** enable **Translate On Air**.

> Not notarized yet — if macOS blocks it: right-click the app → **Open**, or run
> `xattr -dr com.apple.quarantine "/Applications/Translate On Air.app"`.

## Usage

1. Select text in any app.
2. Press `⌘⇧T`.
3. The popup shows **source | translation** side by side. Switch language, copy, or
   press `Esc` to hide.

Open **Settings** (⚙ in the popup, or the menubar tray) to change the hotkey,
default language, provider, OpenAI key, and model.

## Development

Requires Rust, Node 20+, and pnpm.

```sh
pnpm install
pnpm tauri dev
```

Dev note: macOS attaches the Accessibility grant to the process that launches
`pnpm tauri dev`, so grant **Accessibility** to your terminal/IDE during development.

Tests:

```sh
cd src-tauri && cargo test   # Rust: provider resolver, furigana, accelerator
pnpm test                    # Frontend: ruby rendering, popup state
```

## Release

Push a tag `vX.Y.Z` → GitHub Actions builds a universal macOS `.dmg` and publishes
a Release (`.github/workflows/release.yml`). Set the `APPLE_*` repo secrets to get
signed + notarized builds; otherwise the `.dmg` is unsigned (see cask caveats).

The Homebrew cask lives in the **`bachdx2812/homebrew-tap`** repo
(`Casks/translation-on-air.rb`). After a release, bump its `version` and `sha256`
(`shasum -a 256` of the new `.dmg`).
