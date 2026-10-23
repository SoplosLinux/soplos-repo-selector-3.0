mod commands;
mod system_theme;
mod mirrors_data;
pub(crate) mod privilege;
pub(crate) mod utils;

pub use privilege::run_privileged_script_file;
pub use utils::is_running_in_vm;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Must be the first plugin registered, same as the other Soplos
        // Tauri ports: it needs to intercept and exit a second launch
        // before anything else runs.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            use tauri::Manager;
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            system_theme::get_system_theme,
            commands::environment::get_environment_info,
            commands::repos::list_repos,
            commands::repos::save_repos,
            commands::repos::modernize_repo,
            commands::repos::generate_sources,
            commands::gpg::list_gpg_keys,
            commands::gpg::get_gpg_key_details,
            commands::gpg::import_gpg_key,
            commands::gpg::export_gpg_key,
            commands::gpg::delete_gpg_key,
            commands::gpg::pick_gpg_key_file,
            commands::gpg::pick_gpg_export_path,
            commands::speedtest::test_mirrors,
            commands::speedtest::get_region_mirrors,
            commands::speedtest::get_mirror_count,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Soplos Repo Selector");
}
