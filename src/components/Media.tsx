import { convertFileSrc } from "@tauri-apps/api/core";
import { Upload } from "lucide-react";
import type { MediaRef } from "../types";

const IMAGE_FILTERS = [
  {
    name: "Images",
    extensions: ["png", "jpg", "jpeg", "gif", "webp", "avif", "svg"],
  },
];

interface MediaGridProps {
  root: string;
  items: MediaRef[];
  onPick?: (m: MediaRef) => void;
}

export function MediaGrid({ root, items, onPick }: MediaGridProps) {
  return (
    <div className="grid grid-cols-[repeat(auto-fill,minmax(140px,1fr))] gap-3">
      {items.map((m) => (
        <button
          key={m.path}
          onClick={() => onPick?.(m)}
          title={m.public_path}
          className="flex flex-col gap-1.5 rounded-lg border border-line p-2 text-left hover:bg-panel"
        >
          <img
            src={convertFileSrc(`${root}/${m.path}`)}
            alt={m.name}
            className="aspect-square w-full rounded-md bg-canvas object-cover"
          />
          <span className="truncate text-xs text-ink-dim">{m.name}</span>
        </button>
      ))}
      {items.length === 0 && (
        <p className="col-span-full py-12 text-center text-sm text-ink-dim">
          No media yet. Use Upload to copy images into the media folder.
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
}

export function MediaView({ root, items, hasMediaInput, onUpload }: MediaViewProps) {
  return (
    <div className="flex h-full flex-col">
      <header className="flex items-center gap-3 px-6 pt-5 pb-4">
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
        <MediaGrid root={root} items={items} />
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
}

// El mismo grid de Media, como diálogo desde un campo image (UI.md).
export function MediaPickerDialog({
  root,
  items,
  onUpload,
  onPick,
  onClose,
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
          <MediaGrid root={root} items={items} onPick={onPick} />
        </div>
      </div>
    </div>
  );
}

export { IMAGE_FILTERS };
