import { FileText, Image as ImageIcon, X } from "lucide-react";
import type { Field } from "../types";

interface FieldInputProps {
  field: Field;
  value: unknown;
  onChange: (value: unknown) => void;
  /** URL del asset protocol para previsualizar el valor actual. */
  previewSrc?: string;
  /** Abrir el media picker para un campo image | file. */
  onPickMedia?: (fieldName: string) => void;
}

const SUPPORTED = [
  "string",
  "text",
  "date",
  "image",
  "file",
  "rich-text",
  "number",
  "boolean",
  "select",
  "code",
];

// El control de un campo del config. El rich-text vive aparte, en la
// tarjeta del cuerpo (EntryEditor), porque su valor es el documento.
export function FieldInput({
  field,
  value,
  onChange,
  previewSrc,
  onPickMedia,
}: FieldInputProps) {
  if (!SUPPORTED.includes(field.type)) {
    return (
      <input
        disabled
        placeholder={`${field.type} — not supported`}
        title={`${field.type} — not supported`}
        className="w-full cursor-not-allowed rounded-lg bg-panel px-3 py-2 text-sm text-ink-dim outline-none opacity-60"
      />
    );
  }

  const str = value == null ? "" : String(value);

  const inputCls =
    "w-full rounded-lg bg-panel px-3 py-2 text-sm outline-none placeholder:text-ink-dim focus:ring-1 focus:ring-ring";

  if (field.type === "boolean") {
    return (
      <button
        type="button"
        role="switch"
        aria-checked={Boolean(value)}
        onClick={() => onChange(!value)}
        className={`relative h-5 w-9 rounded-full transition-colors ${
          value ? "bg-primary" : "bg-raised"
        }`}
      >
        <span
          className={`absolute top-0.5 h-4 w-4 rounded-full bg-white transition-transform ${
            value ? "translate-x-[18px]" : "translate-x-0.5"
          }`}
        />
      </button>
    );
  }

  if (field.type === "select") {
    return (
      <select
        value={str}
        onChange={(e) => onChange(e.currentTarget.value)}
        className={`${inputCls} appearance-none`}
      >
        <option value="">—</option>
        {(field.values ?? []).map((v) => (
          <option key={v} value={v}>
            {v}
          </option>
        ))}
      </select>
    );
  }

  if (field.type === "number") {
    return (
      <input
        type="number"
        value={str}
        onChange={(e) =>
          onChange(
            e.currentTarget.value === "" ? null : e.currentTarget.valueAsNumber,
          )
        }
        className={inputCls}
      />
    );
  }

  if (field.type === "code") {
    return (
      <textarea
        rows={6}
        value={str}
        onChange={(e) => onChange(e.currentTarget.value)}
        spellCheck={false}
        className={`${inputCls} resize-y font-mono`}
      />
    );
  }

  if (field.type === "image" || field.type === "file") {
    const isImg = field.type === "image";
    return (
      <div className="flex items-center gap-3">
        {isImg && previewSrc ? (
          <img
            src={previewSrc}
            alt=""
            className="h-12 w-12 shrink-0 rounded-md border border-line object-cover"
          />
        ) : (
          <span className="flex h-12 w-12 shrink-0 items-center justify-center rounded-md border border-dashed border-line text-ink-dim">
            {isImg ? <ImageIcon size={16} /> : <FileText size={16} />}
          </span>
        )}
        <span className="min-w-0 flex-1 truncate text-sm text-ink-dim">
          {str || `No ${field.type}`}
        </span>
        <button
          onClick={() => onPickMedia?.(field.name)}
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
        className={`${inputCls} resize-y`}
      />
    );
  }

  return (
    <input
      type={field.type === "date" ? "date" : "text"}
      value={str}
      onChange={(e) => onChange(e.currentTarget.value)}
      placeholder={field.type === "image" ? "/images/…" : ""}
      className={inputCls}
    />
  );
}
