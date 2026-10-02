// Resolución del src de preview de media según el modo del proyecto.
// Local → asset protocol del repo abierto. Remoto → raw URL (repo
// público) o archivo cacheado por el core servido por el asset
// protocol (privado; el token nunca llega al webview).
import { useEffect, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { api } from "../api";

// Caché en memoria: una vez resuelto un path, no se re-invoca por render.
const resolved = new Map<string, string>();

export async function resolveMediaSrc(
  remote: boolean,
  root: string,
  path: string,
): Promise<string> {
  if (!remote) return convertFileSrc(`${root}/${path}`);
  const cached = resolved.get(path);
  if (cached) return cached;
  const src = await api.remoteMediaUrl(path);
  const url = src.is_asset ? convertFileSrc(src.url) : src.url;
  resolved.set(path, url);
  return url;
}

/** Hook para <img>: resuelve async en remoto y re-renderiza al llegar. */
export function useMediaSrc(
  remote: boolean,
  root: string,
  path: string,
): string {
  const [url, setUrl] = useState<string>(() =>
    remote ? (resolved.get(path) ?? "") : convertFileSrc(`${root}/${path}`),
  );

  useEffect(() => {
    if (!remote) return;
    let alive = true;
    void resolveMediaSrc(true, root, path)
      .then((u) => {
        if (alive) setUrl(u);
      })
      .catch(() => undefined);
    return () => {
      alive = false;
    };
    // url queda fuera a propósito: solo dispara al cambiar el path.
  }, [remote, root, path]);

  return remote ? url : convertFileSrc(`${root}/${path}`);
}
