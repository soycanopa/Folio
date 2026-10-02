import { useState } from "react";
import { ChevronDown, CopyPlus, FileJson, FileText, FolderOpen, Image, LayoutGrid, NotebookPen } from "lucide-react";
import type { ContentItem, RepoStatus, RepoSummary } from "../types";

interface SidebarProps {
  summary: RepoSummary;
  status: RepoStatus | null;
  collections: ContentItem[];
  selected: string | null;
  onSelect: (c: ContentItem) => void;
  onOpenRepo: () => void;
  onClone: () => void;
  onMedia: () => void;
  /** Presente solo si el config tiene la colección del canvas. */
  onCanvas?: () => void;
}

export function Sidebar({
  summary,
  status,
  collections,
  selected,
  onSelect,
  onOpenRepo,
  onClone,
  onMedia,
  onCanvas,
}: SidebarProps) {
  const [menuOpen, setMenuOpen] = useState(false);
  const repoName = summary.root.split("/").filter(Boolean).pop() ?? "repo";
  const dirty = status?.dirty ?? false;

  return (
    <aside className="flex h-full w-56 shrink-0 flex-col bg-panel border-r border-line">
      <div className="relative mx-2 mt-2 flex items-center gap-3 rounded-lg px-3 py-3">
        <span className="flex h-8 w-8 shrink-0 items-center justify-center rounded-md bg-primary text-primary-foreground">
          <NotebookPen size={16} />
        </span>
        <span className="min-w-0 flex-1">
          <span className="block truncate font-medium">{repoName}</span>
          <span className="block truncate text-xs text-ink-dim">
            {status?.branch ?? summary.branch}
            {status && status.ahead > 0 ? ` ↑${status.ahead}` : ""}
            {status && status.behind > 0 ? ` ↓${status.behind}` : ""}
          </span>
        </span>
        <button
          onClick={() => setMenuOpen((v) => !v)}
          className="rounded-md p-1 text-ink-dim hover:text-ink"
          title="Open another folder or clone"
        >
          <ChevronDown size={14} />
        </button>
        {menuOpen && (
          <>
            <div className="fixed inset-0 z-10" onClick={() => setMenuOpen(false)} />
            <div className="absolute right-2 top-full z-20 mt-1 w-48 overflow-hidden rounded-lg border border-line bg-panel py-1 text-sm shadow-xl">
              <button
                onClick={() => {
                  setMenuOpen(false);
                  onOpenRepo();
                }}
                className="flex w-full items-center gap-2 px-3 py-1.5 text-left hover:bg-raised"
              >
                <FolderOpen size={14} /> Open folder…
              </button>
              <button
                onClick={() => {
                  setMenuOpen(false);
                  onClone();
                }}
                className="flex w-full items-center gap-2 px-3 py-1.5 text-left hover:bg-raised"
              >
                <CopyPlus size={14} /> Clone from GitHub…
              </button>
            </div>
          </>
        )}
      </div>

      <nav className="mt-4 flex-1 overflow-y-auto px-2">
        <p className="px-2 pb-1 text-[11px] font-medium uppercase tracking-wider text-ink-dim">
          Content
        </p>
        {collections.map((c, i) => {
          const prev = i > 0 ? collections[i - 1] : null;
          const showGroupLabel =
            c.group != null && c.group !== (prev?.group ?? null);
          return (
            <div key={c.name}>
              {showGroupLabel && (
                <p className="px-2 pb-1 pt-4 text-[11px] font-medium uppercase tracking-wider text-ink-dim">
                  {c.group}
                </p>
              )}
              <button
                onClick={() => onSelect(c)}
                className={`flex w-full items-center gap-2.5 rounded-lg px-2 py-1.5 text-left ${
                  selected === c.name ? "bg-raised" : "hover:bg-raised/40"
                }`}
              >
                {c.kind === "file" ? (
                  <FileJson size={15} className="shrink-0 text-ink-dim" />
                ) : (
                  <FileText size={15} className="shrink-0 text-ink-dim" />
                )}
                <span className="truncate">{c.label}</span>
              </button>
            </div>
          );
        })}

        {onCanvas && (
          <button
            onClick={onCanvas}
            className={`flex w-full items-center gap-2.5 rounded-lg px-2 py-1.5 text-left ${
              selected === "__canvas__" ? "bg-raised" : "hover:bg-raised/40"
            }`}
          >
            <LayoutGrid size={15} className="shrink-0 text-ink-dim" />
            <span className="truncate">Canvas</span>
          </button>
        )}

        <p className="px-2 pb-1 pt-5 text-[11px] font-medium uppercase tracking-wider text-ink-dim">
          Media
        </p>
        <button
          onClick={onMedia}
          className={`flex w-full items-center gap-2.5 rounded-lg px-2 py-1.5 text-left ${
            selected === "__media__" ? "bg-raised" : "hover:bg-raised/40"
          }`}
        >
          <Image size={15} className="shrink-0 text-ink-dim" />
          <span className="truncate">Media</span>
        </button>
      </nav>

      <div className="flex items-center gap-2.5 border-t border-line px-4 py-3">
        <span className="relative flex h-7 w-7 items-center justify-center rounded-full bg-raised text-xs font-medium">
          F
          <span
            className={`absolute -bottom-0.5 -right-0.5 h-2.5 w-2.5 rounded-full border-2 border-panel ${
              dirty ? "bg-amber-400" : "bg-primary"
            }`}
            title={dirty ? "Uncommitted changes" : "Clean"}
          />
        </span>
        <span className="text-xs text-ink-dim">
          {dirty ? "Uncommitted changes" : "Clean"}
        </span>
      </div>
    </aside>
  );
}
