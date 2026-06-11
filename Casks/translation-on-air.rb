cask "translation-on-air" do
  version "0.1.0"
  # Unsigned builds change hash every release; pin a real sha256 once you ship
  # notarized artifacts. :no_check is fine for a personal tap.
  sha256 :no_check

  # NOTE: verify this filename matches the actual release asset after the first
  # `vX.Y.Z` tag (Tauri names it from productName + version + arch).
  url "https://github.com/bachdx2812/translation-on-air/releases/download/v#{version}/Translate%20On%20Air_#{version}_universal.dmg"
  name "Translate On Air"
  desc "Menubar translation app with a global hotkey and Japanese furigana"
  homepage "https://github.com/bachdx2812/translation-on-air"

  app "Translate On Air.app"

  caveats <<~EOS
    Translate On Air needs Accessibility permission to read the selected text:
      System Settings → Privacy & Security → Accessibility → enable "Translate On Air".

    Builds are not notarized yet. If macOS blocks the app on first launch:
      right-click the app → Open, or run
      xattr -dr com.apple.quarantine "/Applications/Translate On Air.app"
  EOS
end
