import { useEffect, useMemo, useRef, useState } from "react";
import {
  CircleCheck,
  CircleX,
  EllipsisVertical,
  ExternalLink,
  Loader,
  RotateCw,
  Search,
  XCircle,
} from "lucide-react";
import { toast } from "sonner";
import { api } from "../../api";
import type { Action, ActionRunInfo } from "../../types";
import { formatRunState, isRunActive, relativeTime } from "./runs";

/*
 * Portado de Pages CMS (https://github.com/hunvreus/pagescms) — MIT License.
 * Su actions-page.tsx: tabla de corridas con polling cada 4s (pausado con
 * la ventana oculta), filtros y menú por fila (View on GitHub / Run again /
 * Cancel run). Divergencia documentada: su server guardaba el contexto de
 * cada corrida en su DB; GitHub no lo expone, así que Folio no tiene la
 * columna Context — el resto es copia.
 */

const PAGE_SIZE = 25;

interface ActionsPageProps {
  actions: Action[];
}

export function ActionsPage({ actions }: ActionsPageProps) {
  const [runs, setRuns] = useState<ActionRunInfo[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [statusFilter, setStatusFilter] = useState<"all" | "succeeded" | "failed" | "pending">("all");
  const [actionFilter, setActionFilter] = useState("all");
  const [menuFor, setMenuFor] = useState<number | null>(null);
  const [busyRun, setBusyRun] = useState<number | null>(null);
  const alive = useRef(true);

  const workflows = useMemo(
    () => Array.from(new Set(actions.map((a) => a.workflow))),
    [actions],
  );
  const labelOf = useMemo(() => {
    const map = new Map<string, string>();
    for (const a of actions) {
      if (!map.has(a.workflow)) map.set(a.workflow, a.label);
      if (!map.has(a.name)) map.set(a.name, a.label);
    }
    return map;
  }, [actions]);
  const cancelableOf = useMemo(() => {
    const map = new Map<string, boolean>();
    for (const a of actions) {
      if (!map.has(a.workflow)) map.set(a.workflow, a.cancelable !== false);
    }
    return map;
  }, [actions]);

  const refresh = async () => {
    if (workflows.length === 0) {
      setRuns([]);
      return;
    }
    try {
      const list = await api.actionRuns(workflows, 50);
      if (alive.current) {
        setRuns(list);
        setError(null);
      }
    } catch (e) {
      if (alive.current) setError(String(e));
    }
  };

  useEffect(() => {
    alive.current = true;
    void refresh();
    // Su polling: cada 4s, saltando cuando la pestaña está oculta (aquí
    // la ventana es única, queda el guard por si acaso).
    const interval = setInterval(() => {
      if (!document.hidden) void refresh();
    }, 4000);
    return () => {
      alive.current = false;
      clearInterval(interval);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [workflows.join("|")]);

  const filtered = useMemo(() => {
    let list = runs ?? [];
    if (statusFilter !== "all") {
      list = list.filter((r) => {
        if (statusFilter === "succeeded")
          return r.status === "completed" && r.conclusion === "success";
        if (statusFilter === "failed")
          return r.status === "completed" && r.conclusion !== "success";
        return r.status !== "completed"; // pending
      });
    }
    if (actionFilter !== "all") {
      list = list.filter((r) => r.workflow === actionFilter);
    }
    const q = query.trim().toLowerCase();
    if (q) {
      list = list.filter((r) =>
        [
          labelOf.get(r.workflow) ?? r.workflow,
          r.workflow,
          r.head_branch,
          r.head_sha,
          r.triggered_by,
        ]
          .filter(Boolean)
          .some((s) => (s as string).toLowerCase().includes(q)),
      );
    }
    return list;
  }, [runs, statusFilter, actionFilter, query, labelOf]);

  const runAgain = async (run: ActionRunInfo) => {
    setBusyRun(run.id);
    try {
      await api.rerunActionRun(run.id);
      toast.success("Rerun started.");
      void refresh();
    } catch (e) {
      toast.error(String(e));
    } finally {
      setBusyRun(null);
      setMenuFor(null);
    }
  };

  const cancelRun = async (run: ActionRunInfo) => {
    setBusyRun(run.id);
    try {
      await api.cancelActionRun(run.id);
      toast.success("Run cancelled.");
      void refresh();
    } catch (e) {
      toast.error(String(e));
    } finally {
      setBusyRun(null);
      setMenuFor(null);
    }
  };

  return (
    <div className="flex h-full min-h-0 flex-col gap-3 p-6">
      <div className="flex items-center justify-between gap-3">
        <h1 className="text-lg font-semibold">Actions</h1>
        <div className="flex items-center gap-2">
          <div className="relative">
            <Search size={13} className="absolute left-2.5 top-2.5 text-ink-dim" />
            <input
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder="Search"
              className="w-44 rounded-lg bg-panel py-1.5 pl-7 pr-3 text-sm outline-none placeholder:text-ink-dim focus:ring-1 focus:ring-ring"
            />
          </div>
          <select
            value={statusFilter}
            onChange={(e) => setStatusFilter(e.target.value as typeof statusFilter)}
            className="rounded-lg bg-panel px-2 py-1.5 text-sm outline-none focus:ring-1 focus:ring-ring"
          >
            <option value="all">All statuses</option>
            <option value="succeeded">Succeeded</option>
            <option value="failed">Failed</option>
            <option value="pending">Pending</option>
          </select>
          <select
            value={actionFilter}
            onChange={(e) => setActionFilter(e.target.value)}
            className="rounded-lg bg-panel px-2 py-1.5 text-sm outline-none focus:ring-1 focus:ring-ring"
          >
            <option value="all">All actions</option>
            {Array.from(labelOf.entries()).map(([wf, label]) => (
              <option key={wf} value={wf}>
                {label}
              </option>
            ))}
          </select>
        </div>
      </div>

      {error ? (
        <div className="flex flex-1 items-center justify-center">
          <div className="max-w-sm text-center">
            <p className="font-medium">Something went wrong</p>
            <p className="mt-1 text-xs text-ink-dim">{error}</p>
          </div>
        </div>
      ) : runs === null ? (
        <div className="flex-1 space-y-2">
          {[0, 1, 2].map((i) => (
            <div key={i} className="h-10 animate-pulse rounded-lg bg-panel" />
          ))}
        </div>
      ) : filtered.length === 0 ? (
        <div className="flex flex-1 items-center justify-center">
          <div className="text-center">
            <p className="font-medium">{runs.length === 0 ? "No actions yet." : "No matching actions."}</p>
            {runs.length === 0 && (
              <p className="mt-1 text-xs text-ink-dim">
                Declare actions in your .pages.yml to trigger GitHub workflows.
              </p>
            )}
          </div>
        </div>
      ) : (
        <div className="min-h-0 flex-1 overflow-y-auto">
          <table className="w-full text-left text-sm">
            <thead className="text-xs uppercase tracking-wider text-ink-dim">
              <tr>
                <th className="w-8 px-2 py-2" />
                <th className="px-2 py-2 font-medium">Name</th>
                <th className="px-2 py-2 font-medium">Triggered</th>
                <th className="px-2 py-2 font-medium">Triggered by</th>
                <th className="px-2 py-2 font-medium">Ref</th>
                <th className="w-10 px-2 py-2" />
              </tr>
            </thead>
            <tbody>
              {filtered.slice(0, PAGE_SIZE).map((run) => {
                const label = labelOf.get(run.workflow) ?? run.workflow;
                const active = isRunActive(run);
                const cancelable = cancelableOf.get(run.workflow) !== false;
                return (
                  <tr key={run.id} className="border-t border-line">
                    <td className="px-2 py-2">
                      <span title={formatRunState(run)}>
                        {active ? (
                          <Loader size={15} className="animate-spin text-ink-dim" />
                        ) : run.conclusion === "success" ? (
                          <CircleCheck size={15} className="text-primary" />
                        ) : (
                          <CircleX size={15} className="text-danger" />
                        )}
                      </span>
                    </td>
                    <td className="px-2 py-2 font-medium">{label}</td>
                    <td className="px-2 py-2 text-ink-dim">{relativeTime(run.created_at)}</td>
                    <td className="px-2 py-2">
                      {run.triggered_by ? (
                        <span className="flex items-center gap-1.5">
                          <img
                            src={`https://github.com/${run.triggered_by}.png`}
                            alt={run.triggered_by}
                            className="h-5 w-5 rounded-full"
                          />
                          {run.triggered_by}
                        </span>
                      ) : (
                        <span className="text-ink-dim">—</span>
                      )}
                    </td>
                    <td className="px-2 py-2 font-mono text-xs text-ink-dim">
                      {run.html_url && run.head_sha ? (
                        <a
                          href={`https://github.com/${run.html_url.match(/github\.com\/([^/]+\/[^/]+)\//)?.[1] ?? ""}/commit/${run.head_sha}`}
                          target="_blank"
                          rel="noreferrer"
                          className="hover:text-ink hover:underline"
                        >
                          {run.head_branch ?? "ref"}@{run.head_sha.slice(0, 7)}
                        </a>
                      ) : (
                        `${run.head_branch ?? "ref"}${run.head_sha ? `@${run.head_sha.slice(0, 7)}` : ""}`
                      )}
                    </td>
                    <td className="relative px-2 py-2 text-right">
                      <button
                        onClick={() => setMenuFor(menuFor === run.id ? null : run.id)}
                        className="rounded-md p-1 text-ink-dim hover:text-ink"
                        title="Run actions"
                      >
                        {busyRun === run.id ? (
                          <Loader size={14} className="animate-spin" />
                        ) : (
                          <EllipsisVertical size={14} />
                        )}
                      </button>
                      {menuFor === run.id && (
                        <>
                          <div className="fixed inset-0 z-10" onClick={() => setMenuFor(null)} />
                          <div className="absolute right-2 top-full z-20 mt-1 w-44 overflow-hidden rounded-lg border border-line bg-panel py-1 text-sm shadow-xl">
                            <a
                              href={run.html_url ?? "#"}
                              target="_blank"
                              rel="noreferrer"
                              className={`flex w-full items-center gap-2 px-3 py-1.5 hover:bg-raised ${run.html_url ? "" : "pointer-events-none opacity-40"}`}
                            >
                              <ExternalLink size={13} /> View on GitHub
                            </a>
                            <button
                              onClick={() => void runAgain(run)}
                              disabled={active || busyRun === run.id}
                              className="flex w-full items-center gap-2 px-3 py-1.5 text-left hover:bg-raised disabled:opacity-40"
                            >
                              <RotateCw size={13} /> Run again
                            </button>
                            <div className="my-1 border-t border-line" />
                            <button
                              onClick={() => void cancelRun(run)}
                              disabled={!active || !cancelable || busyRun === run.id}
                              className="flex w-full items-center gap-2 px-3 py-1.5 text-left text-danger hover:bg-raised disabled:opacity-40"
                            >
                              <XCircle size={13} /> Cancel run
                            </button>
                          </div>
                        </>
                      )}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
