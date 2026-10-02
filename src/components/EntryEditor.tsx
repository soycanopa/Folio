import { useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { ChevronRight, Clock, MoreHorizontal, Upload } from "lucide-react";
import type { CommitInfo, ContentItem, MediaRef } from "../types";
import { api } from "../api";
import { FieldInput } from "./FieldInput";
import { RichText } from "./RichText";
import { MediaPickerDialog } from "./Media";

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
}: EntryEditorProps) {
  const [sourceMode, setSourceMode] = useState(false);
  const [menuOpen, setMenuOpen] = useState(false);
  const [picking, setPicking] = useState<string | null>(null);
  const [historyOpen, setHistoryOpen] = useState(false);
  const [history, setHistory] = useState<CommitInfo[] | null>(null);

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
            className="rounded-lg bg-accent px-4 py-1.5 text-sm font-medium text-accent-ink disabled:opacity-40"
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
                onPickImage={setPicking}
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
              <RichText
                value={draft.body}
                onChange={onBodyChange}
                sourceMode={sourceMode}
              />
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
    </div>
  );
}

function relTime(unixSec: number): string {
  const s = Math.floor(Date.now() / 1000) - unixSec;
  if (s < 60) return "just now";
  if (s < 3600) return `${Math.floor(s / 60)}m ago`;
  if (s < 86400) return `${Math.floor(s / 3600)}h ago`;
  if (s < 86400 * 30) return `${Math.floor(s / 86400)}d ago`;
  return new Date(unixSec * 1000).toLocaleDateString();
}
