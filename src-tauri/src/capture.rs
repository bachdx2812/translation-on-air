//! Capture the frontmost app's current selection by synthesizing Cmd+C, reading
//! the clipboard, then restoring the prior clipboard contents.

use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tauri_plugin_clipboard_manager::ClipboardExt;

#[derive(Debug)]
pub enum CaptureError {
    SynthFailed,
    NoSelection,
}

#[derive(Clone, serde::Serialize)]
struct CaptureDonePayload {
    text: String,
}

#[derive(Clone, serde::Serialize)]
struct CaptureErrorPayload {
    code: &'static str,
}

/// Post a synthetic Cmd+C (key down + up) to the HID event tap. Targets whatever
/// app is frontmost — so the popup must NOT be focused when this runs.
#[cfg(target_os = "macos")]
fn synth_cmd_c() -> Result<(), CaptureError> {
    use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation, CGKeyCode};
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    const KEY_C: CGKeyCode = 8; // kVK_ANSI_C

    let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState)
        .map_err(|_| CaptureError::SynthFailed)?;

    let key_down = CGEvent::new_keyboard_event(source.clone(), KEY_C, true)
        .map_err(|_| CaptureError::SynthFailed)?;
    key_down.set_flags(CGEventFlags::CGEventFlagCommand);
    key_down.post(CGEventTapLocation::HID);

    let key_up =
        CGEvent::new_keyboard_event(source, KEY_C, false).map_err(|_| CaptureError::SynthFailed)?;
    key_up.set_flags(CGEventFlags::CGEventFlagCommand);
    key_up.post(CGEventTapLocation::HID);

    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn synth_cmd_c() -> Result<(), CaptureError> {
    Err(CaptureError::SynthFailed)
}

/// Capture the current selection. Strategy: clear clipboard → synth Cmd+C → poll
/// for non-empty clipboard → restore prior contents. Clearing first lets us tell
/// "copied a selection" from "nothing selected" without false positives from
/// stale clipboard text.
pub async fn capture_selection(app: &AppHandle) -> Result<String, CaptureError> {
    let prev = app.clipboard().read_text().ok();
    let _ = app.clipboard().write_text(String::new());

    synth_cmd_c()?;

    let captured = poll_for_nonempty(app).await;

    // Restore prior clipboard text (best-effort; text-only restore in v1).
    match &prev {
        Some(p) => {
            let _ = app.clipboard().write_text(p.clone());
        }
        None => {
            let _ = app.clipboard().write_text(String::new());
        }
    }

    captured.ok_or(CaptureError::NoSelection)
}

/// Poll the clipboard up to ~500ms (10 × 50ms) for the synthetic copy to land.
/// Polling beats a fixed sleep — fast apps return immediately, slow ones get time.
async fn poll_for_nonempty(app: &AppHandle) -> Option<String> {
    for _ in 0..10 {
        tokio::time::sleep(Duration::from_millis(50)).await;
        if let Ok(text) = app.clipboard().read_text() {
            if !text.is_empty() {
                return Some(text);
            }
        }
    }
    None
}

/// Hotkey pipeline: gate on Accessibility, capture the selection, emit the result
/// to the popup webview, then show the popup. Runs off the hotkey handler thread.
pub async fn run_capture_pipeline(app: &AppHandle) {
    if !crate::accessibility::is_trusted(false) {
        // Surface the system prompt once; guide the user via the popup regardless.
        let _ = crate::accessibility::is_trusted(true);
        emit_error(app, "ax-missing");
        let _ = crate::windows::show_popup_inner(app);
        return;
    }

    match capture_selection(app).await {
        Ok(text) => {
            let _ = app.emit_to("popup", "capture-done", CaptureDonePayload { text });
        }
        Err(CaptureError::NoSelection) => emit_error(app, "no-selection"),
        Err(CaptureError::SynthFailed) => emit_error(app, "capture-failed"),
    }

    let _ = crate::windows::show_popup_inner(app);
}

fn emit_error(app: &AppHandle, code: &'static str) {
    let _ = app.emit_to("popup", "capture-error", CaptureErrorPayload { code });
}
