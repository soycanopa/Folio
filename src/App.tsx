import { useCallback, useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { confirm, open as openFolderDialog } from "@tauri-apps/plugin-dialog";
import { FolderOpen, NotebookPen } from "lucide-react";
import { api } from "./api";
import type { ContentItem, PagesConfig, RepoStatus, RepoSummary } from "./types";
import { Sidebar } from "./components/Sidebar";
import { CollectionTable, type EntryRow } from "./components/CollectionTable";
import { EntryEditor, type Draft } from "./components/EntryEditor";
import { CloneDialog, CommitDialog, NewEntryDialog } from "./components/Dialogs";

type View =
  | { kind: "empty" }
  | { kind: "collection"; collection: ContentItem }
  | { kind: "entry"; collection: ContentItem; draft: Draft };

const snap = (d: { fm: Record<string, unknown>; body: string }) =>
  JSON.stringify({ fm: d.fm, body: d.body });

export default function App() {
  const [summary, setSummary] = useState<RepoSummary | null>(null);
  const [config, setConfig] = useState<PagesConfig | null>(null);
  const [status, setStatus] = useState<RepoStatus | null>(null);
  const [view, setView] = useState<View>({ kind: "empty" });
  const [rows, setRows] = useState<EntryRow[]>([]);
  const [message, setMessage] = useState("");
  const [commitOpen, setCommitOpen] = useState(false);
  const [newOpen, setNewOpen] = useState(false);
  const [cloneOpen, setCloneOpen] = useState(false);

  const viewRef = useRef(view);
  viewRef.current = view;

  const refreshStatus = useCallback(() => {
    api.repoStatus().then(setStatus).catch((e) => setMessage(String(e)));
  }, []);

  const loadRows = useCallback(async (c: ContentItem) => {
    try {
      const list = await api.listEntries(c.path);
      const contents = await Promise.all(
        list.map(async (e) => {
          const content = await api.readEntry(e.path);
          return { path: e.path, values: content.frontmatter };
        }),
      );
      setRows(contents);
    } catch (e) {
      setMessage(String(e));
    }
  }, []);

  const selectCollection = useCallback(
    (c: ContentItem) => {
      setView({ kind: "collection", collection: c });
      loadRows(c);
    },
    [loadRows],
  );

  // Un `type: file` no tiene tabla: el ítem del sidebar abre su
  // formulario directo (UI.md).
  const openFileEntry = useCallback(async (item: ContentItem) => {
    try {
      const content = await api.readFileEntry(item.path);
      setView({
        kind: "entry",
        collection: item,
        draft: {
          path: item.path,
          isNew: false,
          fm: content.frontmatter,
          body: content.body,
          snapshot: JSON.stringify({
            fm: content.frontmatter,
            body: content.body,
          }),
        },
      });
    } catch (e) {
      setMessage(String(e));
    }
  }, []);

  const selectItem = useCallback(
    (c: ContentItem) => {
      if (c.kind === "file") void openFileEntry(c);
      else selectCollection(c);
    },
    [openFileEntry, selectCollection],
  );

  const openRepoFlow = useCallback(
    async (path: string) => {
      try {
        const s = await api.openRepo(path);
        setSummary(s);
        localStorage.setItem("folio:lastRepo", path);
        setStatus(null);
        setView({ kind: "empty" });
        setRows([]);
        if (!s.has_config) {
          setConfig(null);
          return;
        }
        const cfg = await api.readConfig();
        setConfig(cfg);
        refreshStatus();
        if (cfg.content.length > 0) selectCollection(cfg.content[0]);
      } catch (e) {
        setMessage(String(e));
      }
    },
    [refreshStatus, selectCollection],
  );

  // Reabrir el último repo (IMPLEMENTATION.md, Fase 1).
  useEffect(() => {
    const last = localStorage.getItem("folio:lastRepo");
    if (last) void openRepoFlow(last);
  }, [openRepoFlow]);

  const openEntry = useCallback(
    async (collection: ContentItem, path: string) => {
      try {
        const content = await api.readEntry(path);
        setView({
          kind: "entry",
          collection,
          draft: {
            path,
            isNew: false,
            fm: content.frontmatter,
            body: content.body,
            snapshot: JSON.stringify({
              fm: content.frontmatter,
              body: content.body,
            }),
          },
        });
      } catch (e) {
        setMessage(String(e));
      }
    },
    [],
  );

  const createEntry = useCallback(
    async (collection: ContentItem, title: string, slug: string) => {
      try {
        const res = await api.createEntry(collection.name, slug);
        if (res.existed) {
          setMessage(`"${slug}" already exists — opening it`);
          await openEntry(collection, res.path);
          return;
        }
        const fm: Record<string, unknown> = {};
        for (const f of collection.fields) {
          if (f.name === "body") continue;
          if (f.name === "title") fm.title = title;
          else if (f.type === "date")
            fm[f.name] = new Date().toISOString().slice(0, 10);
          else fm[f.name] = "";
        }
        // Buffer sucio: el primer Save lo materializa (FLOW.md).
        setView({
          kind: "entry",
          collection,
          draft: { path: res.path, isNew: true, fm, body: "", snapshot: "" },
        });
      } catch (e) {
        setMessage(String(e));
      }
    },
    [openEntry],
  );

  const dirty =
    view.kind === "entry" &&
    snap(view.draft) !== view.draft.snapshot;

  const save = useCallback(async () => {
    const v = viewRef.current;
    if (v.kind !== "entry") return;
    try {
      const persist =
        v.collection.kind === "file"
          ? api.writeFileEntry(v.draft.path, v.draft.fm, v.draft.body)
          : api.writeEntry(v.draft.path, v.draft.fm, v.draft.body);
      await persist;
      setView({
        kind: "entry",
        collection: v.collection,
        draft: { ...v.draft, isNew: false, snapshot: snap(v.draft) },
      });
      refreshStatus();
      if (v.draft.isNew) await loadRows(v.collection);
      setMessage(`Saved ${v.draft.path}`);
    } catch (e) {
      setMessage(String(e));
    }
  }, [loadRows, refreshStatus]);

  const saveRef = useRef(save);
  saveRef.current = save;
  const dirtyRef = useRef(dirty);
  dirtyRef.current = dirty;

  // Cmd+S guarda (IMPLEMENTATION.md, Fase 1).
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "s") {
        e.preventDefault();
        void saveRef.current();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  // Cerrar con buffer sucio avisa (FLOW.md).
  useEffect(() => {
    const promise = getCurrentWindow().onCloseRequested(async (event) => {
      if (!dirtyRef.current) return;
      event.preventDefault();
      const saveFirst = await confirm(
        'There are unsaved changes. Save before closing?',
        { title: "Folio", kind: "warning" },
      );
      if (saveFirst) {
        await saveRef.current();
        await getCurrentWindow().close();
      }
    });
    return () => {
      void promise.then((unlisten) => unlisten());
    };
  }, []);

  const push = useCallback(async () => {
    try {
      const ok = await confirm("Push the current branch to its upstream?", {
        title: "Folio",
      });
      if (!ok) return;
      await api.push();
      refreshStatus();
      setMessage("Pushed");
    } catch (e) {
      setMessage(String(e));
    }
  }, [refreshStatus]);

  const pickFolder = useCallback(async () => {
    const selected = await openFolderDialog({
      directory: true,
      title: "Open repository folder",
    });
    if (typeof selected === "string") await openRepoFlow(selected);
  }, [openRepoFlow]);

  // ---- render ----

  if (!summary) {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-4">
        <span className="flex h-12 w-12 items-center justify-center rounded-xl bg-accent text-accent-ink">
          <NotebookPen size={24} />
        </span>
        <h1 className="text-2xl font-semibold">Folio</h1>
        <p className="max-w-xs text-center text-sm text-ink-dim">
          Edit the content of a local repository, like Pages CMS without the
          cloud.
        </p>
        <button
          onClick={pickFolder}
          className="mt-2 flex items-center gap-2 rounded-lg bg-accent px-4 py-2 text-sm font-medium text-accent-ink"
        >
          <FolderOpen size={15} /> Open folder…
        </button>
        <StatusBar message={message} status={status} config={config} />
      </div>
    );
  }

  const currentCollection =
    view.kind !== "empty" ? view.collection : null;

  return (
    <div className="flex h-full">
      <Sidebar
        summary={summary}
        status={status}
        collections={config?.content ?? []}
        selected={currentCollection?.name ?? null}
        onSelect={selectItem}
        onOpenRepo={pickFolder}
        onClone={() => setCloneOpen(true)}
      />

      <main className="relative flex min-w-0 flex-1 flex-col">
        {!config ? (
          <EmptyConfig summary={summary} onPick={pickFolder} />
        ) : view.kind === "collection" ? (
          <CollectionTable
            collection={view.collection}
            rows={rows}
            onOpen={(path) => void openEntry(view.collection, path)}
            onNew={() => setNewOpen(true)}
          />
        ) : view.kind === "entry" ? (
          <EntryEditor
            collection={view.collection}
            draft={view.draft}
            dirty={dirty}
            onFmChange={(name, value) =>
              setView((v) =>
                v.kind === "entry"
                  ? {
                      ...v,
                      draft: {
                        ...v.draft,
                        fm: { ...v.draft.fm, [name]: value },
                      },
                    }
                  : v,
              )
            }
            onBodyChange={(body) =>
              setView((v) =>
                v.kind === "entry"
                  ? { ...v, draft: { ...v.draft, body } }
                  : v,
              )
            }
            onSave={() => void save()}
            onCommit={() => setCommitOpen(true)}
            onPush={() => void push()}
            showBack={view.collection.kind === "collection"}
            onBack={() => selectCollection(view.collection)}
          />
        ) : (
          <div className="flex flex-1 items-center justify-center text-sm text-ink-dim">
            Select a collection.
          </div>
        )}
        <StatusBar message={message} status={status} config={config} />
      </main>

      {commitOpen && view.kind === "entry" && (
        <CommitDialog
          defaultMessage={`Folio: update "${String(view.draft.fm.title ?? "")}"`}
          onClose={() => setCommitOpen(false)}
          onConfirm={async (msg) => {
            setCommitOpen(false);
            try {
              const oid = await api.commit(msg);
              refreshStatus();
              setMessage(`Commit ${oid.slice(0, 8)}`);
            } catch (e) {
              setMessage(String(e));
            }
          }}
        />
      )}

      {cloneOpen && (
        <CloneDialog
          onClose={() => setCloneOpen(false)}
          onClone={async (url, dest) => {
            setCloneOpen(false);
            try {
              await api.cloneRepo(url, dest);
              await openRepoFlow(dest);
            } catch (e) {
              setMessage(String(e));
            }
          }}
        />
      )}

      {newOpen && currentCollection && (
        <NewEntryDialog
          collection={currentCollection}
          onClose={() => setNewOpen(false)}
          onCreate={(title, slug) => {
            setNewOpen(false);
            void createEntry(currentCollection, title, slug);
          }}
        />
      )}
    </div>
  );
}

