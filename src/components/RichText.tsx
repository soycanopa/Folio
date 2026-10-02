import { useCallback, useRef, useState } from "react";
import { Editor, type ImagePickerResult } from "./editor";
import type { MediaRef } from "../types";
import { MediaPickerDialog } from "./Media";

interface RichTextProps {
  value: string;
  onChange: (markdown: string) => void;
  sourceMode: boolean;
  /** Media para el slash-command de imagen (opcional). */
  media?: {
    root: string;
    mediaInput: string;
    items: MediaRef[];
    onUpload: () => Promise<MediaRef | null>;
  };
}

// El campo rich-text de Folio envuelve el editor portado de Pages CMS
// con format="markdown": emite y consume Markdown, como su Editor toggle.
export function RichText({ value, onChange, sourceMode, media }: RichTextProps) {
  const pickerResolve = useRef<((r: ImagePickerResult | null) => void) | null>(
    null,
  );
  const [pickerOpen, setPickerOpen] = useState(false);

  // El editor pide una imagen de forma imperativa; Folio responde con
  // su media picker. Promesa resuelta al elegir o al cerrar.
  const onRequestImage = useCallback(async () => {
    return new Promise<ImagePickerResult | null>((resolve) => {
      pickerResolve.current = resolve;
      setPickerOpen(true);
    });
  }, []);

  const settlePicker = (result: ImagePickerResult | null) => {
    setPickerOpen(false);
    pickerResolve.current?.(result);
    pickerResolve.current = null;
  };

  if (sourceMode) {
    return (
      <textarea
        value={value}
        onChange={(e) => onChange(e.currentTarget.value)}
        spellCheck={false}
        className="min-h-[240px] w-full resize-y rounded-lg bg-panel p-4 font-mono text-sm outline-none focus:ring-1 focus:ring-ring"
      />
    );
  }

  return (
    <div className="overflow-hidden rounded-lg border border-line bg-panel focus-within:ring-1 focus-within:ring-ring">
      <Editor
        value={value}
        onChange={onChange}
        format="markdown"
        className="folio-editor"
        editorClassName="border-0 rounded-none min-h-[240px] bg-transparent px-4 py-3 text-base focus-visible:ring-0 shadow-none"
        enableImages={Boolean(media)}
        imageFallback="none"
        onRequestImage={media ? onRequestImage : undefined}
      />
      {media && pickerOpen && (
        <MediaPickerDialog
          root={media.root}
          items={media.items}
          onUpload={() => media.onUpload()}
          onPick={(m: MediaRef) =>
            settlePicker({ kind: "url", src: m.public_path, alt: m.name })
          }
          onClose={() => settlePicker(null)}
        />
      )}
    </div>
  );
}
