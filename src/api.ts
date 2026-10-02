import { invoke } from "@tauri-apps/api/core";
import type {
  EntryContent,
  EntryRef,
  NewEntry,
  PagesConfig,
  RepoStatus,
  RepoSummary,
} from "./types";

// El webview no toca el disco: todo pasa por comandos del core (TRD.md).
export const api = {
  openRepo: (path: string) =>
    invoke<RepoSummary>("open_repo", { path }),
  repoStatus: () => invoke<RepoStatus>("repo_status"),
  readConfig: () => invoke<PagesConfig>("read_config"),
  listEntries: (collection: string) =>
    invoke<EntryRef[]>("list_entries", { collection }),
  readEntry: (path: string) => invoke<EntryContent>("read_entry", { path }),
  writeEntry: (
    path: string,
    frontmatter: Record<string, unknown>,
    body: string,
  ) => invoke<string>("write_entry", { path, frontmatter, body }),
  createEntry: (collection: string, slug: string) =>
    invoke<NewEntry>("create_entry", { collection, slug }),
  commit: (message: string) => invoke<string>("commit", { message }),
  push: () => invoke<void>("push"),
};
