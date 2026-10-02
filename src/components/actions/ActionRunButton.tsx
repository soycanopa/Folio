import { useMemo, useState } from "react";
import { toast } from "sonner";
import { api } from "../../api";
import type {
  Action,
  ActionContext,
  ActionField,
  ActionRunInfo,
  Field,
} from "../../types";
import { FieldInput } from "../FieldInput";
import { Modal } from "../Dialogs";

/*
 * Portado de Pages CMS (https://github.com/hunvreus/pagescms) — MIT License.
 * Su RepoActionButtons: correr una action dispara el workflow por
 * workflow_dispatch; `confirm` (default true) o `fields` abren el diálogo
 * con su copy ("This will trigger a GitHub Action." / "Run action").
 * El toast sigue la corrida (su trackActionRun) hasta el conclusion.
 */

interface ActionRunButtonProps {
  action: Action;
  context: ActionContext;
  onDispatched?: () => void;
}

/// Su FieldInput de Folio no conoce los tipos de action field; el mapeo
/// es 1:1 con los nombres que ya soporta.
function toField(f: ActionField): Field {
  const type =
    f.field_type === "text"
      ? "string"
      : f.field_type === "textarea"
        ? "text"
        : f.field_type === "checkbox"
          ? "boolean"
          : f.field_type;
  return {
    name: f.name,
    label: f.label,
    type,
    required: f.required,
    values: f.values,
  };
}

function defaultValue(f: ActionField): unknown {
  if (f.default != null && f.default !== "") {
    if (f.field_type === "number") {
      const n = Number(f.default);
      if (!Number.isNaN(n)) return n;
    }
    if (f.field_type === "checkbox") return f.default === "true";
    return f.default;
  }
  return f.field_type === "checkbox" ? false : "";
}

/// Sus reglas de required (repo-action-buttons L112-128).
function missingRequired(f: ActionField, value: unknown): boolean {
  if (!f.required) return false;
  if (f.field_type === "checkbox") return value !== true;
  if (f.field_type === "number")
    return typeof value !== "number" || Number.isNaN(value);
  return typeof value !== "string" || value.trim().length === 0;
}

export function ActionRunButton({
  action,
  context,
  onDispatched,
}: ActionRunButtonProps) {
  const [open, setOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [values, setValues] = useState<Record<string, unknown>>(() =>
    Object.fromEntries(action.fields.map((f) => [f.name, defaultValue(f)])),
  );

  const hasFields = action.fields.length > 0;
  // Su lógica: confirm !== false o hay fields → diálogo; si no, corre directo.
  const needsDialog = action.confirm?.enabled !== false || hasFields;

  const invalidFields = useMemo(
    () => action.fields.filter((f) => missingRequired(f, values[f.name])),
    [action.fields, values],
  );

  const trackRun = (label: string, toastId: string | number, run: ActionRunInfo) => {
    const tick = async () => {
      try {
        const runs = await api.actionRuns([run.workflow], 20);
        const current = runs.find((r) => r.id === run.id);
        if (!current || current.status !== "completed") {
          setTimeout(() => void tick(), 4000);
          return;
        }
        toast.dismiss(toastId);
        if (current.conclusion === "success") {
          toast.success(`"${label}" succeeded.`);
        } else {
          toast.error(`"${label}" ${formatConclusion(current.conclusion)}.`);
        }
        onDispatched?.();
      } catch {
        // Su copy de fallo de refresh, sin ciclar para siempre.
        setTimeout(() => void tick(), 8000);
      }
    };
    void tick();
  };

  const dispatch = async (inputs: Record<string, unknown>) => {
    setBusy(true);
    const toastId = toast.loading(`Starting "${action.label}"…`);
    try {
      const run = await api.runAction(action, context, inputs);
      if (run) {
        if (run.status === "completed") {
          toast.dismiss(toastId);
          run.conclusion === "success"
            ? toast.success(`"${action.label}" succeeded.`)
            : toast.error(`"${action.label}" failed.`);
          onDispatched?.();
        } else {
          // El mismo toast de carga sigue la corrida hasta el conclusion
          // (su trackActionRun).
          trackRun(action.label, toastId, run);
        }
      } else {
        // Sin claim (workflow lento en aparecer): la tabla de corridas lo
        // toma en su próximo refresh.
        toast.dismiss(toastId);
        toast(`Dispatched "${action.label}" — it will appear in Actions.`);
        onDispatched?.();
      }
      setOpen(false);
    } catch (e) {
      toast.dismiss(toastId);
      toast.error(String(e));
    } finally {
      setBusy(false);
    }
  };

  const handleClick = () => {
    if (needsDialog) {
      setOpen(true);
    } else {
      void dispatch({});
    }
  };

  const confirm = action.confirm;
  const title = confirm?.title ?? action.label;
  const message = confirm?.message ?? "This will trigger a GitHub Action.";
  const submitLabel = confirm?.button ?? action.label ?? "Run action";

  return (
    <>
      <button
        onClick={handleClick}
        disabled={busy}
        className="shrink-0 rounded-lg border border-line px-3 py-1.5 text-sm hover:bg-panel disabled:opacity-40"
      >
        {action.label}
      </button>
      {open && (
        <Modal title={title} onClose={() => !busy && setOpen(false)}>
          <form
            onSubmit={(e) => {
              e.preventDefault();
              void dispatch(values);
            }}
            className="flex flex-col gap-3"
          >
            <p className="text-sm text-ink-dim">{message}</p>
            {action.fields.map((f) => (
              <label key={f.name} className="flex flex-col gap-1.5">
                <span className="text-xs font-medium">
                  {f.label}
                  {f.required ? " *" : ""}
                </span>
                <FieldInput
                  field={toField(f)}
                  value={values[f.name]}
                  onChange={(v) =>
                    setValues((prev) => ({ ...prev, [f.name]: v }))
                  }
                />
              </label>
            ))}
            <div className="mt-1 flex items-center justify-end gap-2">
              <button
                type="button"
                onClick={() => setOpen(false)}
                className="rounded-lg px-3.5 py-1.5 text-sm text-ink-dim hover:text-ink"
              >
                Cancel
              </button>
              <button
                type="submit"
                disabled={busy || invalidFields.length > 0}
                className="rounded-lg bg-primary px-3.5 py-1.5 text-sm font-medium text-primary-foreground disabled:opacity-40"
              >
                {submitLabel}
              </button>
            </div>
          </form>
        </Modal>
      )}
    </>
  );
}

function formatConclusion(conclusion?: string | null): string {
  switch (conclusion) {
    case "cancelled":
      return "was cancelled";
    case "timed_out":
      return "timed out";
    case "skipped":
      return "was skipped";
    default:
      return "failed";
  }
}
