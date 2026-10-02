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

export interface CommitInfo {
  oid: string;
  message: string;
  author: string;
  /** Unix seconds. */
  time: number;
  /** true si no llegó al upstream. */
  local: boolean;
}
