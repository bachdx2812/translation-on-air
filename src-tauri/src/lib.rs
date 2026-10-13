mod accessibility;
mod capture;
mod commands;
mod hotkey;
mod keychain;
mod providers;
mod selection;
#[cfg(target_os = "macos")]
mod services;
mod settings;
mod tray;
mod windows;

use tauri::Manager;
use tauri_plugin_global_shortcut::ShortcutState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(
            // Single dispatcher: every hotkey press runs the capture pipeline off
            // the handler thread. The accelerator itself is registered in setup().
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        let app = app.clone();
                        tauri::async_runtime::spawn(async move {
                            crate::capture::run_capture_pipeline(&app).await;
                        });
                    }
                })
                .build(),
        )
        .manage(providers::ProviderCache::default())
        .invoke_handler(tauri::generate_handler![
            windows::show_popup,
            windows::hide_popup,
            windows::show_settings,
            windows::resize_popup,
            accessibility::check_accessibility,
            accessibility::request_accessibility,
            accessibility::open_accessibility_settings,
            commands::translate,
            commands::detect_providers,
            commands::set_openai_key,
            commands::delete_openai_key,
            commands::has_openai_key,
            commands::copy_text,
            commands::translate_reply,
            selection::bubble_translate,
            selection::hide_bubble,
            settings::get_settings,
            settings::set_settings,
            settings::set_hotkey,
        ])
        .on_window_event(|window, event| match event {
            // Hide the popup on close instead of destroying it, so its webview
            // stays warm and the hotkey shows it instantly (no cold start).
            tauri::WindowEvent::CloseRequested { api, .. } if window.label() == "popup" => {
                api.prevent_close();
                let _ = window.hide();
            }
            // Settings is rarely open, so closing destroys it to free its
            // webview (rebuilt on demand) → back to no-dock accessory mode.
            #[cfg(target_os = "macos")]
            tauri::WindowEvent::Destroyed if window.label() == "settings" => {
                let _ = window
                    .app_handle()
                    .set_activation_policy(tauri::ActivationPolicy::Accessory);
            }
            _ => {}
        })
        .setup(|app| {
            // Background agent: hide dock icon on macOS so the app lives only in the
            // menubar tray. Accessory policy works in both `tauri dev` and bundles.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            tray::create(app.handle())?;
            hotkey::register_from_settings(app.handle())?;
            // Right-click → Services → Translate (Info.plist NSServices entry).
            #[cfg(target_os = "macos")]
            services::register(app.handle());
            // Selection bubble (default on): its window and global mouse
            // monitors exist only while the setting is on.
            selection::set_enabled(app.handle(), settings::selection_bubble(app.handle()));
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
