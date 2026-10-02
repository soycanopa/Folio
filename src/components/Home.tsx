import { useMemo, useState } from "react";
import {
  ArrowUpRight,
  Check,
  ChevronsUpDown,
  CopyPlus,
  FolderGit2,
  FolderOpen,
  LockKeyhole,
  Search,
  Settings,
  X,
} from "lucide-react";
import { relTime } from "../lib/time";
import templates from "../lib/templates";
import type { GhRepo, GithubUser, RecentRepo } from "../types";

interface HomeProps {
  recents: RecentRepo[];
  session: GithubUser | null;
  repos: GhRepo[];
  onOpen: (path: string) => void;
  onRemove: (path: string) => void;
  /** Solo se muestra sin sesión: abrir una carpeta local (v0). */
  onPickFolder?: () => void;
  onClone?: () => void;
  onCloneRepo: (repo: GhRepo) => void;
  onSignIn: () => void;
  onSignOut: () => void;
  onCreateTemplate: (template: string, name: string) => void;
}

// El home de Pages CMS (su app/(main)/page.tsx), fila por fila:
// "Recently visited" (su repo-latest), "Open a project" (su
// repo-select) y "Create from a template" (su repo-templates).
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
  onCreateTemplate,
}: HomeProps) {
  const [keyword, setKeyword] = useState("");
  const [accountOpen, setAccountOpen] = useState(false);
  const [template, setTemplate] = useState<string | null>(null);
  const [templateName, setTemplateName] = useState("");

  const latest = useMemo(
    () => recents.filter((r) => r.last_open > 0).slice(0, 3),
    [recents],
  );
  const filtered = useMemo(() => {
    const q = keyword.trim().toLowerCase();
    return repos.filter((r) => r.full_name.toLowerCase().includes(q));
  }, [repos, keyword]);

  const nameOf = (p: string) => p.split("/").filter(Boolean).pop() ?? p;

  // Su repo-latest: avatar del owner cuando el proyecto vino de GitHub.
  const renderRecent = (r: RecentRepo) => (
    <li key={r.path} className={rowCls}>
      {r.owner_repo ? (
        <img
          src={`https://github.com/${r.owner_repo.split("/")[0]}.png`}
          alt=""
          className="h-6 w-6 shrink-0 rounded"
        />
      ) : (
        <span className="flex h-6 w-6 shrink-0 items-center justify-center rounded bg-raised text-ink-dim">
          <FolderGit2 size={13} />
        </span>
      )}
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

  // Su repo-select: nombre corto del repo, candado si privado, updated
  // relativo, Open. Sin avatar en la fila — igual que ellos.
  const renderRepo = (repo: GhRepo) => (
    <li key={repo.full_name} className={rowCls}>
      <button
        onClick={() => onCloneRepo(repo)}
        className="truncate font-medium hover:underline"
      >
        {repo.full_name.split("/")[1]}
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
                <div className="flex items-stretch rounded-md border border-line">
                  <div className="relative">
                    <button
                      onClick={() => setAccountOpen((v) => !v)}
                      className="inline-flex items-center gap-2 px-3 py-1.5 text-sm hover:bg-panel"
                    >
                      <img
                        src={`https://github.com/${session.login}.png`}
                        alt={session.login}
                        className="size-6 rounded"
                      />
                      <span className="mr-2">{session.login}</span>
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
                            className="block w-full px-3 py-1.5 text-left hover:bg-raised"
                          >
                            Sign out
                          </button>
                        </div>
                      </>
                    )}
                  </div>
                  <a
                    href="https://github.com/settings/applications"
                    target="_blank"
                    rel="noreferrer"
                    title="Manage authorized OAuth Apps"
                    className="flex items-center border-l border-line px-3 text-ink-dim hover:bg-panel hover:text-ink"
                  >
                    <Settings size={16} />
                  </a>
                </div>
                <div className="relative flex-1">
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
              {(onPickFolder || onClone) && (
                <div className="flex w-full items-center gap-x-2">
                  {onPickFolder && (
                    <button
                      onClick={onPickFolder}
                      className="inline-flex items-center gap-2 rounded-md border border-line px-3 py-1.5 text-sm hover:bg-panel"
                    >
                      <FolderOpen size={14} /> Open folder…
                    </button>
                  )}
                  {onClone && (
                    <button
                      onClick={onClone}
                      className="inline-flex items-center gap-2 rounded-md border border-line px-3 py-1.5 text-sm hover:bg-panel"
                    >
                      <CopyPlus size={14} /> Clone from GitHub…
                    </button>
                  )}
                </div>
              )}
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

        {session && (
          <div className="space-y-4">
            <h2 className="text-lg font-medium tracking-tight">
              Create from a template
            </h2>
            <div className="grid grid-cols-2 gap-6 sm:grid-cols-3">
              {templates
                .filter((t) => t.featured)
                .map((t) => (
                  <button
                    key={t.repository}
                    onClick={() => {
                      setTemplate(t.repository);
                      setTemplateName(t.suggested);
                    }}
                    className="overflow-hidden rounded-md border border-line text-left transition-colors hover:bg-accent"
                  >
                    <img
                      src={t.thumbnail}
                      alt={`Preview for ${t.name}`}
                      className="aspect-video w-full object-cover"
                    />
                    <div className="flex items-center gap-x-2 border-t border-line px-3 py-2 text-sm">
                      <span
                        dangerouslySetInnerHTML={{ __html: t.icon }}
                        className="h-4 w-4 shrink-0 [&>svg]:h-4 [&>svg]:w-4"
                      />
                      <span className="truncate font-medium">{t.name}</span>
                    </div>
                  </button>
                ))}
            </div>
          </div>
        )}
      </div>

      {template && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4"
          onClick={() => setTemplate(null)}
        >
          <form
            onSubmit={(e) => {
              e.preventDefault();
              if (templateName.trim()) {
                onCreateTemplate(template, templateName.trim());
                setTemplate(null);
              }
            }}
            className="w-full max-w-[425px] rounded-xl border border-line bg-panel p-6 shadow-2xl"
            onClick={(e) => e.stopPropagation()}
          >
            <h2 className="text-base font-semibold">Copy template</h2>
            <p className="mt-1 mb-4 text-sm text-ink-dim">
              This will create a copy of the template repository below under
              your account.
            </p>
            {session && (
              <a
                href={`https://github.com/${template}`}
                target="_blank"
                rel="noreferrer"
                className="relative flex items-center overflow-hidden rounded-lg border border-line transition-colors hover:bg-accent"
              >
                <img
                  src={templates.find((t) => t.repository === template)?.thumbnail}
                  alt=""
                  className="h-20 aspect-video object-cover"
                />
                <div className="flex flex-1 flex-col justify-center gap-y-1 truncate border-l border-line px-3 py-2 text-left">
                  <div className="truncate font-medium tracking-tight">
                    {templates.find((t) => t.repository === template)?.name}
                  </div>
                  <div className="truncate text-xs text-ink-dim">
                    {template}
                  </div>
                </div>
                <ArrowUpRight size={12} className="absolute right-2 top-2 opacity-50" />
              </a>
            )}
            <div className="mt-4 grid grid-cols-4 items-center gap-4 text-sm">
              <span className="text-right text-ink-dim">Account</span>
              <div className="col-span-3 flex items-center gap-2 rounded-md border border-line px-3 py-1.5">
                {session && (
                  <img
                    src={`https://github.com/${session.login}.png`}
                    alt={session.login}
                    className="h-6 w-6 rounded"
                  />
                )}
                <span className="truncate">{session?.login}</span>
              </div>
            </div>
            <div className="mt-4 grid grid-cols-4 items-center gap-4 text-sm">
              <label className="text-right text-ink-dim">Name</label>
              <input
                value={templateName}
                onChange={(e) => setTemplateName(e.currentTarget.value)}
                required
                className="col-span-3 rounded-md border border-line bg-transparent px-3 py-1.5 outline-none focus-visible:ring-1 focus-visible:ring-ring"
              />
            </div>
            <div className="mt-6 flex justify-end gap-2">
              <button
                type="button"
                onClick={() => setTemplate(null)}
                className="rounded-md px-3.5 py-1.5 text-sm text-ink-dim hover:text-ink"
              >
                Cancel
              </button>
              <button
                type="submit"
                disabled={!templateName.trim()}
                className="rounded-md bg-primary px-3.5 py-1.5 text-sm font-medium text-primary-foreground disabled:opacity-40"
              >
                Create copy
              </button>
            </div>
          </form>
        </div>
      )}
    </div>
  );
}
