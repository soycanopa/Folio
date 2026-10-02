import { useMemo, useState } from "react";
import { CopyPlus, FolderGit2, FolderOpen, Search, X } from "lucide-react";
import { relTime } from "../lib/time";

export interface RecentRepo {
  path: string;
  last_open: number;
}

interface HomeProps {
  recents: RecentRepo[];
  onOpen: (path: string) => void;
  onRemove: (path: string) => void;
  onPickFolder: () => void;
  onClone: () => void;
}

// El home de proyectos de Pages CMS (su app/(main)/page.tsx): columna
// angosta centrada, "Recently visited" con las últimas 3 y "Open a
// project" con búsqueda. Filas pegadas con borde, botón Open outline.
const rowCls =
  "flex gap-x-2 items-center border border-b-0 last:border-b px-3 py-2 text-sm";
const openBtnCls =
  "ml-auto inline-flex items-center rounded-md border border-line px-2.5 py-1 text-xs hover:bg-panel";

export function Home({
  recents,
  onOpen,
  onRemove,
  onPickFolder,
  onClone,
}: HomeProps) {
  const [keyword, setKeyword] = useState("");

  const latest = useMemo(
    () => recents.filter((r) => r.last_open > 0).slice(0, 3),
    [recents],
  );
  const filtered = useMemo(
    () =>
      recents.filter((r) =>
        r.path.toLowerCase().includes(keyword.trim().toLowerCase()),
      ),
    [recents, keyword],
  );

  const nameOf = (p: string) => p.split("/").filter(Boolean).pop() ?? p;

  const renderRow = (r: RecentRepo) => (
    <li key={r.path} className={rowCls}>
      <span className="flex h-6 w-6 shrink-0 items-center justify-center rounded bg-raised text-ink-dim">
        <FolderGit2 size={13} />
      </span>
      <button
        onClick={() => onOpen(r.path)}
        className="truncate font-medium hover:underline"
      >
        {nameOf(r.path)}
      </button>
      <span className="truncate text-muted-foreground" title={r.path}>
        {relTime(r.last_open)}
      </span>
      <button onClick={() => onOpen(r.path)} className={openBtnCls}>
        Open
      </button>
      <button
        onClick={() => onRemove(r.path)}
        title="Remove from list"
        className="rounded-md p-1 text-ink-dim hover:text-danger"
      >
        <X size={12} />
      </button>
    </li>
  );

  return (
    <div className="h-full overflow-y-auto">
      <div className="mx-auto flex min-h-full max-w-screen-sm flex-col justify-center space-y-8 p-6">
        {latest.length > 0 && (
          <div className="space-y-4">
            <h2 className="text-lg font-medium tracking-tight">
              Recently visited
            </h2>
            <ul>{latest.map(renderRow)}</ul>
          </div>
        )}

        <div className="space-y-4">
          <h2 className="text-lg font-medium tracking-tight">
            Open a project
          </h2>
          <div className="flex w-full items-center gap-x-2">
            <button
              onClick={onPickFolder}
              className="inline-flex items-center gap-2 rounded-md border border-line px-3 py-1.5 text-sm hover:bg-panel"
            >
              <FolderOpen size={14} /> Open folder…
            </button>
            <button
              onClick={onClone}
              className="inline-flex items-center gap-2 rounded-md border border-line px-3 py-1.5 text-sm hover:bg-panel"
            >
              <CopyPlus size={14} /> Clone from GitHub…
            </button>
            <div className="relative ml-auto flex-1">
              <Search
                size={16}
                className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 opacity-50"
              />
              <input
                value={keyword}
                onChange={(e) => setKeyword(e.currentTarget.value)}
                placeholder="Search repositories by name"
                className="h-9 w-full rounded-md border border-line bg-transparent pl-9 pr-3 text-sm outline-none placeholder:text-ink-dim focus-visible:ring-1 focus-visible:ring-ring"
              />
            </div>
          </div>

          {recents.length === 0 ? (
            <div className="flex h-[206px] flex-none flex-col items-center justify-center gap-2 rounded-md border border-line bg-accent p-4 text-center">
              <p className="text-sm font-medium">No projects</p>
              <p className="max-w-xs text-sm text-ink-dim">
                Open a local folder that is a git repository with a
                .pages.yml, or clone one from GitHub.
              </p>
            </div>
          ) : filtered.length > 0 ? (
            <ul>{filtered.map(renderRow)}</ul>
          ) : (
            <div className="flex h-[206px] flex-none flex-col items-center justify-center gap-2 rounded-md border border-line bg-accent p-4 text-center">
              <p className="text-sm font-medium">No projects</p>
              <p className="text-sm text-ink-dim">
                No projects matched your search.
              </p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
