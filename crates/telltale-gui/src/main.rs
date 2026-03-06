mod commands;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::get_status,
            commands::get_recent_alerts,
            commands::get_alert_counts,
            commands::get_rules
        ])
        .run(tauri::generate_context!())
        .expect("error while running telltale-gui");
}
