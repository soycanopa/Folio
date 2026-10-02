import { useState } from "react";
import { ChevronRight, Clock, MoreHorizontal } from "lucide-react";
import type { ContentItem } from "../types";
import { FieldInput } from "./FieldInput";
import { RichText } from "./RichText";

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
  onFmChange,
  onBodyChange,
  onSave,
  onCommit,
  onPush,
  onBack,
}: EntryEditorProps) {
  const [sourceMode, setSourceMode] = useState(false);
  const [menuOpen, setMenuOpen] = useState(false);

  const fmFields = collection.fields.filter((f) => f.name !== "body");
  const bodyField = collection.fields.find((f) => f.name === "body");
  const title = String(draft.fm.title ?? "");

  return (
    <div className="flex h-full flex-col">
      <header className="relative flex items-center gap-2 px-6 py-4">
        <button
          onClick={onBack}
          className="flex items-center gap-1 text-sm text-ink-dim hover:text-ink"
        >
          {collection.label}
          <ChevronRight size={14} />
        </button>
        <span className="truncate text-sm">
          Editing &ldquo;{title || draft.path}&rdquo;
        </span>

        <div className="ml-auto flex items-center gap-1.5">
          <button
            disabled
            title="File history — coming soon"
            className="rounded-lg p-2 text-ink-dim opacity-50"
          >
            <Clock size={16} />
          </button>
          <button
            onClick={onSave}
            disabled={!dirty}
            className="rounded-lg bg-accent px-4 py-1.5 text-sm font-medium text-accent-ink disabled:opacity-40"
          >
            Save
          </button>
          <div className="relative">
            <button
              onClick={() => setMenuOpen((v) => !v)}
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
    </div>
  );
}
