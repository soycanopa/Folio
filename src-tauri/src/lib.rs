pub mod commit;
pub mod config;
pub mod entry;
pub mod file_entry;
pub mod gh_api;
pub mod github;
pub mod history;
pub mod media;
pub mod push;
pub mod recent;
pub mod repo;
pub mod state;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            repo::open_repo,
            repo::clone_repo,
            repo::repo_status,
            repo::list_entries,
            config::read_config,
            entry::read_entry,
            entry::write_entry,
            entry::create_entry,
            entry::rename_entry,
            entry::delete_entry,
            file_entry::read_file_entry,
            file_entry::write_file_entry,
            media::list_media,
            media::import_media,
            media::delete_media,
            github::github_login_start,
            github::github_login_poll,
            github::github_session,
            github::github_logout,
            github::github_list_repos,
            github::github_create_from_template,
            recent::list_recent_repos,
            recent::add_recent_repo,
            recent::remove_recent_repo,
            history::file_history,
            commit::commit,
            push::push
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
