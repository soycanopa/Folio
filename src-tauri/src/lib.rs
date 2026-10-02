pub mod commit;
pub mod config;
pub mod entry;
pub mod repo;
pub mod state;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            repo::open_repo,
            repo::repo_status,
            repo::list_entries,
            config::read_config,
            entry::read_entry,
            entry::write_entry,
            entry::create_entry,
            commit::commit
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
