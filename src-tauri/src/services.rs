//! macOS Services provider: right-click selected text → Services → "Translate
//! with Translate On Air" (declared in src-tauri/Info.plist under NSServices).
//!
//! The OS hands us the selection through NSPasteboard, so this path needs NO
//! Accessibility permission — unlike the hotkey pipeline's synthetic Cmd+C.

use objc2::rc::Retained;
use objc2::runtime::NSObject;
use objc2::{define_class, msg_send, AllocAnyThread, DefinedClass, MainThreadMarker};
use objc2_app_kit::{NSApplication, NSPasteboard, NSPasteboardTypeString};
use objc2_foundation::NSString;
use tauri::AppHandle;

pub struct Ivars {
    app: AppHandle,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "TOAServicesProvider"]
    #[ivars = Ivars]
    pub struct ServicesProvider;

    impl ServicesProvider {
        /// Selector named by NSMessage ("translateSelection") in Info.plist:
        /// `translateSelection:userData:error:`. AppKit invokes it on the main
        /// thread with the pasteboard holding the right-clicked selection.
        #[unsafe(method(translateSelection:userData:error:))]
        fn translate_selection(
            &self,
            pboard: &NSPasteboard,
            _user_data: Option<&NSString>,
            _error: *mut *mut NSString,
        ) {
            let text = unsafe { pboard.stringForType(NSPasteboardTypeString) }
                .map(|s| s.to_string())
                .unwrap_or_default();
            if text.trim().is_empty() {
                return;
            }
            let app = self.ivars().app.clone();
            tauri::async_runtime::spawn(async move {
                crate::capture::run_text_pipeline(&app, text).await;
            });
        }
    }
);

impl ServicesProvider {
    fn new(app: AppHandle) -> Retained<Self> {
        let this = Self::alloc().set_ivars(Ivars { app });
        unsafe { msg_send![super(this), init] }
    }
}

/// Register the provider with NSApp. Must run on the main thread (setup() does).
pub fn register(app: &AppHandle) {
    let mtm = MainThreadMarker::new().expect("services::register must run on the main thread");
    let ns_app = NSApplication::sharedApplication(mtm);
    let provider = ServicesProvider::new(app.clone());
    let _: () = unsafe { msg_send![&*ns_app, setServicesProvider: &*provider] };
    // NSApp does not retain its services provider; keep it alive for the app's lifetime.
    std::mem::forget(provider);
}
