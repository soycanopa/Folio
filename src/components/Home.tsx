import { FolderOpen, CopyPlus, FolderGit2, X } from "lucide-react";

interface HomeProps {
  recents: { path: string }[];
  onOpen: (path: string) => void;
  onRemove: (path: string) => void;
  onPickFolder: () => void;
  onClone: () => void;
}

// El home de proyectos de Pages CMS, local: los repos que ya abriste.
export function Home({
  recents,
  onOpen,
  onRemove,
  onPickFolder,
  onClone,
}: HomeProps) {
  return (
    <div className="flex h-full flex-col items-center overflow-y-auto px-6 py-12">
      <div className="w-full max-w-2xl">
        <header className="mb-8 flex items-center gap-3">
          <h1 className="text-2xl font-semibold">Projects</h1>
          <div className="ml-auto flex items-center gap-2">
            <button
              onClick={onClone}
              className="flex items-center gap-1.5 rounded-lg border border-line px-3 py-1.5 text-sm hover:bg-panel"
            >
              <CopyPlus size={14} /> Clone from GitHub…
            </button>
            <button
              onClick={onPickFolder}
              className="flex items-center gap-1.5 rounded-lg bg-primary px-3.5 py-1.5 text-sm font-medium text-primary-foreground"
            >
              <FolderOpen size={14} /> Open folder…
            </button>
          </div>
        </header>

        {recents.length === 0 ? (
          <div className="rounded-xl border border-dashed border-line p-10 text-center">
            <span className="mx-auto mb-3 flex h-10 w-10 items-center justify-center rounded-lg bg-panel text-ink-dim">
              <FolderGit2 size={20} />
            </span>
            <p className="text-sm text-ink-dim">
              No projects yet. Open a local folder that is a git repository
              with a .pages.yml, or clone one from GitHub.
            </p>
          </div>
        ) : (
          <div className="grid grid-cols-2 gap-3">
            {recents.map((r) => {
              const name = r.path.split("/").filter(Boolean).pop() ?? r.path;
              return (
                <div
                  key={r.path}
                  role="button"
                  tabIndex={0}
                  onClick={() => onOpen(r.path)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") onOpen(r.path);
                  }}
                  className="group relative cursor-pointer rounded-xl border border-line bg-panel p-4 hover:border-ring"
                >
                  <div className="flex items-center gap-2.5">
                    <span className="flex h-8 w-8 shrink-0 items-center justify-center rounded-md bg-raised text-ink-dim">
                      <FolderGit2 size={15} />
                    </span>
                    <span className="min-w-0">
                      <span className="block truncate text-sm font-medium">
                        {name}
                      </span>
                      <span
                        className="block truncate text-xs text-ink-dim"
                        title={r.path}
                      >
                        {r.path}
                      </span>
                    </span>
                  </div>
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      onRemove(r.path);
                    }}
                    title="Remove from list"
                    className="absolute right-2 top-2 hidden rounded-md p-1 text-ink-dim hover:text-danger group-hover:block"
                  >
                    <X size={13} />
                  </button>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}
