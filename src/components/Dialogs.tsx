import { useState } from "react";
import type { ContentItem } from "../types";

function Modal({
  title,
  children,
  onClose,
}: {
  title: string;
  children: React.ReactNode;
  onClose: () => void;
}) {
  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4"
      onClick={onClose}
    >
      <div
        className="w-full max-w-sm rounded-xl border border-line bg-panel p-5 shadow-2xl"
        onClick={(e) => e.stopPropagation()}
      >
        <h2 className="mb-4 text-base font-semibold">{title}</h2>
        {children}
      </div>
    </div>
  );
}

const inputCls =
  "w-full rounded-lg bg-canvas px-3 py-2 text-sm outline-none focus:ring-1 focus:ring-accent";
const btnPrimary =
  "rounded-lg bg-accent px-3.5 py-1.5 text-sm font-medium text-accent-ink disabled:opacity-40";
const btnGhost =
  "rounded-lg px-3.5 py-1.5 text-sm text-ink-dim hover:text-ink";

export function CommitDialog({
  defaultMessage,
  onClose,
  onConfirm,
}: {
  defaultMessage: string;
  onClose: () => void;
  onConfirm: (message: string) => void;
}) {
  const [message, setMessage] = useState(defaultMessage);
  return (
    <Modal title="Commit" onClose={onClose}>
      <form
        onSubmit={(e) => {
          e.preventDefault();
          if (message.trim()) onConfirm(message);
        }}
        className="flex flex-col gap-3"
      >
        <textarea
          autoFocus
          rows={3}
          value={message}
          onChange={(e) => setMessage(e.currentTarget.value)}
          placeholder="Commit message"
          className={inputCls}
        />
        <div className="flex justify-end gap-2">
          <button type="button" onClick={onClose} className={btnGhost}>
            Cancel
          </button>
          <button type="submit" disabled={!message.trim()} className={btnPrimary}>
            Commit
          </button>
        </div>
      </form>
    </Modal>
  );
}

export function NewEntryDialog({
  collection,
  onClose,
  onCreate,
}: {
  collection: ContentItem;
  onClose: () => void;
  onCreate: (title: string, slug: string) => void;
}) {
  const [title, setTitle] = useState("");
  const [slug, setSlug] = useState("");
  const slugified = slug || title
    .toLowerCase()
    .trim()
    .replace(/[^a-z0-9\s-]/g, "")
    .replace(/[\s_]+/g, "-")
    .replace(/-+/g, "-");
  return (
    <Modal title={`New entry — ${collection.label}`} onClose={onClose}>
      <form
        onSubmit={(e) => {
          e.preventDefault();
          if (title.trim() && slugified) onCreate(title.trim(), slugified);
        }}
        className="flex flex-col gap-3"
      >
        <label className="flex flex-col gap-1.5 text-sm">
          Title
          <input
            autoFocus
            value={title}
            onChange={(e) => setTitle(e.currentTarget.value)}
            className={inputCls}
          />
        </label>
        <label className="flex flex-col gap-1.5 text-sm">
          Slug
          <input
            value={slug}
            onChange={(e) => setSlug(e.currentTarget.value.toLowerCase())}
            placeholder={slugified || "my-post"}
            className={inputCls}
          />
          <span className="text-xs text-ink-dim">
            {collection.filename
              ? `File: ${collection.filename.replace("{slug}", slugified || "{slug}")}`
              : `File: ${slugified || "{slug}"}.md`}
          </span>
        </label>
        <div className="flex justify-end gap-2">
          <button type="button" onClick={onClose} className={btnGhost}>
            Cancel
          </button>
          <button
            type="submit"
            disabled={!title.trim() || !slugified}
            className={btnPrimary}
          >
            Create
          </button>
        </div>
      </form>
    </Modal>
  );
}
