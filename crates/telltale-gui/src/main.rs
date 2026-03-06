use tauri::Manager;
mod commands;

#[cfg(target_os = "windows")]
fn configure_windows_backdrop(app: &tauri::App) {
    if let Some(window) = app.get_webview_window("main") {
        if let Err(mica_error) = window_vibrancy::apply_mica(&window, None) {
            if let Err(acrylic_error) =
                window_vibrancy::apply_acrylic(&window, Some((20, 28, 43, 140)))
            {
                eprintln!(
                    "warning: failed to apply Mica ({mica_error}) and Acrylic fallback ({acrylic_error})"
                );
            } else {
                eprintln!("warning: Mica unavailable ({mica_error}); applied Acrylic fallback");
            }
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn configure_windows_backdrop(_app: &tauri::App) {}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            configure_windows_backdrop(app);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_status,
            commands::get_recent_alerts,
            commands::get_alert_counts,
            commands::get_rules,
            commands::run_scan
        ])
        .run(tauri::generate_context!())
        .expect("error while running telltale-gui");
}
