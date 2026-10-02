import { Image as ImageIcon, X } from "lucide-react";
import type { Field } from "../types";

interface FieldInputProps {
  field: Field;
  value: unknown;
  onChange: (value: unknown) => void;
  /** URL del asset protocol para previsualizar el valor actual. */
  previewSrc?: string;
  onPickImage?: (fieldName: string) => void;
}

const KNOWN = ["string", "text", "date", "image"];

// El control de un campo del config. El rich-text vive aparte, en la
// tarjeta del cuerpo (EntryEditor), porque su valor es el documento.
export function FieldInput({
  field,
  value,
  onChange,
  previewSrc,
  onPickImage,
}: FieldInputProps) {
  if (!KNOWN.includes(field.type)) {
    return (
      <input
        disabled
        placeholder={`${field.type} — not supported in v0`}
        title={`${field.type} — not supported in v0`}
        className="w-full cursor-not-allowed rounded-lg bg-panel px-3 py-2 text-sm text-ink-dim outline-none opacity-60"
      />
    );
  }

  const str = value == null ? "" : String(value);

  if (field.type === "image") {
    // El image abre el media picker, no es un path suelto (UI.md).
    return (
      <div className="flex items-center gap-3">
        {previewSrc ? (
          <img
            src={previewSrc}
            alt=""
            className="h-12 w-12 shrink-0 rounded-md border border-line object-cover"
          />
        ) : (
          <span className="flex h-12 w-12 shrink-0 items-center justify-center rounded-md border border-dashed border-line text-ink-dim">
            <ImageIcon size={16} />
          </span>
        )}
        <span className="min-w-0 flex-1 truncate text-sm text-ink-dim">
          {str || "No image"}
        </span>
        <button
          onClick={() => onPickImage?.(field.name)}
          className="shrink-0 rounded-lg border border-line px-3 py-1.5 text-sm hover:bg-panel"
        >
          Choose…
        </button>
        {str && (
          <button
            onClick={() => onChange("")}
            title="Remove"
            className="shrink-0 rounded-md p-1 text-ink-dim hover:text-ink"
          >
            <X size={14} />
          </button>
        )}
      </div>
    );
  }

  if (field.type === "text") {
    return (
      <textarea
        rows={3}
        value={str}
        onChange={(e) => onChange(e.currentTarget.value)}
        className="w-full resize-y rounded-lg bg-panel px-3 py-2 text-sm outline-none placeholder:text-ink-dim focus:ring-1 focus:ring-ring"
      />
    );
  }

  return (
    <input
      type={field.type === "date" ? "date" : "text"}
      value={str}
      onChange={(e) => onChange(e.currentTarget.value)}
      placeholder={field.type === "image" ? "/images/…" : ""}
      className="w-full rounded-lg bg-panel px-3 py-2 text-sm outline-none placeholder:text-ink-dim focus:ring-1 focus:ring-ring"
    />
  );
}
