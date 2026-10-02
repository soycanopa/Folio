import { useCallback, useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { confirm, open as openFolderDialog } from "@tauri-apps/plugin-dialog";
import { api } from "./api";
import type {
  ContentItem,
  GithubUser,
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
  OpenModeDialog,
} from "./components/Dialogs";
import { MediaView } from "./components/Media";
import { HomePage } from "./components/home/home-page";
import { getVisits, trackVisit } from "./lib/tracker";
import { toast } from "sonner";
import { SignInScreen } from "./components/SignInScreen";

// El canvas (v2) vive en la colección `showcase` del config (PRD.md).
const CANVAS_NAME = "showcase";

type View =
  | { kind: "home" }
  | { kind: "signin" }
  | { kind: "empty" }
  | { kind: "media" }
  | { kind: "canvas"; collection: ContentItem }
  | { kind: "collection"; collection: ContentItem }
  | { kind: "entry"; collection: ContentItem; draft: Draft };

// Zona de agarre del Overlay (sin title bar visible): franja
// transparente de 28px arriba de todo — donde viven los semáforos —
// presente en toda pantalla. Llama directo a la API de window en vez
// del script interno de drag; si falla, el error se muestra AQUÍ en
// rojo para diagnosticar en vivo (permiso, capa nativa, lo que sea).
function DragStrip() {
  const [dragError, setDragError] = useState<string | null>(null);
  return (
    <div
      className="relative h-7 shrink-0"
      onMouseDown={(e) => {
        if (e.button !== 0) return;
        e.preventDefault();
        getCurrentWindow()
          .startDragging()
          .then(() => setDragError(null))
          .catch((err) => setDragError(String(err)));
      }}
    >
      {dragError && (
        <span className="absolute left-20 top-0 z-50 truncate text-[10px] text-red-400">
          drag: {dragError}
        </span>
      )}
    </div>
  );
}

function Shell({ children }: { children: React.ReactNode }) {
  return (
    <div className="flex h-full flex-col">
      <DragStrip />
      <div className="min-h-0 flex-1">{children}</div>
    </div>
  );
}

// El mensaje de su server al guardar, incluido el aviso de auto-rename.
function savedMessage(path: string, savedAs: string): string {
  return savedAs !== path
    ? `File "${path}" saved successfully but renamed to "${savedAs}" to avoid naming conflict.`
    : `File "${path}" saved successfully.`;
}

// sonner tipa toast.promise como Promise | { unwrap } según versión.
async function toastPromise<T>(
  p: Promise<T>,
  msgs: {
    loading: string;
    success: (v: T) => string;
    error: (e: unknown) => string;
  },
): Promise<T> {
  const t = toast.promise(p, msgs);
  if (t instanceof Promise) return t;
  return t.unwrap();
}

const snap = (d: { fm: Record<string, unknown>; body: string }) =>
  JSON.stringify({ fm: d.fm, body: d.body });

export default function App() {
  const [summary, setSummary] = useState<RepoSummary | null>(null);
  const [config, setConfig] = useState<PagesConfig | null>(null);
  const [status, setStatus] = useState<RepoStatus | null>(null);
  const [view, setView] = useState<View>({ kind: "home" });
  const [ghSession, setGhSession] = useState<GithubUser | null>(null);
  const [rows, setRows] = useState<EntryRow[]>([]);
  const [message, setMessage] = useState("");
  const [commitOpen, setCommitOpen] = useState(false);
  const [newOpen, setNewOpen] = useState(false);
  const [cloneOpen, setCloneOpen] = useState(false);
  /** Repo elegido esperando la pregunta local-vs-remoto. */
  const [openModeFor, setOpenModeFor] = useState<{
    owner: string;
    repo: string;
    branch: string;
  } | null>(null);
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
  const summaryRef = useRef(summary);
  summaryRef.current = summary;

  const refreshStatus = useCallback(() => {
    // Remoto: cada save publica su commit en la rama; no hay working
    // tree ni upstream de los que informar.
    if (summaryRef.current?.mode === "remote") return;
    api.repoStatus().then(setStatus).catch((e) => setMessage(String(e)));
  }, []);

  // El core invalida la sesión en un 401 (remote::gh_error_ui, como su
  // GithubAuthExpired) y sus errores de sesión llevan el texto
  // "sesión de GitHub": al verlo, el home deja de creer que hay sesión.
  const report = useCallback((e: unknown) => {
    const msg = String(e);
    setMessage(msg);
    if (msg.includes("sesión de GitHub")) {
      api.githubSession().then(setGhSession).catch(() => undefined);
    }
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
      report(e);
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
        title: "Choose files",
      });
      if (!Array.isArray(files) || files.length === 0) return null;
      const remote = summaryRef.current?.mode === "remote";
      let last: MediaRef | null = null;
      for (const f of files) {
        // Remoto: cada archivo publica su commit — un toast por archivo,
        // como su media-upload.
        last = remote
          ? await toastPromise(api.importMedia(f), {
              loading: `Uploading ${f.split("/").pop()}`,
              success: (m: MediaRef) => `Uploaded ${m.name}`,
              error: (e: unknown) => String(e),
            })
          : await api.importMedia(f);
      }
      await loadMedia();
      if (!remote) setMessage(`Imported ${files.length} file(s)`);
      return last;
    } catch (e) {
      report(e);
      return null;
    }
  }, [loadMedia]);

  const deleteMedia = useCallback(
    async (m: MediaRef) => {
      try {
        await api.deleteMedia(m.path);
        await loadMedia();
        refreshStatus();
        const remote = summaryRef.current?.mode === "remote";
        setMessage(
          remote ? `Deleted ${m.path}` : `Deleted ${m.path} — commit to apply`,
        );
      } catch (e) {
        report(e);
      }
    },
    [loadMedia, refreshStatus],
  );

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
      report(e);
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
      report(e);
    }
  }, []);

  const openRepoFlow = useCallback(
    async (path: string) => {
      try {
        const s = await api.openRepo(path);
        setSummary(s);
        api.addRecentRepo(path).catch(() => undefined);
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
        report(e);
      }
    },
    [refreshStatus, selectCollection],
  );

  // El home arranca con los proyectos recientes; la lista vieja de
  // localStorage migra una única vez al disco del core.
  useEffect(() => {
    api
      .githubSession()
      .then((session) => {
        setGhSession(session);
      })
      .catch(() => undefined);
    const last = localStorage.getItem("folio:lastRepo");
    api
      .listRecentRepos()
      .then(async (list) => {
        if (last && list.length === 0) {
          await api.addRecentRepo(last);
          localStorage.removeItem("folio:lastRepo");
        }
      })
      .catch(() => undefined);
  }, []);

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
        report(e);
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
        report(e);
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
  // Como su isBusy: mientras publica no se dispara otro save (un doble
  // Cmd+S en remoto publicaría dos commits).
  const [saving, setSaving] = useState(false);
  const savingRef = useRef(false);

  const save = useCallback(async () => {
    // Como su isBusy: mientras publica no se dispara otro save (un doble
    // Cmd+S en remoto publicaría dos commits).
    if (savingRef.current) return;
    savingRef.current = true;
    setSaving(true);
    try {
      const v = viewRef.current;
      const remote = summaryRef.current?.mode === "remote";
      if (v.kind === "canvas") {
        const persist = api.writeFileEntry(
          canvasPath,
          canvasLayout as unknown as Record<string, unknown>,
          "",
        );
        // Remoto: Save publica directo en la rama (decisión del dueño);
        // toast como su entry.tsx ("Saving your file" → mensaje del save).
        const path = remote
          ? await toastPromise(persist, {
              loading: "Saving your file",
              success: (p: string) => savedMessage(canvasPath, p),
              error: (e: unknown) => String(e),
            })
          : await persist;
        setCanvasSnapshot(JSON.stringify(canvasLayout));
        if (path !== canvasPath) setCanvasPath(path);
        if (!remote) refreshStatus();
        else setMessage("");
        return;
      }
      if (v.kind !== "entry") return;
      const persist =
        v.collection.kind === "file"
          ? api.writeFileEntry(v.draft.path, v.draft.fm, v.draft.body)
          : api.writeEntry(v.draft.path, v.draft.fm, v.draft.body);
      const path = remote
        ? await toastPromise(persist, {
            loading: "Saving your file",
            success: (p: string) => savedMessage(v.draft.path, p),
            error: (e: unknown) => String(e),
          })
        : await persist;
      setView({
        kind: "entry",
        collection: v.collection,
        // El auto-rename del 422 puede cambiar el path final.
        draft: { ...v.draft, path, isNew: false, snapshot: snap(v.draft) },
      });
      if (remote) {
        void loadRows(v.collection);
        setMessage("");
      } else {
        refreshStatus();
        if (v.draft.isNew) await loadRows(v.collection);
        setMessage(`Saved ${path}`);
      }
    } catch (e) {
      report(e);
    } finally {
      savingRef.current = false;
      setSaving(false);
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

  const goHome = useCallback(() => {
    guardNav(() => setView({ kind: "home" }));
  }, [guardNav]);



  const handleSignOut = useCallback(async () => {
    try {
      await api.githubLogout();
      setGhSession(null);
    } catch (e) {
      report(e);
    }
  }, []);

  // Proyecto remoto: sin carpeta, rama por defecto, edición contra la
  // API de GitHub (AGENTS.md). El Save publica directo en la rama.
  const openRemoteRepoFlow = useCallback(
    async (owner: string, repo: string) => {
      try {
        const s = await api.openRemoteRepo(owner, repo);
        setSummary(s);
        trackVisit(owner, repo, s.branch, "remote");
        setStatus(null);
        setView({ kind: "empty" });
        setRows([]);
        if (!s.has_config) {
          setConfig(null);
          return;
        }
        const cfg = await api.readConfig();
        setConfig(cfg);
        void loadMedia();
        if (cfg.content.length > 0) selectCollection(cfg.content[0]);
      } catch (e) {
        report(e);
      }
    },
    [loadMedia, selectCollection],
  );

  // El Open de su home pregunta local (clona en la carpeta que elijas)
  // o remoto (como la web, sin clonar) — salvo que el tracker ya sepa
  // cómo se abrió antes: los recientes reabren directo.
  const handleOpenRepo = useCallback(
    (visit: { owner: string; repo: string; branch: string }) => {
      const known = getVisits().find(
        (v) =>
          v.owner.toLowerCase() === visit.owner.toLowerCase() &&
          v.repo.toLowerCase() === visit.repo.toLowerCase(),
      );
      if (known?.mode === "remote") {
        void openRemoteRepoFlow(visit.owner, visit.repo);
        return;
      }
      setOpenModeFor(visit);
    },
    [openRemoteRepoFlow],
  );

  const cloneVisit = useCallback(
    async (visit: { owner: string; repo: string; branch: string }) => {
      try {
        const dest = await openFolderDialog({
          directory: true,
          title: `Choose where to clone ${visit.repo}`,
        });
        if (typeof dest !== "string") return;
        const target = `${dest}/${visit.repo}`;
        await api.cloneRepo(
          `https://github.com/${visit.owner}/${visit.repo}.git`,
          target,
        );
        trackVisit(visit.owner, visit.repo, visit.branch, "local");
        await openRepoFlow(target);
      } catch (e) {
        report(e);
      }
    },
    [openRepoFlow],
  );

  const handleCreateTemplate = useCallback(
    async (repository: string, name: string): Promise<string | null> => {
      try {
        const fullName = await api.githubCreateFromTemplate(repository, name);
        return `Created ${fullName} on your account.`;
      } catch (e) {
        toast.error(String(e));
        return null;
      }
    },
    [],
  );

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
    if (summaryRef.current?.mode === "remote") return;
    try {
      const ok = await confirm("Push the current branch to its upstream?", {
        title: "Folio",
      });
      if (!ok) return;
      await api.push();
      refreshStatus();
      setMessage("Pushed");
    } catch (e) {
      report(e);
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

  if (view.kind === "signin") {
    return (
      <Shell>
        <SignInScreen
          onStart={api.githubLoginStart}
          onPoll={api.githubLoginPoll}
          onAuthorized={async () => {
            const session = await api.githubSession().catch(() => null);
            setGhSession(session);
            setView({ kind: "home" });
          }}
        />
      </Shell>
    );
  }

  const homeUser = ghSession
    ? { accounts: [{ login: ghSession.login, repositorySelection: "all" as const }] }
    : null;

  // La pregunta local-vs-remoto vive en las dos ramas: sin proyecto
  // abierto el return temprano del home se la saltaba (bug: Open no
  // hacía nada visible).
  const openModeDialog = openModeFor && (
    <OpenModeDialog
      owner={openModeFor.owner}
      repo={openModeFor.repo}
      onClose={() => setOpenModeFor(null)}
      onLocal={() => {
        const v = openModeFor;
        setOpenModeFor(null);
        void cloneVisit(v);
      }}
      onRemote={() => {
        const v = openModeFor;
        setOpenModeFor(null);
        void openRemoteRepoFlow(v.owner, v.repo);
      }}
    />
  );

  if (!summary) {
    return (
      <Shell>
        <HomePage
          user={homeUser}
          loadRepos={(keyword) => api.githubListRepos(keyword)}
          onOpenRepo={(v) => void handleOpenRepo(v)}
          onCreateTemplate={handleCreateTemplate}
          onSignIn={() => setView({ kind: "signin" })}
          onSignOut={() => void handleSignOut()}
        />
        {openModeDialog}
      </Shell>
    );
  }

  const currentCollection =
    view.kind === "collection" || view.kind === "entry"
      ? view.collection
      : null;

  return (
    <Shell>
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
        onHome={goHome}
      />

      <main className="relative flex min-w-0 flex-1 flex-col">
        {!config ? (
          <EmptyConfig
            summary={summary}
            remote={summary.mode === "remote"}
            onPick={summary.mode === "remote" ? goHome : pickFolder}
          />
        ) : view.kind === "home" ? (
          <HomePage
            user={homeUser}
            loadRepos={(keyword) => api.githubListRepos(keyword)}
            onSignOut={() => void handleSignOut()}
            onOpenRepo={(v) => guardNav(() => void handleOpenRepo(v))}
            onCreateTemplate={(t, n) =>
              new Promise<string | null>((resolve) => {
                guardNav(() => {
                  void handleCreateTemplate(t, n).then(resolve);
                });
              })
            }
            onSignIn={() => setView({ kind: "signin" })}
          />
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
            remote={summary.mode === "remote"}
            busy={saving}
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
            onCommit={summary.mode === "local" ? () => setCommitOpen(true) : undefined}
            onPush={summary.mode === "local" ? () => void push() : undefined}
          />
        ) : view.kind === "media" ? (
          <MediaView
            root={summary.root}
            remote={summary.mode === "remote"}
            items={mediaItems}
            hasMediaInput={Boolean(config?.media.input)}
            onUpload={() => void uploadMedia()}
            onDelete={(m) => void deleteMedia(m)}
          />
        ) : view.kind === "entry" ? (
          <EntryEditor
            collection={view.collection}
            draft={view.draft}
            dirty={dirty}
            remote={summary.mode === "remote"}
            busy={saving}
            seeAllChangesUrl={
              summary.owner_repo
                ? `https://github.com/${summary.owner_repo}/commits/${encodeURIComponent(
                    summary.branch,
                  )}/${view.draft.path
                    .split("/")
                    .map(encodeURIComponent)
                    .join("/")}`
                : undefined
            }
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
            onCommit={summary.mode === "local" ? () => setCommitOpen(true) : undefined}
            onPush={summary.mode === "local" ? () => void push() : undefined}
            onRename={async (newName) => {
              const v = viewRef.current;
              if (v.kind !== "entry") return;
              try {
                const newPath = await api.renameEntry(v.draft.path, newName);
                refreshStatus();
                setMessage(
                  summary.mode === "remote"
                    ? `Renamed to ${newName}`
                    : `Renamed to ${newName} — commit to apply`,
                );
                await openEntry(v.collection, newPath);
              } catch (e) {
                report(e);
              }
            }}
            onDelete={async () => {
              const v = viewRef.current;
              if (v.kind !== "entry") return;
              try {
                await api.deleteEntry(v.draft.path);
                refreshStatus();
                setMessage(
                  summary.mode === "remote"
                    ? `Deleted ${v.draft.path}`
                    : `Deleted ${v.draft.path} — commit to apply`,
                );
                selectCollection(v.collection);
              } catch (e) {
                report(e);
              }
            }}
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
        <StatusBar
          message={message}
          status={status}
          config={config}
          summary={summary}
          dirty={dirty}
        />
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
              report(e);
            }
          }}
        />
      )}

      {openModeDialog}

      {cloneOpen && (
        <CloneDialog
          onClose={() => setCloneOpen(false)}
          onClone={async (url, dest) => {
            setCloneOpen(false);
            try {
              await api.cloneRepo(url, dest);
              await openRepoFlow(dest);
            } catch (e) {
              report(e);
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
    </Shell>
  );
}

function EmptyConfig({
  summary,
  remote,
  onPick,
}: {
  summary: RepoSummary;
  /** Remoto: el botón vuelve al home; no hay carpeta que elegir. */
  remote?: boolean;
  onPick: () => void;
}) {
  return (
    <div className="flex flex-1 flex-col items-center justify-center gap-3">
      <h1 className="text-lg font-semibold">No content configuration</h1>
      <p className="max-w-md text-center text-sm text-ink-dim">
        {summary.config_error ??
          `This repository has no .pages.yml at its root. Folio reads that file to build its forms, like Pages CMS does.`}
      </p>
      <button
        onClick={onPick}
        className="mt-2 rounded-lg border border-line px-4 py-2 text-sm hover:bg-panel"
      >
        {remote ? "Choose another repository" : "Choose another folder"}
      </button>
    </div>
  );
}

function StatusBar({
  message,
  status,
  config,
  summary,
  dirty,
}: {
  message: string;
  status: RepoStatus | null;
  config: PagesConfig | null;
  summary: RepoSummary;
  dirty: boolean;
}) {
  const remote = summary.mode === "remote";
  return (
    <footer className="flex h-7 shrink-0 items-center gap-4 border-t border-line bg-panel/60 px-4 text-[11px] text-ink-dim">
      {remote ? (
        <>
          <span className="text-ink">{summary.branch}</span>
          <span className="text-sky-400">◆ GitHub (remote)</span>
          {dirty && <span className="text-amber-400">● unsaved</span>}
        </>
      ) : (
        status && (
          <>
            <span>{status.branch}</span>
            {status.dirty && (
              <span className="text-amber-400">● uncommitted</span>
            )}
            {status.ahead > 0 && <span>↑{status.ahead} to push</span>}
            {status.behind > 0 && <span>↓{status.behind} to pull</span>}
            {!status.has_upstream && <span>no upstream</span>}
          </>
        )
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
