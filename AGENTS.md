# AGENTS.md — Folio

Instrucciones para cualquier agente (o humano) que trabaje en este repositorio. Folio lo construye su dueño; el agente ejecuta, no decide el producto.

Las reglas generales provienen del contrato de Circulo (`soycanopa/circulo`, AGENTS.md). Lo marcado **[Folio]** es de este producto.

## Acuerdo de trabajo

- Sé directo y técnico.
- Si propones algo, explica el porqué con la fuente. Aquí: docs de Tauri 2, docs del crate `git2`, la API de GitHub, y el código de Pages CMS. No cites de memoria una API.
- Si una decisión humana contradice la barra de calidad o un invariante, levanta la mano. No la implementes para quedar bien. Constrúyelo sólido.
- Si tienes una idea mejor, propónla y di por qué.
- **No inventes APIs.** Si no está en la doc oficial del crate o del framework, no existe. Lee antes de escribir.
- **No asumas. Pregunta.** Si el producto, el protocolo o un permiso no están claros, pregunta antes de implementar.
- **Sin cambios sin un plan** (corto basta: el issue o el mensaje de commit). Trabajo grande → plan escrito y aprobado antes de codear.
- **No entres en bucle.** Si un enfoque falla dos veces, para, escribe qué probaste y cambia de estrategia o pregunta.

## Regla de oro del producto **[Folio]**

**Pages CMS es la fuente de la verdad. Folio es Pages CMS corriendo en la Mac del dueño; la única diferencia es que no hay server.**

- Antes de construir cualquier pantalla, flujo, icono o string: **lee el código de Pages CMS** (`hunvreus/pagescms`, MIT) y cópialo. Porta sus componentes, sus tokens, sus layouts, su copy. Nada de interpretaciones ni rediseños "inspirados".
- Cuando portes un archivo, va con cabecera de licencia MIT y entrada en `third_party/NOTICE`.
- Si una función de su producto depende de su server (login, cuentas, Actions), **no inventes el equivalente local**: presenta cómo se comporta la web, explica la dependencia, y el dueño decide. Ejemplos ya decididos: sesión GitHub por OAuth Device Flow + Keychain; templates creados con la API `generate` del token del usuario.
- "A medias" no existe: si algo va, va completo como en la app, o explícitamente queda fuera con el dueño enterado.

## Git y entrega

- **Cada producto vive en su propio repositorio.** Nada de mezclar productos en un monorepo. Extraer una feature a un repo aparte requiere decisión explícita del dueño: el agente no decide solo cuándo algo es "grande" — lo propone y el dueño resuelve.
- Todo el trabajo va en ramas (`feature/<fase-o-tema>`). Nunca commit directo a `main`; `main` solo recibe merges `--no-ff` de fase o fix.
- Commits granulares: un cambio lógico cada uno. Mensaje `type(scope): summary` (`feat(core): …`, `fix(ui): …`, `test: …`, `docs: …`, `chore: …`).
- Los tests son parte del trabajo, no un extra. El parser, el stage de git y cualquier módulo de core llevan sus tests en el mismo commit, nunca después.
- Antes de dar por cerrado un cambio: `cargo test --manifest-path src-tauri/Cargo.toml` y `pnpm build` pasan en local.
- Versiones y releases: la versión vive en `src-tauri/Cargo.toml`, `package.json` y `src-tauri/tauri.conf.json` (las tres, juntas). Cada fase o hito cierra con un tag `vX.Y.Z` sobre su merge y una release de GitHub cuyas notas listan los commits granulares del rango. Empezamos en `v0.0.0`.
- No se commitea `src-tauri/target/`, `node_modules/`, `dist/` ni el `.git` de fixtures.

## Código

- Funciones cortas. Errores explícitos. Sin abstracción prematura.
- Comentarios solo para el porqué, un invariante o una trampa. No narres lo que el código ya dice.
- Un módulo, una responsabilidad. Nada de un archivo dios.
- No crees un framework interno por si acaso.
- El modelo de datos vive en el core (Rust); el frontend tipa espejo, no decide.

## Invariantes de Folio **[Folio]**

- Rust posee disco y git. El webview no escribe archivos; llama comandos.
- Un write de contenido tiene que resolver dentro de `content[].path` o `media.input` del `.pages.yml`. Si no, se rechaza.
- El commit stagea solo los paths que Folio tocó en la sesión. Prohibido `git add -A`. (El modo remoto, cuando exista, cumple lo mismo por diseño: el árbol de la Git Data API solo lleva los paths tocados.)
- No se guardan tokens en archivos ni en el webview. La sesión de GitHub vive en el Keychain de macOS, creada con `security add-generic-password -T <binario>` (ACL del binario, sin prompts) y cacheada en memoria por corrida. El push usa la credencial del sistema.
- No se crea `.pages.yml` ni una colección por iniciativa propia.
- No se embebe el servidor de Pages CMS (Next, Postgres, GitHub App).
- La UI no sabe de git más allá de lo que el comando devuelve. El core no importa componentes React.

## Trampas del harness **[Folio]**

Leídas a la mala; no vuelvas a pisarlas:

