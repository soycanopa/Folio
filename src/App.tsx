import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

// Spike Fase 0: cableado de los comandos del core, sin la piel de
// Pages CMS (esa llega en Fase 1 con el layout de DESIGN.md).
type RepoSummary = { root: string; branch: string };
type EntryRef = { path: string };
type EntryContent = { frontmatter: Record<string, unknown>; body: string };

const COLLECTION = "src/content/blog";

function App() {
  const [repoPath, setRepoPath] = useState("");
  const [summary, setSummary] = useState<RepoSummary | null>(null);
  const [entries, setEntries] = useState<EntryRef[]>([]);
  const [entryPath, setEntryPath] = useState<string | null>(null);
  const [frontmatter, setFrontmatter] = useState<Record<string, unknown>>({});
  const [title, setTitle] = useState("");
  const [body, setBody] = useState("");
  const [message, setMessage] = useState("");
  const [status, setStatus] = useState("");

  async function openRepo() {
    try {
      const s = await invoke<RepoSummary>("open_repo", { path: repoPath });
      setSummary(s);
      const list = await invoke<EntryRef[]>("list_entries", {
        collection: COLLECTION,
      });
      setEntries(list);
      setEntryPath(null);
      setStatus(`${s.root} @ ${s.branch} — ${list.length} entradas`);
    } catch (e) {
      setStatus(String(e));
    }
  }

  async function openEntry(path: string) {
    try {
      const entry = await invoke<EntryContent>("read_entry", { path });
      setEntryPath(path);
      setFrontmatter(entry.frontmatter);
      setTitle(String(entry.frontmatter.title ?? ""));
      setBody(entry.body);
      setStatus(path);
    } catch (e) {
      setStatus(String(e));
    }
  }

  async function saveEntry() {
    if (!entryPath) return;
    try {
      // Se reenvía el front matter completo: los campos que Folio no
      // describe (p.ej. `custom`) llegan de vuelta intactos al core.
      await invoke<string>("write_entry", {
        path: entryPath,
        frontmatter: { ...frontmatter, title },
        body,
      });
      setStatus(`guardado: ${entryPath}`);
    } catch (e) {
      setStatus(String(e));
    }
  }

  async function commit() {
    try {
      const oid = await invoke<string>("commit", { message });
      setStatus(`commit ${oid}`);
    } catch (e) {
      setStatus(String(e));
    }
  }

  return (
    <main className="spike">
      <h1>Folio — spike Fase 0</h1>

      <section>
        <input
          placeholder="/ruta/al/repo (p.ej. fixtures/blog ya con git init)"
          value={repoPath}
          onChange={(e) => setRepoPath(e.currentTarget.value)}
        />
        <button onClick={openRepo}>Open</button>
      </section>

      {summary && (
        <section>
          <ul>
            {entries.map((e) => (
              <li key={e.path}>
                <button onClick={() => openEntry(e.path)}>{e.path}</button>
              </li>
            ))}
          </ul>
        </section>
      )}

      {entryPath && (
        <section>
          <label>
            Title
            <input
              value={title}
              onChange={(e) => setTitle(e.currentTarget.value)}
            />
          </label>
          <label>
            Body
            <textarea
              rows={10}
              value={body}
              onChange={(e) => setBody(e.currentTarget.value)}
            />
          </label>
          <button onClick={saveEntry}>Save</button>
        </section>
      )}

      <section>
        <input
          placeholder="commit message"
          value={message}
          onChange={(e) => setMessage(e.currentTarget.value)}
        />
        <button onClick={commit}>Commit</button>
      </section>

      <p className="status">{status}</p>
    </main>
  );
}

export default App;
