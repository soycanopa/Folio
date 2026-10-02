import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { Toaster } from "sonner";
import "./index.css";

// Un crash de React no puede ser pantalla negra: el error queda
// visible y copiable, dentro de la piel de la app.
class ErrorBoundary extends React.Component<
  { children: React.ReactNode },
  { error: Error | null }
> {
  state = { error: null as Error | null };

  static getDerivedStateFromError(error: Error) {
    return { error };
  }

  render() {
    if (this.state.error) {
      return (
        <div className="flex h-full flex-col gap-3 overflow-auto p-8">
          <h1 className="text-lg font-semibold text-danger">
            Something broke
          </h1>
          <pre className="whitespace-pre-wrap rounded-lg bg-panel p-4 text-xs text-ink-dim">
            {this.state.error.stack ?? String(this.state.error)}
          </pre>
          <button
            onClick={() => this.setState({ error: null })}
            className="self-start rounded-lg bg-primary px-4 py-2 text-sm font-medium text-primary-foreground"
          >
            Reload view
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}

// Errores fuera del árbol (promesas, listeners) también a la vista.
function showGlobalError(where: string, detail: unknown) {
  const el = document.createElement("pre");
  el.style.cssText =
    "position:fixed;inset:auto 0 0 0;z-index:9999;max-height:40vh;overflow:auto;" +
    "background:#3b0d0d;color:#ffb4b4;padding:12px;font-size:11px;white-space:pre-wrap";
  el.textContent = `[${where}] ${detail instanceof Error ? (detail.stack ?? String(detail)) : String(detail)}`;
  document.body.appendChild(el);
}

window.addEventListener("error", (e) => showGlobalError("error", e.error ?? e.message));
window.addEventListener("unhandledrejection", (e) => showGlobalError("promise", e.reason));

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ErrorBoundary>
      <App />
      <Toaster theme="dark" position="bottom-right" />
    </ErrorBoundary>
  </React.StrictMode>,
);
