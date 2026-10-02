import { useCallback, useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { confirm, open as openFolderDialog } from "@tauri-apps/plugin-dialog";
import { FolderOpen, NotebookPen } from "lucide-react";
import { api } from "./api";
import type {
  ContentItem,
  MediaRef,
  PagesConfig,
  RepoStatus,
  RepoSummary,
} from "./types";
import { Sidebar } from "./components/Sidebar";
import { CollectionTable, type EntryRow } from "./components/CollectionTable";
import { EntryEditor, type Draft } from "./components/EntryEditor";
import {
  DEFAULT_LAYOUT,
  Canvas,
  slugOf,
  type CanvasLayout,
} from "./components/Canvas";
import {
  CloneDialog,
  CommitDialog,
  DiscardDialog,
  NewEntryDialog,
} from "./components/Dialogs";
import { MediaView } from "./components/Media";

// El canvas (v2) vive en la colección `showcase` del config (PRD.md).
const CANVAS_NAME = "showcase";

type View =
  | { kind: "empty" }
  | { kind: "media" }
  | { kind: "canvas"; collection: ContentItem }
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
  const [mediaItems, setMediaItems] = useState<MediaRef[]>([]);
  /** Navegación bloqueada por cambios sin guardar: run() al resolver. */
  const [guard, setGuard] = useState<null | "close" | { run: () => void }>(
    null,
  );
  const [canvasRows, setCanvasRows] = useState<
    { path: string; values: Record<string, unknown> }[]
  >([]);
  const [canvasPath, setCanvasPath] = useState("");
  const [canvasLayout, setCanvasLayout] = useState<CanvasLayout | null>(null);
  const [canvasSnapshot, setCanvasSnapshot] = useState("");
  const [canvasReturn, setCanvasReturn] = useState(false);

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

  const loadMedia = useCallback(async () => {
    try {
      const items = await api.listMedia();
      setMediaItems(items);
    } catch {
      // Sin media.input declarado no hay librería; la UI lo explica.
      setMediaItems([]);
    }
  }, []);

  const uploadMedia = useCallback(async (): Promise<MediaRef | null> => {
    try {
      const files = await openFolderDialog({
        multiple: true,
        title: "Choose images",
        filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg", "gif", "webp", "avif", "svg"] }],
      });
      if (!Array.isArray(files) || files.length === 0) return null;
      let last: MediaRef | null = null;
      for (const f of files) {
        last = await api.importMedia(f);
      }
      await loadMedia();
      setMessage(`Imported ${files.length} file(s)`);
      return last;
    } catch (e) {
      setMessage(String(e));
      return null;
    }
  }, [loadMedia]);

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

  // El canvas lee la colección showcase + su layout.json (FLOW.md v2).
  // Las fichas sin nodo se materializan en cascada para que el primer
  // snapshot sea estable (dirty solo tras un cambio real).
  const openCanvas = useCallback(async (c: ContentItem) => {
    try {
      const layoutPath = `${c.path}/layout.json`;
      let layout: CanvasLayout = DEFAULT_LAYOUT;
      try {
        const raw = await api.readFileEntry(layoutPath);
        const data = raw.frontmatter as Partial<CanvasLayout>;
        layout = {
          viewport: data.viewport ?? DEFAULT_LAYOUT.viewport,
          nodes: Array.isArray(data.nodes) ? data.nodes : [],
        };
      } catch {
        // Sin layout.json todavía: arranca con el default.
      }
      const list = await api.listEntries(c.path);
      const cards = await Promise.all(
        list.map(async (e) => ({
          path: e.path,
          values: (await api.readEntry(e.path)).frontmatter,
        })),
      );
      const nodes = [...layout.nodes];
      cards.forEach((card, i) => {
        const id = slugOf(card.path);
        if (!nodes.some((n) => n.id === id)) {
          nodes.push({
            id,
            x: 60 + (i % 4) * 260,
            y: 60 + Math.floor(i / 4) * 240,
            w: 220,
          });
        }
      });
      const full: CanvasLayout = { viewport: layout.viewport, nodes };
      setCanvasRows(cards);
      setCanvasPath(layoutPath);
      setCanvasLayout(full);
      setCanvasSnapshot(JSON.stringify(full));
      setView({ kind: "canvas", collection: c });
    } catch (e) {
      setMessage(String(e));
    }
  }, []);

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
        void loadMedia();
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

  const entryDirty =
    view.kind === "entry" && snap(view.draft) !== view.draft.snapshot;
  const canvasDirty =
    view.kind === "canvas" &&
    canvasLayout !== null &&
    JSON.stringify(canvasLayout) !== canvasSnapshot;
  const dirty = entryDirty || canvasDirty;

  const save = useCallback(async () => {
    const v = viewRef.current;
    if (v.kind === "canvas") {
      try {
        await api.writeFileEntry(
          canvasPath,
          canvasLayout as unknown as Record<string, unknown>,
          "",
        );
        setCanvasSnapshot(JSON.stringify(canvasLayout));
        refreshStatus();
        setMessage(`Saved ${canvasPath}`);
      } catch (e) {
        setMessage(String(e));
      }
      return;
    }
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
  }, [canvasLayout, canvasPath, loadRows, refreshStatus]);

  const saveRef = useRef(save);
  saveRef.current = save;
  const dirtyRef = useRef(dirty);
  dirtyRef.current = dirty;

  // Cambiar de vista o de repo con buffer sucio avisa: guardar,
  // descartar o cancelar (FLOW.md).
  const guardNav = useCallback((run: () => void) => {
    if (dirtyRef.current) setGuard({ run });
    else run();
  }, []);

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

  // Cerrar con buffer sucio avisa: guardar, descartar o cancelar
  // (FLOW.md). Un commit ya hecho no bloquea.
  useEffect(() => {
    const promise = getCurrentWindow().onCloseRequested(async (event) => {
      if (!dirtyRef.current) return;
      event.preventDefault();
      setGuard("close");
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
    view.kind === "collection" || view.kind === "entry"
      ? view.collection
      : null;

  return (
    <div className="flex h-full">
      <Sidebar
        summary={summary}
        status={status}
        collections={config?.content ?? []}
        selected={
          view.kind === "media"
            ? "__media__"
            : currentCollection?.name ?? null
        }
        onSelect={(c) => guardNav(() => selectItem(c))}
        onOpenRepo={() => guardNav(() => void pickFolder())}
        onClone={() => guardNav(() => setCloneOpen(true))}
        onMedia={() =>
          guardNav(() => {
            setView({ kind: "media" });
            void loadMedia();
          })
        }
        onCanvas={
          config?.content.some((c) => c.name === CANVAS_NAME)
            ? () => {
                const c = config.content.find((x) => x.name === CANVAS_NAME);
                if (c) guardNav(() => void openCanvas(c));
              }
            : undefined
        }
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
        ) : view.kind === "canvas" ? (
          <Canvas
            collection={view.collection}
            root={summary.root}
            mediaInput={config?.media.input ?? null}
            cards={canvasRows}
            layout={canvasLayout ?? DEFAULT_LAYOUT}
            dirty={canvasDirty}
            onChange={(l) => setCanvasLayout(l)}
            onOpenEntry={(path) => {
              setCanvasReturn(true);
              void openEntry(view.collection, path);
            }}
            onSave={() => void save()}
            onCommit={() => setCommitOpen(true)}
            onPush={() => void push()}
          />
        ) : view.kind === "media" ? (
          <MediaView
            root={summary.root}
            items={mediaItems}
            hasMediaInput={Boolean(config?.media.input)}
            onUpload={() => void uploadMedia()}
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
            root={summary.root}
            mediaInput={config?.media.input ?? null}
            mediaItems={mediaItems}
            onUploadMedia={uploadMedia}
            onBack={() => {
              if (canvasReturn && view.collection.name === CANVAS_NAME) {
                setView({ kind: "canvas", collection: view.collection });
              } else {
                selectCollection(view.collection);
              }
            }}
          />
        ) : (
          <div className="flex flex-1 items-center justify-center text-sm text-ink-dim">
            Select a collection.
          </div>
        )}
        <StatusBar message={message} status={status} config={config} />
      </main>

      {guard && (
        <DiscardDialog
          closeLabel={guard === "close" ? "Save & Close" : "Save"}
          onSave={async () => {
            const g = guard;
            setGuard(null);
            await saveRef.current();
            if (g === "close") {
              await getCurrentWindow().close();
            } else {
              g.run();
            }
          }}
          onDiscard={() => {
            const g = guard;
            setGuard(null);
            if (g === "close") {
              // Descartar cierra sin volver a preguntar.
              getCurrentWindow().destroy();
            } else {
              g.run();
            }
          }}
          onCancel={() => setGuard(null)}
        />
      )}

      {commitOpen && (view.kind === "entry" || view.kind === "canvas") && (
        <CommitDialog
          defaultMessage={
            view.kind === "canvas"
              ? "Folio: update canvas layout"
              : `Folio: update "${String(view.draft.fm.title ?? "")}"`
          }
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
