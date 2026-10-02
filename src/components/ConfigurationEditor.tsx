import { useEffect, useState } from "react";
import { api } from "../api";

interface ConfigurationEditorProps {
  raw: string;
  dirty: boolean;
  busy: boolean;
  onChange: (raw: string) => void;
  onSave: () => void;
}

/*
 * Portado de Pages CMS (https://github.com/hunvreus/pagescms) — MIT License.
 * Su página /configuration (app/(main)/[owner]/[repo]/[branch]/configuration)
 * es el Entry apuntando a `.pages.yml`; el lint es su `parseAndValidateConfig`
 * corriendo sobre el texto (aquí: validate_config del core, con debounce).
 * Solo edita el archivo existente: no hay creación de configuración.
 */
export function ConfigurationEditor({
  raw,
  dirty,
  busy,
  onChange,
  onSave,
}: ConfigurationEditorProps) {
  const [errors, setErrors] = useState<string[]>([]);

  useEffect(() => {
    const t = setTimeout(() => {
      api
        .validateConfig(raw)
        .then(() => setErrors([]))
        .catch((e) => setErrors([String(e)]));
    }, 300);
    return () => clearTimeout(t);
  }, [raw]);

  return (
    <div className="flex h-full min-h-0 flex-col gap-3 p-6">
      <div className="flex items-center justify-between gap-3">
        <div className="min-w-0">
          <h1 className="truncate text-lg font-semibold">Configuration</h1>
          <p className="truncate text-xs text-ink-dim">.pages.yml</p>
        </div>
        <button
          onClick={onSave}
          disabled={!dirty || busy}
          className="shrink-0 rounded-lg bg-primary px-4 py-1.5 text-sm font-medium text-primary-foreground disabled:opacity-40"
        >
          Save
        </button>
      </div>
      <textarea
        value={raw}
        onChange={(e) => onChange(e.target.value)}
        spellCheck={false}
        className="min-h-0 flex-1 resize-none rounded-lg border border-line bg-panel p-4 font-mono text-sm leading-relaxed outline-none focus:ring-1 focus:ring-ring"
      />
      {errors.length > 0 && (
        <div className="max-h-32 shrink-0 overflow-auto whitespace-pre-wrap rounded-lg border border-danger/40 bg-panel p-3 text-xs text-danger">
          {errors.join("\n")}
        </div>
      )}
    </div>
  );
}
