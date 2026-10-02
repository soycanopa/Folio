import { useMemo, useState } from "react";
import { Check, ChevronsUpDown, CopyPlus, FolderGit2, FolderOpen, LockKeyhole, LogOut, Search, X } from "lucide-react";
import { relTime } from "../lib/time";
import type { GhRepo, GithubUser, RecentRepo } from "../types";

interface HomeProps {
  recents: RecentRepo[];
  session: GithubUser | null;
  repos: GhRepo[];
  onOpen: (path: string) => void;
  onRemove: (path: string) => void;
  onPickFolder: () => void;
  onClone: () => void;
  onCloneRepo: (repo: GhRepo) => void;
  onSignIn: () => void;
  onSignOut: () => void;
}

// El home de Pages CMS (su app/(main)/page.tsx): columna angosta
// centrada, "Recently visited" y "Open a project". Sin sesión, el
// bloque de proyectos muestra el empty de su page con el botón de
// sign-in; con sesión, las filas de su RepoSelect (avatar, nombre,
// candado, updatedAt, Open).
const rowCls =
  "flex gap-x-2 items-center border border-b-0 last:border-b px-3 py-2 text-sm";
const openBtnCls =
  "ml-auto inline-flex items-center rounded-md border border-line px-2.5 py-1 text-xs hover:bg-panel";

export function Home({
  recents,
  session,
  repos,
  onOpen,
  onRemove,
  onPickFolder,
  onClone,
  onCloneRepo,
  onSignIn,
  onSignOut,
}: HomeProps) {
  const [keyword, setKeyword] = useState("");
  const [accountOpen, setAccountOpen] = useState(false);

  const latest = useMemo(
    () => recents.filter((r) => r.last_open > 0).slice(0, 3),
    [recents],
  );
  const filtered = useMemo(() => {
    const q = keyword.trim().toLowerCase();
    return repos.filter(
      (r) =>
        r.full_name.toLowerCase().includes(q) ||
        r.owner.toLowerCase().includes(q),
    );
  }, [repos, keyword]);

  const nameOf = (p: string) => p.split("/").filter(Boolean).pop() ?? p;

  const renderRecent = (r: RecentRepo) => (
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

  const renderRepo = (repo: GhRepo) => (
    <li key={repo.full_name} className={rowCls}>
      <img
        src={`https://github.com/${repo.owner}.png`}
        alt={repo.owner}
        className="h-6 w-6 shrink-0 rounded"
      />
      <button
        onClick={() => onCloneRepo(repo)}
        className="truncate font-medium hover:underline"
      >
        {repo.full_name}
      </button>
      {repo.private && <LockKeyhole size={12} className="shrink-0 opacity-50" />}
      <span className="truncate text-muted-foreground">
        {relTime(repo.updated_at)}
      </span>
      <button onClick={() => onCloneRepo(repo)} className={openBtnCls}>
        Open
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
            <ul>{latest.map(renderRecent)}</ul>
          </div>
        )}

        <div className="space-y-4">
          <h2 className="text-lg font-medium tracking-tight">
            Open a project
          </h2>

          {session ? (
            <>
              <div className="flex w-full items-center gap-x-2">
                <div className="relative">
                  <button
                    onClick={() => setAccountOpen((v) => !v)}
                    className="inline-flex items-center gap-2 rounded-md border border-line px-3 py-1.5 text-sm hover:bg-panel"
                  >
                    <img
                      src={`https://github.com/${session.login}.png`}
                      alt={session.login}
                      className="size-6 rounded"
                    />
                    <span className="mr-1">{session.login}</span>
                    <ChevronsUpDown size={14} className="ml-auto opacity-50" />
                  </button>
                  {accountOpen && (
                    <>
                      <div
                        className="fixed inset-0 z-10"
                        onClick={() => setAccountOpen(false)}
                      />
                      <div className="absolute left-0 top-full z-20 mt-1 w-48 overflow-hidden rounded-lg border border-line bg-panel py-1 text-sm shadow-xl">
                        <div className="flex items-center gap-2 px-3 py-1.5">
                          <Check size={14} className="shrink-0" />
                          <span className="truncate">{session.login}</span>
                        </div>
                        <div className="my-1 border-t border-line" />
                        <button
                          onClick={() => {
                            setAccountOpen(false);
                            onSignOut();
                          }}
                          className="flex w-full items-center gap-2 px-3 py-1.5 text-left hover:bg-raised"
                        >
                          <LogOut size={14} /> Sign out
                        </button>
                      </div>
                    </>
                  )}
                </div>
                <button
                  onClick={onPickFolder}
                  title="Open a local folder"
                  className="inline-flex items-center gap-2 rounded-md border border-line px-3 py-1.5 text-sm hover:bg-panel"
                >
                  <FolderOpen size={14} /> Local folder…
                </button>
                <button
                  onClick={onClone}
                  title="Clone by URL"
                  className="inline-flex items-center gap-2 rounded-md border border-line px-3 py-1.5 text-sm hover:bg-panel"
                >
                  <CopyPlus size={14} /> Clone…
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

              {filtered.length > 0 ? (
                <ul>{filtered.map(renderRepo)}</ul>
              ) : (
                <div className="flex h-[206px] flex-none flex-col items-center justify-center gap-2 rounded-md border border-line bg-accent p-4 text-center">
                  <p className="text-sm font-medium">No projects</p>
                  <p className="text-sm text-ink-dim">
                    No projects matched your search.
                  </p>
                </div>
              )}
            </>
          ) : (
            <>
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
              </div>
              <div className="flex h-[206px] flex-none flex-col items-center justify-center gap-3 rounded-md border border-line bg-accent p-4 text-center">
                <p className="text-sm font-medium">Sign in to GitHub</p>
                <p className="max-w-xs text-sm text-ink-dim">
                  Sign in to open your GitHub repositories, like on the Pages
                  CMS platform.
                </p>
                <button
                  onClick={onSignIn}
                  className="inline-flex items-center gap-2 rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground"
                >
                  <svg
                    role="img"
                    viewBox="0 0 24 24"
                    xmlns="http://www.w3.org/2000/svg"
                    fill="currentColor"
                    className="size-4"
                  >
                    <title>GitHub</title>
                    <path d="M12 .297c-6.63 0-12 5.373-12 12 0 5.303 3.438 9.8 8.205 11.385.6.113.82-.258.82-.577 0-.285-.01-1.04-.015-2.04-3.338.724-4.042-1.61-4.042-1.61C4.422 18.07 3.633 17.7 3.633 17.7c-1.087-.744.084-.729.084-.729 1.205.084 1.838 1.236 1.838 1.236 1.07 1.835 2.809 1.305 3.495.998.108-.776.417-1.305.76-1.605-2.665-.3-5.466-1.332-5.466-5.93 0-1.31.465-2.38 1.235-3.22-.135-.303-.54-1.523.105-3.176 0 0 1.005-.322 3.3 1.23.96-.267 1.98-.399 3-.405 1.02.006 2.04.138 3 .405 2.28-1.552 3.285-1.23 3.285-1.23.645 1.653.24 2.873.12 3.176.765.84 1.23 1.91 1.23 3.22 0 4.61-2.805 5.625-5.475 5.92.42.36.81 1.096.81 2.22 0 1.606-.015 2.896-.015 3.286 0 .315.21.69.825.57C20.565 22.092 24 17.592 24 12.297c0-6.627-5.373-12-12-12" />
                  </svg>
                  Sign in with GitHub
                </button>
              </div>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
