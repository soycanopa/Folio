// Espejo de los tipos que serializa el core (src-tauri/src).
export interface Media {
  input?: string | null;
  output?: string | null;
  /** false/ausente → conserva nombre; "safe" | "random". */
  rename?: string | null;
}

export interface MediaRef {
  path: string;
  name: string;
  /** Ruta pública (media.output + nombre) para el front matter. */
  public_path: string;
  /** true si la extensión es de imagen (thumbnail en el grid). */
  is_image: boolean;
}

export interface Operations {
  create: boolean;
  rename: boolean;
  delete: boolean;
}

export interface Field {
  name: string;
  label: string;
  type: string;
  required: boolean;
  help?: string | null;
  /** Options de un select. */
  values?: string[];
}

export interface ViewCfg {
  fields: string[];
  sort?: string | null;
  order?: string | null;
}

export interface ContentItem {
  name: string;
  /** "collection" | "file". */
  kind: string;
  label: string;
  path: string;
  filename?: string | null;
  fields: Field[];
  view?: ViewCfg | null;
  operations: Operations;
  /** Label del `type: group` que lo contiene. */
  group?: string | null;
}

export interface PagesConfig {
  media: Media;
  content: ContentItem[];
  warnings: string[];
}

export interface RepoSummary {
  root: string;
  branch: string;
  has_config: boolean;
  config_error?: string | null;
}

export interface RepoStatus {
  branch: string;
  dirty: boolean;
  ahead: number;
  behind: number;
  has_upstream: boolean;
}

export interface EntryRef {
  path: string;
}

export interface EntryContent {
  frontmatter: Record<string, unknown>;
  body: string;
}

export interface NewEntry {
  path: string;
  existed: boolean;
}

export interface DeviceCodeStart {
  device_code: string;
  user_code: string;
  verification_uri: string;
  interval: number;
  expires_in: number;
}

export type DevicePoll =
  | { status: "pending" }
  | { status: "slow_down" }
  | { status: "authorized"; access_token: string }
  | { status: "denied" }
  | { status: "expired" };

export interface GithubUser {
  login: string;
}

export interface GhRepo {
  full_name: string;
  owner: string;
  private: boolean;
  /** Unix seconds de updatedAt. */
  updated_at: number;
  default_branch: string;
  clone_url: string;
}

export interface RecentRepo {
  path: string;
  /** Unix seconds de la última apertura; 0 si el registro es viejo. */
  last_open: number;
}

export interface CommitInfo {
  oid: string;
  message: string;
  author: string;
  /** Unix seconds. */
  time: number;
  /** true si no llegó al upstream. */
  local: boolean;
}
