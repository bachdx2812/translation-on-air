//! Update check against the GitHub "latest release". The app ships through
//! Homebrew and is ad-hoc signed, so it only tells the user a newer version
//! exists (tray item + Settings); installing stays `brew upgrade`.

use serde::Serialize;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use tauri::{AppHandle, Manager, State};

const LATEST_RELEASE_API: &str =
    "https://api.github.com/repos/bachdx2812/translation-on-air/releases/latest";
/// Only pages under this prefix may be opened by `open_update_page`.
const RELEASES_PAGE: &str = "https://github.com/bachdx2812/translation-on-air/releases/";
const FIRST_CHECK_DELAY: Duration = Duration::from_secs(20);
const CHECK_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Clone)]
struct Release {
    version: String,
    url: String,
}

/// Result of the latest check, kept so Settings can show it without a request.
#[derive(Default)]
pub struct UpdateState(Mutex<Option<Result<Release, String>>>);

#[derive(Serialize)]
pub struct UpdateStatus {
    current: String,
    /// Newest released version; `None` until a check succeeded.
    latest: Option<String>,
    available: bool,
    /// Why the last check failed, if it did.
    error: Option<String>,
}

fn current_version(app: &AppHandle) -> String {
    app.package_info().version.to_string()
}

fn status(app: &AppHandle, state: &UpdateState) -> UpdateStatus {
    let current = current_version(app);
    let last = state.0.lock().unwrap().clone();
    match last {
        Some(Ok(release)) => UpdateStatus {
            available: is_newer(&release.version, &current),
            latest: Some(release.version),
            current,
            error: None,
        },
        Some(Err(error)) => UpdateStatus {
            current,
            latest: None,
            available: false,
            error: Some(error),
        },
        None => UpdateStatus {
            current,
            latest: None,
            available: false,
            error: None,
        },
    }
}

/// `true` when `latest` ("v0.4.3" or "0.4.3") is a higher x.y.z than `current`.
fn is_newer(latest: &str, current: &str) -> bool {
    fn parse(v: &str) -> Option<[u64; 3]> {
        let mut parts = v.trim().trim_start_matches('v').split('.');
        let mut out = [0; 3];
        for slot in &mut out {
            *slot = parts.next()?.parse().ok()?;
        }
        parts.next().is_none().then_some(out)
    }
    matches!((parse(latest), parse(current)), (Some(l), Some(c)) if l > c)
}

fn parse_release(body: &str) -> Option<Release> {
    let v: serde_json::Value = serde_json::from_str(body).ok()?;
    let tag = v["tag_name"].as_str()?;
    let url = v["html_url"].as_str()?;
    url.starts_with(RELEASES_PAGE).then(|| Release {
        version: tag.trim_start_matches('v').to_string(),
        url: url.to_string(),
    })
}

fn client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .expect("reqwest client builds")
    })
}

async fn fetch_latest(app: &AppHandle) -> Result<Release, String> {
    // GitHub rejects API requests without a User-Agent.
    let resp = client()
        .get(LATEST_RELEASE_API)
        .header(
            "User-Agent",
            format!("translate-on-air/{}", current_version(app)),
        )
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|_| "network".to_string())?;
    if !resp.status().is_success() {
        return Err(format!("http-{}", resp.status().as_u16()));
    }
    let body = resp.text().await.map_err(|_| "network".to_string())?;
    parse_release(&body).ok_or_else(|| "bad-response".to_string())
}

/// Check now, remember the result and refresh the tray's update item.
async fn check(app: &AppHandle) -> UpdateStatus {
    let result = fetch_latest(app).await;
    let state = app.state::<UpdateState>();
    *state.0.lock().unwrap() = Some(result);
    let status = status(app, &state);
    let update = status.available.then(|| status.latest.clone()).flatten();
    let _ = crate::tray::set_update(app, update.as_deref());
    status
}

/// Check shortly after launch, then once a day (the app may run for weeks
/// when it starts at login).
pub fn start_background_checks(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(FIRST_CHECK_DELAY).await;
        loop {
            check(&app).await;
            tokio::time::sleep(CHECK_INTERVAL).await;
        }
    });
}

/// Last known status, without a network request.
#[tauri::command]
pub fn update_status(app: AppHandle, state: State<'_, UpdateState>) -> UpdateStatus {
    status(&app, &state)
}

#[tauri::command]
pub async fn check_for_update(app: AppHandle) -> UpdateStatus {
    check(&app).await
}

/// Open the newest release's page (only a page of this repo's releases).
#[tauri::command]
pub fn open_update_page(app: AppHandle, state: State<'_, UpdateState>) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let url = match state.0.lock().unwrap().as_ref() {
        Some(Ok(release)) => release.url.clone(),
        _ => format!("{RELEASES_PAGE}latest"),
    };
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::{is_newer, parse_release};

    #[test]
    fn compares_versions_numerically() {
        assert!(is_newer("v0.4.3", "0.4.2"));
        assert!(is_newer("0.10.0", "0.9.9"));
        assert!(is_newer("1.0.0", "0.99.99"));
        assert!(!is_newer("v0.4.2", "0.4.2"));
        assert!(!is_newer("0.4.1", "0.4.2"));
    }

    #[test]
    fn ignores_versions_it_cannot_read() {
        assert!(!is_newer("v0.5.0-beta.1", "0.4.2"));
        assert!(!is_newer("nightly", "0.4.2"));
        assert!(!is_newer("0.5", "0.4.2"));
    }

    #[test]
    fn reads_tag_and_page_from_the_release_json() {
        let body = r#"{"tag_name":"v0.4.3","html_url":"https://github.com/bachdx2812/translation-on-air/releases/tag/v0.4.3","draft":false}"#;
        let release = parse_release(body).unwrap();
        assert_eq!(release.version, "0.4.3");
        assert!(release.url.ends_with("/tag/v0.4.3"));
    }

    #[test]
    fn rejects_pages_outside_this_repo() {
        let body = r#"{"tag_name":"v9.9.9","html_url":"https://example.com/evil"}"#;
        assert!(parse_release(body).is_none());
    }
}
