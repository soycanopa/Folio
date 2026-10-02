import { invoke } from "@tauri-apps/api/core";
import type {
  EntryContent,
  EntryRef,
  MediaRef,
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
  repoStatus: () => invoke<RepoStatus>("repo_status"),
  readConfig: () => invoke<PagesConfig>("read_config"),
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
  listRecentRepos: () =>
    invoke<{ path: string }[]>("list_recent_repos"),
  addRecentRepo: (path: string) =>
    invoke<{ path: string }[]>("add_recent_repo", { path }),
  removeRecentRepo: (path: string) =>
    invoke<{ path: string }[]>("remove_recent_repo", { path }),
  fileHistory: (path: string) =>
    invoke<import("./types").CommitInfo[]>("file_history", { path }),
};
