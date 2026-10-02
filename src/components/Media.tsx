import { useState } from "react";
import { Copy, FileText, Trash2, Upload } from "lucide-react";
import type { MediaRef } from "../types";
import { MediaImg } from "./MediaImg";

interface MediaGridProps {
  root: string;
  items: MediaRef[];
  onPick?: (m: MediaRef) => void;
  onDelete?: (m: MediaRef) => void;
  /** Remoto: los src se resuelven contra la API, no contra el disco. */
  remote?: boolean;
}

export function MediaGrid({ root, items, onPick, onDelete, remote }: MediaGridProps) {
  const [copied, setCopied] = useState<string | null>(null);

  const copyPath = (m: MediaRef) => {
    void navigator.clipboard.writeText(m.public_path).then(() => {
      setCopied(m.path);
      window.setTimeout(() => setCopied(null), 1500);
    });
  };

  return (
    <div className="grid grid-cols-[repeat(auto-fill,minmax(140px,1fr))] gap-3">
      {items.map((m) => (
        <div
          key={m.path}
          role={onPick ? "button" : undefined}
          onClick={() => onPick?.(m)}
          title={m.public_path}
          className={`group relative flex flex-col gap-1.5 rounded-lg border border-line p-2 text-left ${
            onPick ? "cursor-pointer hover:bg-panel" : ""
          }`}
        >
          {m.is_image ? (
            <MediaImg
              root={root}
              path={m.path}
              remote={remote ?? false}
              alt={m.name}
              className="aspect-square w-full rounded-md bg-canvas object-cover"
            />
          ) : (
            <span className="flex aspect-square w-full items-center justify-center rounded-md bg-canvas text-ink-dim">
              <FileText size={28} />
            </span>
          )}
          <span className="truncate text-xs text-ink-dim">{m.name}</span>
          <div className="absolute right-2 top-2 hidden gap-1 group-hover:flex">
            <button
              onClick={(e) => {
                e.stopPropagation();
                copyPath(m);
              }}
              title={copied === m.path ? "Copied!" : `Copy ${m.public_path}`}
              className="rounded-md bg-canvas/90 p-1.5 text-ink-dim hover:text-ink"
            >
              <Copy size={12} />
            </button>
            {onDelete && (
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  onDelete(m);
                }}
                title="Delete"
                className="rounded-md bg-canvas/90 p-1.5 text-ink-dim hover:text-danger"
              >
                <Trash2 size={12} />
              </button>
            )}
          </div>
        </div>
      ))}
      {items.length === 0 && (
        <p className="col-span-full py-12 text-center text-sm text-ink-dim">
          No media yet. Use Upload to copy files into the media folder.
        </p>
      )}
    </div>
  );
}

interface MediaViewProps {
  root: string;
  items: MediaRef[];
  hasMediaInput: boolean;
  onUpload: () => void;
  onDelete: (m: MediaRef) => void;
  remote?: boolean;
}

export function MediaView({ root, items, hasMediaInput, onUpload, onDelete, remote }: MediaViewProps) {
  return (
    <div className="flex h-full flex-col">
      <header
        data-tauri-drag-region="deep"
        className="flex items-center gap-3 px-6 pt-5 pb-4">
        <h1 className="text-xl font-semibold">Media</h1>
        <button
          onClick={onUpload}
          disabled={!hasMediaInput}
          title={hasMediaInput ? undefined : "The config has no media.input"}
          className="ml-auto flex items-center gap-1.5 rounded-lg border border-line px-3 py-1.5 text-sm hover:bg-panel disabled:opacity-40"
        >
          <Upload size={14} /> Upload
        </button>
      </header>
      <div className="flex-1 overflow-y-auto px-6 pb-10">
        <MediaGrid root={root} items={items} onDelete={onDelete} remote={remote} />
      </div>
    </div>
  );
}

interface MediaPickerDialogProps {
  root: string;
  items: MediaRef[];
  onUpload: () => void;
  onPick: (m: MediaRef) => void;
  onClose: () => void;
  remote?: boolean;
}

// El mismo grid de Media, como diálogo desde un campo image (UI.md).
export function MediaPickerDialog({
  root,
  items,
  onUpload,
  onPick,
  onClose,
  remote,
}: MediaPickerDialogProps) {
  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4"
      onClick={onClose}
    >
      <div
        className="flex h-[70vh] w-full max-w-2xl flex-col rounded-xl border border-line bg-panel p-5 shadow-2xl"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="mb-4 flex items-center">
          <h2 className="text-base font-semibold">Choose an image</h2>
          <button
            onClick={onUpload}
            className="ml-auto flex items-center gap-1.5 rounded-lg border border-line px-3 py-1.5 text-sm hover:bg-raised"
          >
            <Upload size={14} /> Upload…
          </button>
        </div>
        <div className="flex-1 overflow-y-auto pr-1">
          <MediaGrid root={root} items={items} onPick={onPick} remote={remote} />
        </div>
      </div>
    </div>
  );
}
