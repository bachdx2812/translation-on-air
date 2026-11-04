//! Launch at login through a per-user LaunchAgent. Works on every supported
//! macOS (12+) without an extra dependency, and the plist's presence *is* the
//! setting, so removing the app from Login Items in System Settings shows up
//! as "off" here too.

use std::path::{Path, PathBuf};
use tauri::AppHandle;

fn plist_path(app: &AppHandle) -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    let file = format!("{}.plist", app.config().identifier);
    Some(PathBuf::from(home).join("Library/LaunchAgents").join(file))
}

pub fn is_enabled(app: &AppHandle) -> bool {
    plist_path(app).is_some_and(|p| p.exists())
}

pub fn set_enabled(app: &AppHandle, on: bool) -> std::io::Result<()> {
    let path = plist_path(app).ok_or_else(|| std::io::Error::other("HOME is not set"))?;
    if on {
        let content = plist(&app.config().identifier, &std::env::current_exe()?);
        write_agent(&path, &content)
    } else {
        remove_agent(&path)
    }
}

fn write_agent(path: &Path, content: &str) -> std::io::Result<()> {
    // Rewriting an unchanged agent would make macOS announce it again.
    if std::fs::read_to_string(path).is_ok_and(|old| old == content) {
        return Ok(());
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, content)
}

fn remove_agent(path: &Path) -> std::io::Result<()> {
    match std::fs::remove_file(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

/// Re-point an enabled agent at this executable, so launch at login keeps
/// working after the app is reinstalled or moved. Skipped in dev builds, which
/// must not point the login item at a binary under `target/`.
pub fn refresh(app: &AppHandle) {
    if cfg!(debug_assertions) {
        return;
    }
    if is_enabled(app) {
        if let Err(e) = set_enabled(app, true) {
            eprintln!("[autostart] refresh failed: {e}");
        }
    }
}

/// `AssociatedBundleIdentifiers` lets System Settings show the app's name and
/// icon for the login item; `Aqua` keeps it out of non-GUI (e.g. ssh) sessions.
fn plist(label: &str, exe: &Path) -> String {
    let label = xml_escape(label);
    let exe = xml_escape(&exe.display().to_string());
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>Label</key>
	<string>{label}</string>
	<key>ProgramArguments</key>
	<array>
		<string>{exe}</string>
	</array>
	<key>RunAtLoad</key>
	<true/>
	<key>ProcessType</key>
	<string>Interactive</string>
	<key>LimitLoadToSessionType</key>
	<string>Aqua</string>
	<key>AssociatedBundleIdentifiers</key>
	<array>
		<string>{label}</string>
	</array>
</dict>
</plist>
"#
    )
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::{plist, remove_agent, write_agent, xml_escape};
    use std::path::Path;

    #[test]
    fn writes_and_removes_the_agent_file() {
        let dir = std::env::temp_dir().join(format!("toa-autostart-{}", std::process::id()));
        let path = dir.join("LaunchAgents/com.example.plist");

        write_agent(&path, "v1").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "v1");
        write_agent(&path, "v2").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "v2");

        remove_agent(&path).unwrap();
        assert!(!path.exists());
        // Turning it off twice is not an error.
        remove_agent(&path).unwrap();

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn plist_runs_the_app_binary_at_login() {
        let p = plist(
            "com.bachdx.translateonair",
            Path::new("/Applications/Translate On Air.app/Contents/MacOS/translate-on-air"),
        );
        assert!(p.contains("<string>com.bachdx.translateonair</string>"));
        assert!(p.contains(
            "<string>/Applications/Translate On Air.app/Contents/MacOS/translate-on-air</string>"
        ));
        assert!(p.contains("<key>RunAtLoad</key>\n\t<true/>"));
    }

    #[test]
    fn escapes_xml_special_characters() {
        assert_eq!(
            xml_escape(r#"A & <B> "C""#),
            "A &amp; &lt;B&gt; &quot;C&quot;"
        );
    }
}
