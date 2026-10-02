import type { Field } from "../types";

interface FieldInputProps {
  field: Field;
  value: unknown;
  onChange: (value: unknown) => void;
}

const KNOWN = ["string", "text", "date", "image"];

// El control de un campo del config. El rich-text vive aparte, en la
// tarjeta del cuerpo (EntryEditor), porque su valor es el documento.
export function FieldInput({ field, value, onChange }: FieldInputProps) {
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

  if (field.type === "text") {
    return (
      <textarea
        rows={3}
        value={str}
        onChange={(e) => onChange(e.currentTarget.value)}
        className="w-full resize-y rounded-lg bg-panel px-3 py-2 text-sm outline-none placeholder:text-ink-dim focus:ring-1 focus:ring-accent"
      />
    );
  }

  return (
    <input
      type={field.type === "date" ? "date" : "text"}
      value={str}
      onChange={(e) => onChange(e.currentTarget.value)}
      placeholder={field.type === "image" ? "/images/…" : ""}
      className="w-full rounded-lg bg-panel px-3 py-2 text-sm outline-none placeholder:text-ink-dim focus:ring-1 focus:ring-accent"
    />
  );
}
