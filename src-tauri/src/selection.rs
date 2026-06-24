//! PopClip-style selection bubble (macOS). A global mouse-up monitor reads the
//! focused element's selected text via the Accessibility API (passive — no
//! clipboard disturbance, unlike capture.rs's synthetic Cmd+C) and shows a small
//! floating "Dịch" button near the cursor. Clicking it runs the translate
//! pipeline. macOS has no API to add a top-level item to another app's
//! right-click menu, so this is how we get an "outside the Services menu" UX.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use tauri::{AppHandle, Manager, PhysicalPosition};

/// Whether the bubble feature is on. The global monitor stays installed for the
/// app's lifetime; this flag (flipped by settings) gates whether it acts.
static ENABLED: AtomicBool = AtomicBool::new(true);
/// The most recently captured selection, handed to the pipeline on bubble click.
static SELECTED: Mutex<String> = Mutex::new(String::new());
/// Cursor position at the last left-mouse-down, to tell a drag (selection
/// gesture) from a plain click on mouse-up.
static LAST_DOWN: Mutex<Option<(f64, f64)>> = Mutex::new(None);

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
/// Logs the AX error + focused role so failures are diagnosable.
#[cfg(target_os = "macos")]
unsafe fn read_selected_text() -> Option<String> {
    use accessibility_sys::{
        kAXErrorSuccess, kAXFocusedUIElementAttribute, kAXRoleAttribute, kAXSelectedTextAttribute,
        AXUIElementCopyAttributeValue, AXUIElementCreateSystemWide, AXUIElementRef,
    };
    use core_foundation::base::{CFRelease, CFTypeRef, TCFType};
    use core_foundation::string::{CFString, CFStringRef};

    unsafe fn copy_attr(elem: AXUIElementRef, attr: &str) -> Result<CFTypeRef, i32> {
        let cf_attr = CFString::new(attr);
        let mut value: CFTypeRef = std::ptr::null();
        let err = AXUIElementCopyAttributeValue(
            elem,
            cf_attr.as_concrete_TypeRef(),
            &mut value as *mut CFTypeRef,
        );
        if err == kAXErrorSuccess && !value.is_null() {
            Ok(value)
        } else {
            Err(err)
        }
    }

    let system = AXUIElementCreateSystemWide();
    if system.is_null() {
        return None;
    }
    let focused = match copy_attr(system, kAXFocusedUIElementAttribute) {
        Ok(v) => v,
        Err(e) => {
            CFRelease(system as CFTypeRef);
            eprintln!("[bubble] AX focused-element err={e}");
            return None;
        }
    };
    CFRelease(system as CFTypeRef);

    // Diagnostic: what kind of element is focused (helps spot our own webview).
    if let Ok(role) = copy_attr(focused as AXUIElementRef, kAXRoleAttribute) {
        let r = CFString::wrap_under_create_rule(role as CFStringRef).to_string();
        eprintln!("[bubble] focused role={r}");
    }

    let text_ref = match copy_attr(focused as AXUIElementRef, kAXSelectedTextAttribute) {
        Ok(v) => v,
        Err(e) => {
            CFRelease(focused);
            eprintln!("[bubble] AX selected-text err={e}");
            return None;
        }
    };
    CFRelease(focused);

    // wrap_under_create_rule takes ownership and releases on drop.
    let s = CFString::wrap_under_create_rule(text_ref as CFStringRef).to_string();
    Some(s)
}

/// Record the cursor at mouse-down so mouse-up can tell a drag (selection
/// gesture) from a plain click.
#[cfg(target_os = "macos")]
fn on_mouse_down(app: &AppHandle) {
    if !ENABLED.load(Ordering::Relaxed) {
        return;
    }
    if let Ok(p) = app.cursor_position() {
        if let Ok(mut g) = LAST_DOWN.lock() {
            *g = Some((p.x, p.y));
        }
    }
}

