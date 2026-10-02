import { ChevronDown, FileText, Image, NotebookPen } from "lucide-react";
import type { ContentItem, RepoStatus, RepoSummary } from "../types";

interface SidebarProps {
  summary: RepoSummary;
  status: RepoStatus | null;
  collections: ContentItem[];
  selected: string | null;
  onSelect: (c: ContentItem) => void;
  onOpenRepo: () => void;
}

export function Sidebar({
  summary,
  status,
  collections,
  selected,
  onSelect,
  onOpenRepo,
}: SidebarProps) {
  const repoName = summary.root.split("/").filter(Boolean).pop() ?? "repo";
  const dirty = status?.dirty ?? false;

  return (
    <aside className="flex h-full w-56 shrink-0 flex-col bg-panel border-r border-line">
      <button
        onClick={onOpenRepo}
        className="flex items-center gap-3 px-4 py-4 text-left hover:bg-raised/40 rounded-lg mx-2 mt-2"
        title="Open another folder"
      >
        <span className="flex h-8 w-8 shrink-0 items-center justify-center rounded-md bg-accent text-accent-ink">
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
        <ChevronDown size={14} className="text-ink-dim" />
      </button>

      <nav className="mt-4 flex-1 overflow-y-auto px-2">
        <p className="px-2 pb-1 text-[11px] font-medium uppercase tracking-wider text-ink-dim">
          Content
        </p>
        {collections.map((c) => (
          <button
            key={c.name}
            onClick={() => onSelect(c)}
            className={`flex w-full items-center gap-2.5 rounded-lg px-2 py-1.5 text-left ${
              selected === c.name ? "bg-raised" : "hover:bg-raised/40"
            }`}
          >
            <FileText size={15} className="shrink-0 text-ink-dim" />
            <span className="truncate">{c.label}</span>
          </button>
        ))}

        <p className="px-2 pb-1 pt-5 text-[11px] font-medium uppercase tracking-wider text-ink-dim">
          Media
        </p>
        <button
          disabled
          title="Media library arrives in v1"
          className="flex w-full cursor-not-allowed items-center gap-2.5 rounded-lg px-2 py-1.5 text-left opacity-50"
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
              dirty ? "bg-amber-400" : "bg-accent"
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