function EmptyConfig({
  summary,
  onPick,
}: {
  summary: RepoSummary;
  onPick: () => void;
}) {
  return (
    <div className="flex flex-1 flex-col items-center justify-center gap-3">
      <h1 className="text-lg font-semibold">No content configuration</h1>
      <p className="max-w-md text-center text-sm text-ink-dim">
        {summary.config_error ??
          "This repository has no .pages.yml at its root. Folio reads that file to build its forms, like Pages CMS does."}
      </p>
      <button
        onClick={onPick}
        className="mt-2 rounded-lg border border-line px-4 py-2 text-sm hover:bg-panel"
      >
        Choose another folder
      </button>
    </div>
  );
}

function StatusBar({
  message,
  status,
  config,
}: {
  message: string;
  status: RepoStatus | null;
  config: PagesConfig | null;
}) {
  return (
    <footer className="flex h-7 shrink-0 items-center gap-4 border-t border-line bg-panel/60 px-4 text-[11px] text-ink-dim">
      {status && (
        <>
          <span>{status.branch}</span>
          {status.dirty && <span className="text-amber-400">● uncommitted</span>}
          {status.ahead > 0 && <span>↑{status.ahead} to push</span>}
          {status.behind > 0 && <span>↓{status.behind} to pull</span>}
          {!status.has_upstream && <span>no upstream</span>}
        </>
      )}
      {config && config.warnings.length > 0 && (
        <span title={config.warnings.join("\n")}>
          ⚠ {config.warnings.length} config warning(s)
        </span>
      )}
      <span className="ml-auto truncate">{message}</span>
    </footer>
  );
}
