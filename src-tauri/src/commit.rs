use crate::state::{AppState, RepoState};

#[tauri::command]
pub fn commit(state: tauri::State<'_, AppState>, message: &str) -> Result<String, String> {
    let mut guard = state.lock().unwrap();
    let st = guard.as_mut().ok_or("no hay repo abierto")?;
    commit_touched(st, message)
}

/// Commit que stagea solo los paths que Folio tocó en la sesión
/// (`RepoState::touched`). Prohibido `add -A`: lo demás del working tree
/// se queda afuera.
pub fn commit_touched(st: &mut RepoState, message: &str) -> Result<String, String> {
    let message = message.trim();
    if message.is_empty() {
        return Err("mensaje de commit vacío".to_string());
    }
    if st.touched.is_empty() {
        return Err("nada que commitear: Folio no tocó archivos".to_string());
    }

    let repo = &st.repo;
    let head_commit: Option<git2::Commit> =
        repo.head().ok().and_then(|r| r.peel_to_commit().ok());
    let head_tree = head_commit
        .as_ref()
        .map(|c| c.tree())
        .transpose()
        .map_err(|e| format!("leer tree de HEAD: {e}"))?;

    // Invariante: el commit no se lleva cambios ajenos. Si el index ya
    // tiene algo stageado fuera de lo que Folio tocó (p.ej. un `git add`
    // manual), se rechaza en vez de incluirlo. Deltas sobre paths propios
    // se toleran: es el reintento tras un commit que falló a mitad.
    let diff = repo
        .diff_tree_to_index(head_tree.as_ref(), None, None)
        .map_err(|e| format!("comparar index con HEAD: {e}"))?;
    let foreign = diff.deltas().any(|d| {
        let path = d.new_file().path().or_else(|| d.old_file().path());
        !matches!(path, Some(p) if st.touched.iter().any(|t| t == p))
    });
    if foreign {
        return Err(
            "el index ya tiene cambios stageados ajenos a Folio; commitealos aparte"
                .to_string(),
        );
    }

    let mut index = repo.index().map_err(|e| format!("abrir index: {e}"))?;
    for rel in &st.touched {
        // Si el archivo ya no está en disco fue un borrado de Folio:
        // se stagea como eliminación (delete_entry).
        if st.root.join(rel).exists() {
            index
                .add_path(rel)
                .map_err(|e| format!("stagear {}: {e}", rel.display()))?;
        } else {
            index
                .remove_path(rel)
                .map_err(|e| format!("stagear borrado de {}: {e}", rel.display()))?;
        }
    }
    let tree_oid = index
        .write_tree()
        .map_err(|e| format!("write_tree: {e}"))?;
    // Persistir el index antes del commit, como hace `git add`: si el
    // commit falla, lo stageado son paths propios y el reintento pasa.
    index.write().map_err(|e| format!("index.write: {e}"))?;
    let tree = repo
        .find_tree(tree_oid)
        .map_err(|e| format!("find_tree: {e}"))?;

    let sig = repo
        .signature()
        .or_else(|_| git2::Signature::now("Folio", "folio@localhost"))
        .map_err(|e| format!("firma de git: {e}"))?;
    let parents: Vec<&git2::Commit> = head_commit.iter().collect();
    let oid = repo
        .commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)
        .map_err(|e| format!("commit: {e}"))?;

    st.touched.clear();
    Ok(oid.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::write_entry_tracked;
    use serde_yaml_ng::Value;
    use std::fs;
    use std::path::{Path, PathBuf};

    fn setup_repo() -> (tempfile::TempDir, RepoState) {
        let dir = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        let root = dir.path().to_path_buf();

        let blog = root.join("src/content/blog");
        fs::create_dir_all(&blog).unwrap();
        fs::write(
            blog.join("hello-world.md"),
            "---\ntitle: Hello world\npubDate: 2026-09-28\n---\n\nHello body.\n",
        )
        .unwrap();
        fs::write(root.join("README.md"), "no relacionado\n").unwrap();

        // Commit inicial del fixture, index escrito como lo haría `git add`.
        {
            let mut index = repo.index().unwrap();
            index.add_path(Path::new("src/content/blog/hello-world.md")).unwrap();
            index.add_path(Path::new("README.md")).unwrap();
            let tree_oid = index.write_tree().unwrap();
            index.write().unwrap();
            let tree = repo.find_tree(tree_oid).unwrap();
            let sig = git2::Signature::now("Fixture", "fixture@test").unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "fixture: init", &tree, &[])
                .unwrap();
        }

        let st = RepoState {
            repo,
            root,
            config: Some(crate::config::parse_config(CONFIG_YAML).unwrap()),
            touched: Vec::new(),
        };
        (dir, st)
    }

    const CONFIG_YAML: &str =
        "content:\n  - name: blog\n    path: src/content/blog\n    fields:\n      - name: title\n        type: string\n";

    fn head_blob(st: &RepoState, path: &str) -> Vec<u8> {
        let head = st.repo.head().unwrap().peel_to_commit().unwrap();
        let tree = head.tree().unwrap();
        let entry = tree.get_path(Path::new(path)).unwrap();
        st.repo.find_blob(entry.id()).unwrap().content().to_vec()
    }

    fn fm(yaml: &str) -> Value {
        serde_yaml_ng::from_str(yaml).unwrap()
    }

    #[test]
    fn commit_no_se_lleva_al_archivo_ajeno_sucio() {
        let (dir, mut st) = setup_repo();
        // El usuario ensucia un archivo que Folio no tocó.
        fs::write(dir.path().join("README.md"), "cambio ajeno\n").unwrap();

        write_entry_tracked(
            &mut st,
            "src/content/blog/hello-world.md",
            fm("title: Hello editado\npubDate: 2026-09-28\n"),
            "Hello editado.\n",
        )
        .unwrap();
        let oid = commit_touched(&mut st, "folio: edita hello world").unwrap();

        // El commit lleva el post editado y el README original.
        assert_eq!(
            head_blob(&st, "src/content/blog/hello-world.md"),
            b"---\ntitle: Hello editado\npubDate: 2026-09-28\n---\n\nHello editado.\n"
        );
        assert_eq!(head_blob(&st, "README.md"), b"no relacionado\n");

        // El working tree conserva el cambio ajeno, sin commitear.
        assert_eq!(
            fs::read_to_string(dir.path().join("README.md")).unwrap(),
            "cambio ajeno\n"
        );
        let status = st.repo.status_file(Path::new("README.md")).unwrap();
        assert!(status.is_wt_modified());
        assert!(!status.is_index_modified());

        // HEAD apunta al commit nuevo y la lista de tocados queda limpia.
        assert_eq!(
            st.repo.head().unwrap().peel_to_commit().unwrap().id().to_string(),
            oid
        );
        assert_eq!(st.touched, Vec::<PathBuf>::new());
    }

    #[test]
    fn commit_crea_entrada_nueva_con_padre() {
        let (_dir, mut st) = setup_repo();
        write_entry_tracked(
            &mut st,
            "src/content/blog/nuevo.md",
            fm("title: Nuevo\npubDate: 2026-10-01\n"),
            "cuerpo\n",
        )
        .unwrap();
        commit_touched(&mut st, "folio: crea nuevo").unwrap();
        assert!(head_blob(&st, "src/content/blog/nuevo.md").len() > 0);
        let head = st.repo.head().unwrap().peel_to_commit().unwrap();
        assert_eq!(head.parent_count(), 1);
        assert_eq!(head.message().unwrap(), "folio: crea nuevo");
    }

    #[test]
    fn commit_rechaza_index_con_cambios_ajenos_stageados() {
        let (dir, mut st) = setup_repo();
        fs::write(dir.path().join("README.md"), "ajeno stageado\n").unwrap();
        let mut index = st.repo.index().unwrap();
        index.add_path(Path::new("README.md")).unwrap();
        index.write().unwrap();

        write_entry_tracked(
            &mut st,
            "src/content/blog/hello-world.md",
            fm("title: Hello\npubDate: 2026-09-28\n"),
            "x\n",
        )
        .unwrap();
        let err = commit_touched(&mut st, "m").unwrap_err();
        assert!(err.contains("stageados"), "err: {err}");
    }

    #[test]
    fn commit_sin_tocados_o_sin_mensaje_es_error() {
        let (_dir, mut st) = setup_repo();
        assert!(commit_touched(&mut st, "m").unwrap_err().contains("nada"));
        st.touched.push(PathBuf::from("src/content/blog/x.md"));
        assert!(commit_touched(&mut st, "   ").unwrap_err().contains("vacío"));
    }
}