#[cfg(target_os = "macos")]
fn on_mouse_up(app: &AppHandle) {
    if !ENABLED.load(Ordering::Relaxed) {
        return;
    }

    // Snapshot the cursor AT mouse-up: the async Cmd+C path runs hundreds of ms
    // later, by which point the pointer has moved — so the bubble must be placed
    // using this position, not a fresh read.
    let up = app.cursor_position().ok().map(|p| (p.x, p.y));

    // Did the pointer travel far enough to be a drag-select (vs a plain click)?
    let dragged = match (LAST_DOWN.lock().ok().and_then(|g| *g), up) {
        (Some((dx, dy)), Some((ux, uy))) => ((ux - dx).powi(2) + (uy - dy).powi(2)).sqrt() > 6.0,
        _ => false,
    };

    // 1) Try the Accessibility API first — passive, no clipboard disturbance.
    let ax = unsafe { read_selected_text() };
    eprintln!(
        "[bubble] mouseUp dragged={dragged} ax={:?}",
        ax.as_deref().map(|t| t.chars().take(30).collect::<String>())
    );
    if let Some(t) = ax {
        if !t.trim().is_empty() {
            show_bubble(app, t, up);
            return;
        }
    }

    // 2) AX gave nothing usable. If the user just drag-selected, fall back to a
    //    synthetic Cmd+C capture (works in browsers/PDF/Electron where AX has no
    //    selected-text). Runs off-thread; clipboard is saved + restored.
    if dragged {
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            match crate::capture::capture_selection(&app).await {
                Ok(t) if !t.trim().is_empty() => {
                    let app2 = app.clone();
                    let _ = app.run_on_main_thread(move || show_bubble(&app2, t, up));
                }
                _ => {
                    let app2 = app.clone();
                    let _ = app.run_on_main_thread(move || hide_bubble_window(&app2));
                }
            }
        });
        return;
    }

    // 3) Plain click, no selection → dismiss any open bubble.
    hide_bubble_window(app);
}

#[cfg(target_os = "macos")]
fn hide_bubble_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("bubble") {
        let _ = win.hide();
    }
}

/// Store the selection and show the bubble next to the drag-release point
/// (`at`, physical global coords captured at mouse-up). Must run on the main
/// thread (callers ensure this).
#[cfg(target_os = "macos")]
fn show_bubble(app: &AppHandle, text: String, at: Option<(f64, f64)>) {
    if let Ok(mut g) = SELECTED.lock() {
        *g = text;
    }
    let Some((cx, cy)) = at.or_else(|| app.cursor_position().ok().map(|p| (p.x, p.y))) else {
        eprintln!("[bubble] no cursor position");
        return;
    };
    let _ = (BUBBLE_W, BUBBLE_H);

    let Some(win) = app.get_webview_window("bubble") else {
        eprintln!("[bubble] ERROR: no 'bubble' window");
        return;
    };

    // Scale the offset to the cursor's monitor so the bubble sits the same small
    // gap below-right of the release point on Retina and non-Retina alike.
    let scale = win.scale_factor().unwrap_or(1.0);
    let x = (cx + 8.0 * scale).max(0.0);
    let y = (cy + 12.0 * scale).max(0.0);

    let _ = win.set_position(PhysicalPosition::new(x, y));
    let _ = win.set_always_on_top(true);
    // show() only (no set_focus): an Accessory app ordering a window front does
    // not steal first-responder from the host app.
    let _ = win.show();
    eprintln!(
        "[bubble] shown at physical ({x:.0}, {y:.0}) visible={:?}",
        win.is_visible()
    );
}

/// Install the global mouse-up monitor. Must run on the main thread (setup does).
#[cfg(target_os = "macos")]
pub fn register(app: &AppHandle) {
    use block2::RcBlock;
    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;
    use objc2_app_kit::{NSEvent, NSEventMask};
    use std::ptr::NonNull;

    // Global monitors only observe events bound for OTHER apps, so clicks on our
    // own bubble never re-trigger these.
    let down_app = app.clone();
    let down_handler = RcBlock::new(move |_event: NonNull<NSEvent>| {
        on_mouse_down(&down_app);
    });
    let down_token: Option<Retained<AnyObject>> = NSEvent::addGlobalMonitorForEventsMatchingMask_handler(
        NSEventMask::LeftMouseDown,
        &down_handler,
    );

    let up_app = app.clone();
    let up_handler = RcBlock::new(move |_event: NonNull<NSEvent>| {
        on_mouse_up(&up_app);
    });
    let up_token: Option<Retained<AnyObject>> =
        NSEvent::addGlobalMonitorForEventsMatchingMask_handler(NSEventMask::LeftMouseUp, &up_handler);

    // Keep the monitors (and their blocks) alive for the app's lifetime.
    match (down_token, up_token) {
        (Some(d), Some(u)) => {
            std::mem::forget(d);
            std::mem::forget(u);
            eprintln!("[bubble] global mouse monitors installed");
        }
        _ => eprintln!("[bubble] ERROR: failed to install mouse monitors"),
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
