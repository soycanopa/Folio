import { useEffect, useRef, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { Maximize, Minus, MoreHorizontal, Plus } from "lucide-react";
import type { ContentItem } from "../types";
import { resolveMediaSrc } from "../lib/media-src";

// Esquema mínimo del TRD.md. El id referencia el slug de la ficha;
// Folio no interpreta el lienzo al construir el sitio.
export interface CanvasNode {
  id: string;
  x: number;
  y: number;
  w: number;
}

export interface CanvasLayout {
  viewport: { x: number; y: number; zoom: number };
  nodes: CanvasNode[];
}

export const DEFAULT_LAYOUT: CanvasLayout = {
  viewport: { x: 0, y: 0, zoom: 1 },
  nodes: [],
};

export function slugOf(path: string): string {
  const base = path.split("/").pop() ?? path;
  return base.replace(/\.[^.]+$/, "");
}

interface CanvasProps {
  collection: ContentItem;
  root: string;
  mediaInput: string | null;
  cards: { path: string; values: Record<string, unknown> }[];
  layout: CanvasLayout;
  dirty: boolean;
  /** Remoto: las previews se resuelven contra la API, no el disco. */
  remote?: boolean;
  /** Save en curso: el botón se bloquea (su isBusy). */
  busy?: boolean;
  onChange: (l: CanvasLayout) => void;
  onOpenEntry: (path: string) => void;
  onSave: () => void;
  /** Solo local: en remoto cada save publica su commit en la rama. */
  onCommit?: () => void;
  onPush?: () => void;
}

const MIN_ZOOM = 0.3;
const MAX_ZOOM = 2.5;
const clamp = (v: number, min: number, max: number) =>
  Math.min(max, Math.max(min, v));

export function Canvas({
  collection,
  root,
  mediaInput,
  cards,
  layout,
  dirty,
  remote,
  busy,
  onChange,
  onOpenEntry,
  onSave,
  onCommit,
  onPush,
}: CanvasProps) {
  const surfaceRef = useRef<HTMLDivElement>(null);
  const [menuOpen, setMenuOpen] = useState(false);
  const drag = useRef<null | {
    mode: "node" | "pan";
    id?: string;
    startX: number;
    startY: number;
    origX: number;
    origY: number;
    moved: boolean;
  }>(null);

  const v = layout.viewport;

  // La rueda con ctrl/cmd acerca; sin modificador, panea. Listener
  // directo para poder usar passive: false.
  useEffect(() => {
    const el = surfaceRef.current;
    if (!el) return;
    const onWheel = (e: WheelEvent) => {
      e.preventDefault();
      if (e.ctrlKey || e.metaKey) {
        const rect = el.getBoundingClientRect();
        const cx = e.clientX - rect.left;
        const cy = e.clientY - rect.top;
        const zoom2 = clamp(v.zoom * Math.exp(-e.deltaY * 0.0015), MIN_ZOOM, MAX_ZOOM);
        onChange({
          ...layout,
          viewport: {
            zoom: zoom2,
            x: cx - ((cx - v.x) * zoom2) / v.zoom,
            y: cy - ((cy - v.y) * zoom2) / v.zoom,
          },
        });
      } else {
        onChange({
          ...layout,
          viewport: { ...v, x: v.x - e.deltaX, y: v.y - e.deltaY },
        });
      }
    };
    el.addEventListener("wheel", onWheel, { passive: false });
    return () => el.removeEventListener("wheel", onWheel);
  }, [layout, onChange]);

  function nodeById(slug: string, i: number): CanvasNode {
    return (
      layout.nodes.find((n) => n.id === slug) ?? {
        id: slug,
        x: 60 + (i % 4) * 260,
        y: 60 + Math.floor(i / 4) * 240,
        w: 220,
      }
    );
  }

  function pointerDownNode(e: React.PointerEvent, node: CanvasNode) {
    e.stopPropagation();
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    drag.current = {
      mode: "node",
      id: node.id,
      startX: e.clientX,
      startY: e.clientY,
      origX: node.x,
      origY: node.y,
      moved: false,
    };
  }

  function pointerDownBg(e: React.PointerEvent) {
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    drag.current = {
      mode: "pan",
      startX: e.clientX,
      startY: e.clientY,
      origX: v.x,
      origY: v.y,
      moved: false,
    };
  }

  function pointerMove(e: React.PointerEvent) {
    const d = drag.current;
    if (!d) return;
    const dx = e.clientX - d.startX;
    const dy = e.clientY - d.startY;
    if (Math.abs(dx) + Math.abs(dy) > 3) d.moved = true;
    if (!d.moved) return;
    if (d.mode === "pan") {
      onChange({
        ...layout,
        viewport: { ...v, x: d.origX + dx, y: d.origY + dy },
      });
    } else {
      onChange({
        ...layout,
        nodes: layout.nodes.map((n) =>
          n.id === d.id
            ? { ...n, x: d.origX + dx / v.zoom, y: d.origY + dy / v.zoom }
            : n,
        ),
      });
    }
  }

  function pointerUp() {
    const d = drag.current;
    drag.current = null;
    if (d?.mode === "node" && !d.moved && d.id) {
      const card = cards.find((c) => slugOf(c.path) === d.id);
      if (card) onOpenEntry(card.path);
    }
  }

  function zoomBy(factor: number) {
    const el = surfaceRef.current;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    const cx = rect.width / 2;
    const cy = rect.height / 2;
    const zoom2 = clamp(v.zoom * factor, MIN_ZOOM, MAX_ZOOM);
    onChange({
      ...layout,
      viewport: {
        zoom: zoom2,
        x: cx - ((cx - v.x) * zoom2) / v.zoom,
        y: cy - ((cy - v.y) * zoom2) / v.zoom,
      },
    });
  }

  const imageField =
    collection.fields.find((f) => f.type === "image")?.name ?? null;

  // Remoto: las previews de las fichas se resuelven contra la API y van
  // llegando al mapa (raw URL o caché del core).
  const cardImageNames = imageField
    ? cards
        .map((c) => String(c.values[imageField] ?? "").split("/").pop())
        .filter(Boolean)
        .join(",")
    : "";
  const [thumbs, setThumbs] = useState<Record<string, string>>({});
  useEffect(() => {
    if (!remote || !mediaInput || !cardImageNames) return;
    let alive = true;
    void (async () => {
      const next: Record<string, string> = {};
      for (const n of cardImageNames.split(",")) {
        try {
          next[n] = await resolveMediaSrc(true, root, `${mediaInput}/${n}`);
        } catch {
          // sin preview para ese nombre
        }
      }
      if (alive && Object.keys(next).length > 0) {
        setThumbs((t) => ({ ...t, ...next }));
      }
    })();
    return () => {
      alive = false;
    };
  }, [remote, root, mediaInput, cardImageNames]);

  const previewSrc = (values: Record<string, unknown>): string | undefined => {
    if (!imageField || !mediaInput) return undefined;
    const val = values[imageField];
    if (typeof val !== "string" || !val) return undefined;
    const name = val.split("/").pop();
    if (!name) return undefined;
    if (remote) return thumbs[name];
    return convertFileSrc(`${root}/${mediaInput}/${name}`);
  };

  return (
    <div className="flex h-full flex-col">
      <header
          data-tauri-drag-region
          className="flex items-center gap-2 px-6 py-4">
        <h1 className="text-xl font-semibold">Canvas</h1>
        <span className="text-sm text-ink-dim">· {collection.label}</span>
        <div className="ml-auto flex items-center gap-1.5">
          <div className="relative">
            <button
              onClick={() => setMenuOpen((v) => !v)}
              className="rounded-lg p-2 hover:bg-panel"
              title="More"
            >
              <MoreHorizontal size={16} />
            </button>
            {menuOpen && (
              <>
                <div
                  className="fixed inset-0 z-10"
                  onClick={() => setMenuOpen(false)}
                />
                <div className="absolute right-0 z-20 mt-1 w-44 overflow-hidden rounded-lg border border-line bg-panel py-1 text-sm">
                  {onCommit && (
                    <button
                      onClick={() => {
                        setMenuOpen(false);
                        onCommit();
                      }}
                      className="block w-full px-3 py-1.5 text-left hover:bg-raised"
                    >
                      Commit…
                    </button>
                  )}
                  {onPush && (
                    <button
                      onClick={() => {
                        setMenuOpen(false);
                        onPush();
                      }}
                      className="block w-full px-3 py-1.5 text-left hover:bg-raised"
                    >
                      Push
                    </button>
                  )}
                </div>
              </>
            )}
          </div>
          <button
            onClick={onSave}
            disabled={!dirty || busy}
            className="rounded-lg bg-primary px-4 py-1.5 text-sm font-medium text-primary-foreground disabled:opacity-40"
          >
            Save
          </button>
        </div>
      </header>

      <div
        ref={surfaceRef}
        onPointerDown={pointerDownBg}
        onPointerMove={pointerMove}
        onPointerUp={pointerUp}
        className="relative flex-1 cursor-grab overflow-hidden bg-canvas active:cursor-grabbing"
      >
        <div
          className="absolute left-0 top-0 origin-top-left"
          style={{
            transform: `translate(${v.x}px, ${v.y}px) scale(${v.zoom})`,
          }}
        >
          {cards.map((card, i) => {
            const node = nodeById(slugOf(card.path), i);
            const img = previewSrc(card.values);
            return (
              <div
                key={card.path}
                className="absolute select-none"
                style={{ left: node.x, top: node.y, width: node.w }}
                onPointerDown={(e) => pointerDownNode(e, node)}
              >
                <div className="cursor-grab overflow-hidden rounded-xl border border-line bg-panel shadow-lg active:cursor-grabbing">
                  {img ? (
                    <img
                      src={img}
                      alt=""
                      draggable={false}
                      className="h-28 w-full bg-raised object-cover"
                    />
                  ) : (
                    <div className="h-10 bg-raised" />
                  )}
                  <div className="truncate px-3 py-2.5 text-sm font-medium">
                    {String(card.values.title ?? slugOf(card.path))}
                  </div>
                </div>
              </div>
            );
          })}
        </div>

        <div className="absolute bottom-4 right-4 flex items-center gap-1 rounded-lg border border-line bg-panel p-1 shadow-xl">
          <button
            onClick={() => zoomBy(1 / 1.2)}
            className="rounded-md p-1.5 hover:bg-raised"
            title="Zoom out"
          >
            <Minus size={14} />
          </button>
          <span className="w-12 text-center text-xs text-ink-dim">
            {Math.round(v.zoom * 100)}%
          </span>
          <button
            onClick={() => zoomBy(1.2)}
            className="rounded-md p-1.5 hover:bg-raised"
            title="Zoom in"
          >
            <Plus size={14} />
          </button>
          <button
            onClick={() =>
              onChange({ ...layout, viewport: { x: 0, y: 0, zoom: 1 } })
            }
            className="rounded-md p-1.5 hover:bg-raised"
            title="Reset view"
          >
            <Maximize size={14} />
          </button>
        </div>
      </div>
    </div>
  );
}
