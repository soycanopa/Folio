// E2e del spike Fase 0 contra el fixture real (IMPLEMENTATION.md):
// copiar fixtures/blog a un tempdir, hacerlo repo, editar + crear un post
// con los mismos funciones que usan los comandos, commit, y verificar
// que el árbol ajeno queda intacto.
use std::{
    fs,
    path::{Path, PathBuf},
};

use folio_lib::{
    commit::commit_touched,
    entry::{parse_entry, read_entry_at, write_entry_tracked},
    state::RepoState,
};
use serde_yaml_ng::Value;

fn copy_fixture(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).unwrap();
    for entry in fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        if entry.file_name() == ".git" {
            continue;
        }
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if from.is_dir() {
            copy_fixture(&from, &to);
        } else {
            fs::copy(&from, &to).unwrap();
        }
    }
}

fn collect_files(dir: &Path, base: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        if entry.file_name() == ".git" {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, base, out);
        } else {
            out.push(path.strip_prefix(base).unwrap().to_path_buf());
        }
    }
}

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/blog")
}

#[test]
fn spike_edita_crea_commitea_y_el_arbol_ajeno_queda_intacto() {
    let fixture = fixture_root();
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    copy_fixture(&fixture, &root);

    let repo = git2::Repository::init(&root).unwrap();
    // Commit inicial del fixture, como el setup de su README.
    {
        let mut files = Vec::new();
        collect_files(&root, &root, &mut files);
        let mut index = repo.index().unwrap();
        for f in &files {
            index.add_path(f).unwrap();
        }
        let tree_oid = index.write_tree().unwrap();
        index.write().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        let sig = git2::Signature::now("Fixture", "fixture@test").unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "fixture: init", &tree, &[])
            .unwrap();
    }

    let mut st = RepoState {
        repo,
        root: root.clone(),
        touched: Vec::new(),
    };

    // El usuario ensucia un archivo ajeno a la colección.
    fs::write(root.join("README.md"), "cambio ajeno\n").unwrap();

    // Editar hello-world: front matter completo reenviado (con `custom`).
    let entry = read_entry_at(&root, "src/content/blog/hello-world.md").unwrap();
    let mut fm = match entry.frontmatter {
        Value::Mapping(m) => m,
        other => panic!("front matter inesperado: {other:?}"),
    };
    fm.insert(Value::from("title"), Value::from("Hello world (editado)"));
    write_entry_tracked(
        &mut st,
        "src/content/blog/hello-world.md",
        Value::Mapping(fm),
        &format!("{}Editado desde el spike.\n", entry.body),
    )
    .unwrap();

    // Crear un post nuevo.
    let (fm_nuevo, body_nuevo) =
        parse_entry("---\ntitle: Post nuevo del spike\npubDate: 2026-10-01\n---\n\nCuerpo nuevo.\n")
            .unwrap();
    write_entry_tracked(
        &mut st,
        "src/content/blog/post-nuevo-spike.md",
        Value::Mapping(fm_nuevo),
        &body_nuevo,
    )
    .unwrap();

    // Un solo commit con ambos paths tocados.
    let oid = commit_touched(&mut st, "folio: spike — edita y crea").unwrap();

    let head = st.repo.head().unwrap().peel_to_commit().unwrap();
    assert_eq!(head.id().to_string(), oid);
    let tree = head.tree().unwrap();

    let blob = |path: &str| -> Vec<u8> {
        let e = tree.get_path(Path::new(path)).unwrap();
        st.repo.find_blob(e.id()).unwrap().content().to_vec()
    };

    // El post editado entra con el título nuevo y el campo desconocido
    // del fixture intacto.
    let edited = blob("src/content/blog/hello-world.md");
    let edited = String::from_utf8(edited).unwrap();
    assert!(edited.contains("title: Hello world (editado)"));
    assert!(edited.contains("custom: preserved-by-folio"));
    assert!(edited.contains("Editado desde el spike."));

    // El post nuevo entra.
    assert!(blob("src/content/blog/post-nuevo-spike.md").len() > 0);

    // El README commiteado sigue siendo el original del fixture y el
    // cambio ajeno sigue sucio en el working tree.
    assert_eq!(
        blob("README.md"),
        fs::read(fixture.join("README.md")).unwrap()
    );
    assert_eq!(
        fs::read_to_string(root.join("README.md")).unwrap(),
        "cambio ajeno\n"
    );
    let status = st.repo.status_file(Path::new("README.md")).unwrap();
    assert!(status.is_wt_modified());
    assert!(!status.is_index_modified());
}
