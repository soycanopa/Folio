import { useMediaSrc } from "../lib/media-src";

// Miniatura de media con resolución por modo: en remoto resuelve async
// (raw URL o caché local del core) y muestra un placeholder mientras.
export function MediaImg({
  root,
  path,
  remote,
  alt,
  className,
}: {
  root: string;
  path: string;
  remote: boolean;
  alt: string;
  className?: string;
}) {
  const src = useMediaSrc(remote, root, path);
  if (!src) return <span aria-hidden className={className} />;
  return <img src={src} alt={alt} className={className} />;
}
