import { useMemo, useState } from "react";
import { Plus, Search } from "lucide-react";
import type { ContentItem } from "../types";

export interface EntryRow {
  path: string;
  values: Record<string, unknown>;
}

interface CollectionTableProps {
  collection: ContentItem;
  rows: EntryRow[];
  onOpen: (path: string) => void;
  onNew: () => void;
}

const SIMPLE_TYPES = ["string", "date"];

function columnsFor(collection: ContentItem): string[] {
  if (collection.view?.fields?.length) return collection.view.fields;
  return collection.fields
    .filter((f) => SIMPLE_TYPES.includes(f.type))
    .map((f) => f.name);
}

export function CollectionTable({
  collection,
  rows,
  onOpen,
  onNew,
}: CollectionTableProps) {
  const [query, setQuery] = useState("");

  const columns = useMemo(() => columnsFor(collection), [collection]);

  const sorted = useMemo(() => {
    const sort = collection.view?.sort;
    const order = collection.view?.order ?? "asc";
    const list = rows.filter((r) =>
      JSON.stringify(r.values).toLowerCase().includes(query.toLowerCase()),
    );
    if (!sort) return list;
    return [...list].sort((a, b) => {
      const av = String(a.values[sort] ?? "");
      const bv = String(b.values[sort] ?? "");
      const cmp = av < bv ? -1 : av > bv ? 1 : 0;
      return order === "desc" ? -cmp : cmp;
    });
  }, [rows, query, collection]);

  const label = (col: string) =>
    collection.fields.find((f) => f.name === col)?.label ?? col;

  return (
    <div className="flex h-full flex-col">
      <header
        data-tauri-drag-region
        className="flex items-center gap-3 px-6 pt-5 pb-4">
        <h1 className="text-xl font-semibold">{collection.label}</h1>
        <div className="relative ml-auto w-64">
          <Search
            size={14}
            className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-ink-dim"
          />
          <input
            value={query}
            onChange={(e) => setQuery(e.currentTarget.value)}
            placeholder="Search…"
            className="w-full rounded-lg bg-panel py-1.5 pl-8 pr-3 text-sm outline-none placeholder:text-ink-dim focus:ring-1 focus:ring-ring"
          />
        </div>
        <button
          onClick={onNew}
          className="flex items-center gap-1.5 rounded-lg border border-line px-3 py-1.5 text-sm hover:bg-panel"
        >
          <Plus size={14} /> New
        </button>
      </header>

      <div className="flex-1 overflow-y-auto px-6">
        <table className="w-full text-sm">
          <thead>
            <tr className="text-left text-[11px] uppercase tracking-wider text-ink-dim">
              {columns.map((col) => (
                <th key={col} className="border-b border-line px-3 py-2 font-medium">
                  {label(col)}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {sorted.map((row) => (
              <tr
                key={row.path}
                onClick={() => onOpen(row.path)}
                className="cursor-pointer hover:bg-panel/60"
              >
                {columns.map((col) => (
                  <td
                    key={col}
                    className="max-w-0 truncate border-b border-line/50 px-3 py-2.5"
                  >
                    {String(
                      row.values[col] ??
                        (col === "title" ? basename(row.path) : ""),
                    )}
                  </td>
                ))}
              </tr>
            ))}
            {sorted.length === 0 && (
              <tr>
                <td
                  colSpan={columns.length || 1}
                  className="px-3 py-8 text-center text-ink-dim"
                >
                  No entries yet.
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}

function basename(path: string): string {
  const base = path.split("/").pop() ?? path;
  return base.replace(/\.md$/, "");
}
