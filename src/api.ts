import { invoke } from "@tauri-apps/api/core";
import type {
  ConfigSave,
  EntryContent,
  EntryRef,
  MediaRef,
  MediaSrc,
  NewEntry,
  PagesConfig,
  RepoStatus,
  RepoSummary,
} from "./types";

// El webview no toca el disco: todo pasa por comandos del core (TRD.md).
export const api = {
  openRepo: (path: string) =>
    invoke<RepoSummary>("open_repo", { path }),
  cloneRepo: (url: string, dest: string) =>
    invoke<RepoSummary>("clone_repo", { url, dest }),
  openRemoteRepo: (owner: string, repo: string) =>
    invoke<RepoSummary>("open_remote_repo", { owner, repo }),
  remoteMediaUrl: (path: string) =>
    invoke<MediaSrc>("remote_media_url", { path }),
  repoStatus: () => invoke<RepoStatus>("repo_status"),
  readConfig: () => invoke<PagesConfig>("read_config"),
  // Editor Configuration: texto crudo del .pages.yml existente.
  readConfigRaw: () => invoke<string>("read_config_raw"),
  validateConfig: (raw: string) => invoke<void>("validate_config", { raw }),
  writeConfig: (raw: string) => invoke<ConfigSave>("write_config", { raw }),
  listEntries: (collection: string) =>
    invoke<EntryRef[]>("list_entries", { collection }),
  readEntry: (path: string) => invoke<EntryContent>("read_entry", { path }),
  readFileEntry: (path: string) =>
    invoke<EntryContent>("read_file_entry", { path }),
  writeEntry: (
    path: string,
    frontmatter: Record<string, unknown>,
    body: string,
  ) => invoke<string>("write_entry", { path, frontmatter, body }),
  createEntry: (collection: string, slug: string) =>
    invoke<NewEntry>("create_entry", { collection, slug }),
  renameEntry: (path: string, newName: string) =>
    invoke<string>("rename_entry", { path, newName }),
  deleteEntry: (path: string) => invoke<void>("delete_entry", { path }),
  writeFileEntry: (
    path: string,
    frontmatter: Record<string, unknown>,
    body: string,
  ) => invoke<string>("write_file_entry", { path, frontmatter, body }),
  commit: (message: string) => invoke<string>("commit", { message }),
  push: () => invoke<void>("push"),
  listMedia: () => invoke<MediaRef[]>("list_media"),
  importMedia: (src: string) => invoke<MediaRef>("import_media", { src }),
  deleteMedia: (path: string) => invoke<void>("delete_media", { path }),
  githubLoginStart: () =>
    invoke<import("./types").DeviceCodeStart>("github_login_start"),
  githubLoginPoll: (deviceCode: string) =>
    invoke<import("./types").DevicePoll>("github_login_poll", { deviceCode }),
  githubSession: () =>
    invoke<import("./types").GithubUser | null>("github_session"),
  githubLogout: () => invoke<void>("github_logout"),
  githubListRepos: (keyword: string) =>
    invoke<import("./types").GhRepo[]>("github_list_repos", { keyword }),
  listRecentRepos: () =>
    invoke<import("./types").RecentRepo[]>("list_recent_repos"),
  addRecentRepo: (path: string, ownerRepo?: string) =>
    invoke<import("./types").RecentRepo[]>("add_recent_repo", {
      path,
      ownerRepo: ownerRepo ?? null,
    }),
  githubCreateFromTemplate: (template: string, name: string) =>
    invoke<string>("github_create_from_template", { template, name }),
  removeRecentRepo: (path: string) =>
    invoke<import("./types").RecentRepo[]>("remove_recent_repo", { path }),
  fileHistory: (path: string) =>
    invoke<import("./types").CommitInfo[]>("file_history", { path }),
};
