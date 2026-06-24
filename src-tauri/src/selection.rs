//! PopClip-style selection bubble (macOS). A global mouse-up monitor reads the
//! focused element's selected text via the Accessibility API (passive — no
//! clipboard disturbance, unlike capture.rs's synthetic Cmd+C) and shows a small
//! floating "Dịch" button near the cursor. Clicking it runs the translate
//! pipeline. macOS has no API to add a top-level item to another app's
//! right-click menu, so this is how we get an "outside the Services menu" UX.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use tauri::{AppHandle, LogicalPosition, Manager};

/// Whether the bubble feature is on. The global monitor stays installed for the
/// app's lifetime; this flag (flipped by settings) gates whether it acts.
static ENABLED: AtomicBool = AtomicBool::new(true);
/// The most recently captured selection, handed to the pipeline on bubble click.
static SELECTED: Mutex<String> = Mutex::new(String::new());

/// Bubble window size (logical points); keep in sync with tauri.conf.json.
const BUBBLE_W: f64 = 104.0;
const BUBBLE_H: f64 = 40.0;

pub fn set_enabled(on: bool) {
    ENABLED.store(on, Ordering::Relaxed);
    if !on {
        // Nothing to actively tear down; the next mouse-up early-returns.
    }
}

/// Hand the stored selection to the caller (bubble_translate command).
fn take_selected() -> String {
    SELECTED.lock().map(|g| g.clone()).unwrap_or_default()
}

/// Read the focused UI element's selected text via the Accessibility API.
/// Returns None when nothing is selected or the app isn't AX-introspectable.
#[cfg(target_os = "macos")]
unsafe fn read_selected_text() -> Option<String> {
    use accessibility_sys::{
        kAXErrorSuccess, kAXFocusedUIElementAttribute, kAXSelectedTextAttribute,
        AXUIElementCopyAttributeValue, AXUIElementCreateSystemWide, AXUIElementRef,
    };
    use core_foundation::base::{CFRelease, CFTypeRef, TCFType};
    use core_foundation::string::{CFString, CFStringRef};

    unsafe fn copy_attr(elem: AXUIElementRef, attr: &str) -> Option<CFTypeRef> {
        let cf_attr = CFString::new(attr);
        let mut value: CFTypeRef = std::ptr::null();
        let err = AXUIElementCopyAttributeValue(
            elem,
            cf_attr.as_concrete_TypeRef(),
            &mut value as *mut CFTypeRef,
        );
        if err == kAXErrorSuccess && !value.is_null() {
            Some(value)
        } else {
            None
        }
    }

    let system = AXUIElementCreateSystemWide();
    if system.is_null() {
        return None;
    }
    let focused = copy_attr(system, kAXFocusedUIElementAttribute);
    CFRelease(system as CFTypeRef);
    let focused = focused?;

    let text_ref = copy_attr(focused as AXUIElementRef, kAXSelectedTextAttribute);
    CFRelease(focused);
    let text_ref = text_ref?;

    // wrap_under_create_rule takes ownership and releases on drop.
    let s = CFString::wrap_under_create_rule(text_ref as CFStringRef).to_string();
    Some(s)
}

/// Cursor position converted from Cocoa global coords (bottom-left origin) to
/// Tauri's top-left logical coords.
#[cfg(target_os = "macos")]
fn cursor_top_left() -> Option<(f64, f64)> {
    use objc2_app_kit::{NSEvent, NSScreen};
    use objc2_foundation::MainThreadMarker;

    let mtm = MainThreadMarker::new()?;
    let p = NSEvent::mouseLocation();
    let screens = NSScreen::screens(mtm);
    let height = screens.firstObject().map(|s| s.frame().size.height)?;
    Some((p.x, height - p.y))
}

#[cfg(target_os = "macos")]
fn on_mouse_up(app: &AppHandle) {
    if !ENABLED.load(Ordering::Relaxed) {
        return;
    }
    let text = match unsafe { read_selected_text() } {
        Some(t) if !t.trim().is_empty() => t,
        _ => {
            // Selection cleared (e.g. a plain click) → dismiss any open bubble.
            if let Some(win) = app.get_webview_window("bubble") {
                let _ = win.hide();
            }
            return;
        }
    };

    if let Ok(mut g) = SELECTED.lock() {
        *g = text;
    }

    let Some((cx, cy)) = cursor_top_left() else {
        return;
    };
    // Sit the bubble just above-right of the cursor, clamped on-screen.
    let x = (cx + 8.0).max(0.0);
    let y = (cy - BUBBLE_H - 8.0).max(0.0);
    let _ = (BUBBLE_W, x, y);

    if let Some(win) = app.get_webview_window("bubble") {
        let _ = win.set_position(LogicalPosition::new(x, y));
        // show() only (no set_focus): an Accessory app ordering a window front
        // does not steal first-responder from the host app.
        let _ = win.show();
    }
}

/// Install the global mouse-up monitor. Must run on the main thread (setup does).
#[cfg(target_os = "macos")]
pub fn register(app: &AppHandle) {
    use block2::RcBlock;
    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;
    use objc2_app_kit::{NSEvent, NSEventMask};
    use std::ptr::NonNull;

    let app = app.clone();
    let handler = RcBlock::new(move |_event: NonNull<NSEvent>| {
        on_mouse_up(&app);
    });

    // Global monitors only observe events bound for OTHER apps, so clicks on our
    // own bubble never re-trigger this.
    let token: Option<Retained<AnyObject>> =
        NSEvent::addGlobalMonitorForEventsMatchingMask_handler(NSEventMask::LeftMouseUp, &handler);
    // Keep the monitor (and its block) alive for the app's lifetime.
    if let Some(token) = token {
        std::mem::forget(token);
    }
}

#[cfg(not(target_os = "macos"))]
pub fn register(_app: &AppHandle) {}

/// Translate the stored selection and dismiss the bubble (called from the
/// bubble's button).
#[tauri::command]
pub fn bubble_translate(app: AppHandle) {
    if let Some(win) = app.get_webview_window("bubble") {
        let _ = win.hide();
    }
    let text = take_selected();
    if text.trim().is_empty() {
        return;
    }
    tauri::async_runtime::spawn(async move {
        crate::capture::run_text_pipeline(&app, text).await;
    });
}

#[tauri::command]
pub fn hide_bubble(app: AppHandle) {
    if let Some(win) = app.get_webview_window("bubble") {
        let _ = win.hide();
    }
}
