import { lazy, Suspense, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import {
  ChevronRight,
  Clock,
  MoreHorizontal,
  Pencil,
  Trash2,
  Upload,
} from "lucide-react";
import type { CommitInfo, ContentItem, MediaRef } from "../types";
import { api } from "../api";
import { relTime } from "../lib/time";
import { FieldInput } from "./FieldInput";
import { MediaPickerDialog } from "./Media";

// TipTap es el grueso del bundle: se carga al entrar a un formulario.
const RichText = lazy(() =>
  import("./RichText").then((m) => ({ default: m.RichText })),
);

export interface Draft {
  path: string;
  isNew: boolean;
  fm: Record<string, unknown>;
  body: string;
  snapshot: string;
}

interface EntryEditorProps {
  collection: ContentItem;
  draft: Draft;
  dirty: boolean;
  /** false para `type: file`: misma barra, sin breadcrumb de lista (UI.md). */
  showBack: boolean;
  root: string;
  mediaInput?: string | null;
  mediaItems: MediaRef[];
  onUploadMedia: () => Promise<MediaRef | null>;
  onFmChange: (name: string, value: unknown) => void;
  onBodyChange: (body: string) => void;
  onSave: () => void;
  onCommit: () => void;
  onPush: () => void;
  onBack: () => void;
  onRename: (newName: string) => void;
  onDelete: () => void;
}

export function EntryEditor({
  collection,
  draft,
  dirty,
  showBack,
  root,
  mediaInput,
  mediaItems,
  onUploadMedia,
  onFmChange,
  onBodyChange,
  onSave,
  onCommit,
  onPush,
  onBack,
  onRename,
  onDelete,
}: EntryEditorProps) {
  const [sourceMode, setSourceMode] = useState(false);
  const [menuOpen, setMenuOpen] = useState(false);
  const [picking, setPicking] = useState<string | null>(null);
  const [historyOpen, setHistoryOpen] = useState(false);
  const [history, setHistory] = useState<CommitInfo[] | null>(null);
  const [renameOpen, setRenameOpen] = useState(false);
  const [newName, setNewName] = useState("");
  const [deleteOpen, setDeleteOpen] = useState(false);

  async function toggleHistory() {
    const next = !historyOpen;
    setHistoryOpen(next);
    setMenuOpen(false);
    if (next) {
      try {
        setHistory(await api.fileHistory(draft.path));
      } catch {
        setHistory([]);
      }
    }
  }

  const fmFields = collection.fields.filter((f) => f.name !== "body");
  const bodyField = collection.fields.find((f) => f.name === "body");
  const title = String(draft.fm.title ?? "");

  // La ruta pública del front matter apunta al basename en media.input.
  const previewSrcFor = (v: unknown): string | undefined => {
    if (typeof v !== "string" || !v || !mediaInput) return undefined;
    const name = v.split("/").pop();
    if (!name) return undefined;
    return convertFileSrc(`${root}/${mediaInput}/${name}`);
  };

  return (
    <div className="flex h-full flex-col">
      <header className="relative flex items-center gap-2 px-6 py-4">
        {showBack && (
          <button
            onClick={onBack}
            className="flex items-center gap-1 text-sm text-ink-dim hover:text-ink"
          >
            {collection.label}
            <ChevronRight size={14} />
          </button>
        )}
        <span className="truncate text-sm">
          Editing &ldquo;{title || draft.path}&rdquo;
        </span>

        <div className="ml-auto flex items-center gap-1.5">
          <div className="relative">
            <button
              onClick={() => void toggleHistory()}
              title="File history"
              className={`rounded-lg p-2 hover:bg-panel ${
                historyOpen ? "bg-panel" : "text-ink-dim"
              }`}
            >
              <Clock size={16} />
            </button>
            {historyOpen && (
              <>
                <div
                  className="fixed inset-0 z-10"
                  onClick={() => setHistoryOpen(false)}
                />
                <div className="absolute right-0 z-20 mt-1 max-h-96 w-80 overflow-y-auto rounded-lg border border-line bg-panel py-2 shadow-2xl">
                  {(history ?? []).map((c) => (
                    <div
                      key={c.oid}
                      className="flex items-start gap-2.5 px-3 py-2"
                    >
                      <span className="mt-0.5 flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-raised text-[11px] font-medium">
                        {c.author.charAt(0).toUpperCase()}
                      </span>
                      <span className="min-w-0 flex-1">
                        <span className="block truncate text-sm">
                          {c.message || c.oid.slice(0, 7)}
                        </span>
                        <span className="block text-xs text-ink-dim">
                          {c.author} · {relTime(c.time)}
                        </span>
                      </span>
                      {c.local && (
                        <span className="mt-1 shrink-0 rounded-full bg-raised px-2 py-0.5 text-[10px] text-ink-dim">
                          Local
                        </span>
                      )}
                    </div>
                  ))}
                  {history != null && history.length === 0 && (
                    <p className="px-3 py-3 text-sm text-ink-dim">
                      No commits for this file yet.
                    </p>
                  )}
                  {history == null && (
                    <p className="px-3 py-3 text-sm text-ink-dim">Loading…</p>
                  )}
                  <div className="mt-1 border-t border-line pt-1">
                    <button
                      onClick={() => {
                        setHistoryOpen(false);
                        onPush();
                      }}
                      className="flex w-full items-center gap-2 px-3 py-1.5 text-left text-sm hover:bg-raised"
                    >
                      <Upload size={14} /> Push…
                    </button>
                  </div>
                </div>
              </>
            )}
          </div>
          <button
            onClick={onSave}
            disabled={!dirty}
            className="rounded-lg bg-primary px-4 py-1.5 text-sm font-medium text-primary-foreground disabled:opacity-40"
          >
            Save
          </button>
          <div className="relative">
            <button
              onClick={() => {
                setMenuOpen((v) => !v);
                setHistoryOpen(false);
              }}
              className="rounded-lg p-2 hover:bg-panel"
              title="More"
            >
              <MoreHorizontal size={16} />
            </button>
            {menuOpen && (
              <>
                <div
                  className="fixed inset-0 z-10"
                  onClick={() => setMenuOpen(false)}
                />
                <div className="absolute right-0 z-20 mt-1 w-44 overflow-hidden rounded-lg border border-line bg-panel py-1 text-sm">
                  {collection.operations.rename && (
                    <button
                      onClick={() => {
                        setMenuOpen(false);
                        setNewName(basename(draft.path));
                        setRenameOpen(true);
                      }}
                      className="flex w-full items-center gap-2 px-3 py-1.5 text-left hover:bg-raised"
                    >
                      <Pencil size={13} /> Rename…
                    </button>
                  )}
                  {collection.operations.delete && (
                    <button
                      onClick={() => {
                        setMenuOpen(false);
                        setDeleteOpen(true);
                      }}
                      className="flex w-full items-center gap-2 px-3 py-1.5 text-left text-danger hover:bg-raised"
                    >
                      <Trash2 size={13} /> Delete…
                    </button>
                  )}
                  {(collection.operations.rename ||
                    collection.operations.delete) && (
                    <div className="my-1 border-t border-line" />
                  )}
                  <button
                    onClick={() => {
                      setMenuOpen(false);
                      onCommit();
                    }}
                    className="block w-full px-3 py-1.5 text-left hover:bg-raised"
                  >
                    Commit…
                  </button>
                  <button
                    onClick={() => {
                      setMenuOpen(false);
                      onPush();
                    }}
                    className="block w-full px-3 py-1.5 text-left hover:bg-raised"
                  >
                    Push
                  </button>
                </div>
              </>
            )}
          </div>
        </div>
      </header>

      <div className="flex-1 overflow-y-auto px-6 pb-10">
        <div className="mx-auto flex max-w-2xl flex-col gap-5">
          {fmFields.map((field) => (
            <div key={field.name} className="flex flex-col gap-1.5">
              <div className="flex items-center gap-2">
                <label className="text-sm font-medium">{field.label}</label>
                {field.required && (
                  <span className="rounded-full bg-raised px-2 py-0.5 text-[10px] text-ink-dim">
                    Required
                  </span>
                )}
              </div>
              <FieldInput
                field={field}
                value={draft.fm[field.name]}
                onChange={(v) => onFmChange(field.name, v)}
                previewSrc={previewSrcFor(draft.fm[field.name])}
                onPickMedia={setPicking}
              />
              {field.help && (
                <p className="text-xs text-ink-dim">{field.help}</p>
              )}
            </div>
          ))}

          {bodyField && (
            <div className="flex flex-col gap-1.5">
              <div className="flex items-center gap-2">
                <label className="text-sm font-medium">
                  {bodyField.label || "Body"}
                </label>
                {bodyField.required && (
                  <span className="rounded-full bg-raised px-2 py-0.5 text-[10px] text-ink-dim">
                    Required
                  </span>
                )}
                <div className="ml-auto flex overflow-hidden rounded-full bg-raised text-xs">
                  <button
                    onClick={() => setSourceMode(false)}
                    className={`px-3 py-1 ${!sourceMode ? "bg-panel text-ink" : "text-ink-dim"}`}
                  >
                    Editor
                  </button>
                  <button
                    onClick={() => setSourceMode(true)}
                    className={`px-3 py-1 ${sourceMode ? "bg-panel text-ink" : "text-ink-dim"}`}
                  >
                    Source
                  </button>
                </div>
              </div>
              <Suspense
                fallback={
                  <div className="min-h-[240px] rounded-lg bg-panel" />
                }
              >
                <RichText
                  key={draft.path}
                  value={draft.body}
                  onChange={onBodyChange}
                  sourceMode={sourceMode}
                  media={
                    mediaInput
                      ? {
                          root,
                          mediaInput,
                          items: mediaItems,
                          onUpload: onUploadMedia,
                        }
                      : undefined
                  }
                />
              </Suspense>
              {bodyField.help && (
                <p className="text-xs text-ink-dim">{bodyField.help}</p>
              )}
            </div>
          )}
        </div>
      </div>

      {picking && (
        <MediaPickerDialog
          root={root}
          items={mediaItems}
          onUpload={() => void onUploadMedia()}
          onPick={(m) => {
            onFmChange(picking, m.public_path);
            setPicking(null);
          }}
          onClose={() => setPicking(null)}
        />
      )}

      {renameOpen && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4"
          onClick={() => setRenameOpen(false)}
        >
          <form
            onSubmit={(e) => {
              e.preventDefault();
              if (newName.trim() && newName !== basename(draft.path)) {
                setRenameOpen(false);
                onRename(newName.trim());
              }
            }}
            className="w-full max-w-sm rounded-xl border border-line bg-panel p-5 shadow-2xl"
            onClick={(e) => e.stopPropagation()}
          >
            <h2 className="mb-4 text-base font-semibold">Rename entry</h2>
            <input
              autoFocus
              value={newName}
              onChange={(e) => setNewName(e.currentTarget.value)}
              className="w-full rounded-lg bg-canvas px-3 py-2 text-sm outline-none focus:ring-1 focus:ring-ring"
            />
            <div className="mt-4 flex justify-end gap-2">
              <button
                type="button"
                onClick={() => setRenameOpen(false)}
                className="rounded-lg px-3.5 py-1.5 text-sm text-ink-dim hover:text-ink"
              >
                Cancel
              </button>
              <button
                type="submit"
                disabled={!newName.trim()}
                className="rounded-lg bg-primary px-3.5 py-1.5 text-sm font-medium text-primary-foreground disabled:opacity-40"
              >
                Rename
              </button>
            </div>
          </form>
        </div>
      )}

      {deleteOpen && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4"
          onClick={() => setDeleteOpen(false)}
        >
          <div
            className="w-full max-w-sm rounded-xl border border-line bg-panel p-5 shadow-2xl"
            onClick={(e) => e.stopPropagation()}
          >
            <h2 className="mb-2 text-base font-semibold">Delete entry</h2>
            <p className="mb-4 text-sm text-ink-dim">
              Delete <span className="text-ink">{basename(draft.path)}</span>?
              It will be removed on the next commit.
            </p>
            <div className="flex justify-end gap-2">
              <button
                onClick={() => setDeleteOpen(false)}
                className="rounded-lg px-3.5 py-1.5 text-sm text-ink-dim hover:text-ink"
              >
                Cancel
              </button>
              <button
                onClick={() => {
                  setDeleteOpen(false);
                  onDelete();
                }}
                className="rounded-lg bg-danger px-3.5 py-1.5 text-sm font-medium text-white"
              >
                Delete
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

function basename(path: string): string {
  return path.split("/").pop() ?? path;
}