- **HMR encadenado + recompilado de Rust = pantalla rota.** Con cambios grandes de frontend, relanza `pnpm tauri dev` en vez de confiar en el hot-reload. El ErrorBoundary de `main.tsx` se queda: un crash se ve en pantalla, nunca negro. Si el dueño reporta un crash, pídele el texto del panel.
- **`window.prompt` no existe** en el WKWebView de Tauri. Cualquier input va en componente propio.
- **pnpm no expone transitivos**: si un archivo importado vive solo como dependencia de otro paquete (p.ej. `@tiptap/core`), agrégalo como dependencia directa.
- **`serde_yaml` está archivado**; se usa `serde_yaml_ng`. El storage de TipTap no viene tipado para v3: acceso acotado con cast documentado.
- **git2 sin transports** (`default-features = false`): clone y push van por el `git` del sistema. `Index::add_path` stagea el archivo completo e ignora gitignore; el commit escribe el index a disco antes de crear el commit (como `git add` real).
- **El scope del asset protocol se amplía en runtime** solo a la carpeta del repo abierto (`app.asset_protocol_scope().allow_directory`); las miniaturas del webview van por `convertFileSrc`. En remoto también se amplía a `{app_cache_dir}/remote-media` (caché de previews privadas, clave sha).
- **El keyring crate genera prompts en dev** (cada rebuild cambia el binario). Por eso la sesión usa el CLI `security` con ACL — y además, en debug, el item se crea con `-A`: la firma ad-hoc cambia en cada rebuild y sin `-A` macOS pide la clave del keychain en cada arranque. Tradeoff del dueño (2026-10-02): en dev cualquier app del usuario puede leer el item; release mantiene el ACL por binario (identidad estable, un "Always Allow"). No lo "simplifiques" de vuelta.
- **Los comandos síncronos de Tauri corren en el main thread** (doc "Calling Rust"): cualquier comando que dispatchea a red es `async fn` con args owned y la red en `tauri::async_runtime::spawn_blocking`; el lock del `AppState` nunca cruza un await (clonar datos → red → re-lock comprobando que el proyecto remoto siga siendo el mismo).
- **`titleBarStyle: "Overlay"` (macOS)**: los semáforos quedan sobre la esquina del sidebar (padding `pt-7`) y la ventana se arrastra con `data-tauri-drag-region` — que exige el permiso `core:window:allow-start-dragging` en la capability (`core:default` NO lo trae) y no arrastra con la ventana sin foco (issue #4316 de tauri). El atributo solo aplica al elemento exacto, no a los hijos.
- **`Error::StatusCode` de ureq 3 no expone el body** (ahí vive el `message` de GitHub del 409/422): `gh_api` construye su agente con `http_status_as_error(false)` y lee el status a mano. Además ureq **nunca reenvía `Authorization` tras un redirect** (default `Never`): la descarga de media privada persigue redirects a mano, solo a hosts `github.com`/`*.githubusercontent.com`.

## Pages CMS **[Folio]**

Excepción a "referencias: ideas sí, código no": su repo es MIT y la regla de oro manda copiar. Se copian componentes, tokens, templates y textos, con cabecera y `third_party/NOTICE`. No se copia ni ejecuta `app/`, `db/` ni el setup de la GitHub App.

## Mapa del repo

```text
src-tauri/src/     core: repo.rs (open/status/clone), config.rs (.pages.yml),
                   entry.rs, file_entry.rs, commit.rs, push.rs, media.rs,
                   history.rs, github.rs (Device Flow + Keychain), recent.rs,
                   gh_api.rs (cliente REST/GraphQL de GitHub), remote.rs
                   (modo remoto: save/auto-rename/rename Git Data/proxy de
                   previews), commit_message.rs (plantillas del remoto)
src/components/    editor/ (port de su editor), ui/ (su pila shadcn),
                   home/ (su page + repo-select/latest/templates)
src/lib/           templates.ts (su lista), tracker.ts (sus visitas + modo),
                   media-src.ts (src de previews local/remoto), utils (cn)
fixtures/blog/     repo mínimo para los tests del crate
```

El playground de pruebas manuales y los docs de producto (PRD, TRD, UX, UI, FLOW, DESIGN, IMPLEMENTATION) viven en el workspace del dueño, fuera de este repo. La fuente pública de comportamiento es este archivo y el README.

## Alcance y siguientes pasos **[Folio]**

Hecho: paridad con la plataforma (editor portado, home con sesión GitHub, media, canvas, operaciones de entrada, campos completos) y **proyectos remotos** (abrir sin clonar como la web, edición contra la API de GitHub, Save publica directo en la rama por defecto; commit-messages portados con override del `.pages.yml`; 409/422 con auto-rename; rename por Git Data; historial con link a GitHub; previews privadas por proxy del core).

Pendiente acordado con el dueño:

1. **Agente de contenido (v2)**: chat sobre la entrada abierta, transporte `opencode serve` (HTTP+SSE), scope file|collection, aceptar pasa por `write_entry`, rechazar no toca disco. Sin shell, sin push, no edita Astro ni `src-tauri`.

Fuera de alcance explícito (decisión del dueño, 2026-10-02): RepoBranches (cambio/creación de rama en remoto — se abre con la rama por defecto), editor `/configuration` para repos sin `.pages.yml`, PRs para repos con rules (solo el error que los sugiere, como la web).
