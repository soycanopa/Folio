# Folio

**Pages CMS running on your Mac.** Folio is a desktop CMS for git repositories: it reads `.pages.yml`, builds editing forms for your content collections, and commits changes to your local clone — the Pages CMS experience (editor, UI and workflows), without the server.

```text
┌ sidebar │  Blog › Editing "Introduction"      ⏱   Save  ··· ┐
│ repo    │                                                     │
│ Content │   Title          Required                           │
│  Blog   │   [                                            ]    │
│  Notes  │   Publish date   Required                           │
│ Media   │   [                                            ]    │
│         │   Body                     Editor | Source         │
│ avatar  │   ┌────────────────────────────────────────────┐   │
└─────────┴── └────────────────────────────────────────────┘   ┘
```

## Why

If your site treats content as files (Markdown + front matter, images, a `.pages.yml` describing collections — Astro, Next, Eleventy, Jekyll…), editing posts means either hand-editing and committing, or using a hosted CMS that depends on its cloud. Folio is that CMS on your machine: the repository is the only source of truth, and everything Pages CMS does in the cloud, Folio does locally.

## Features

- **Projects home** — sign in with GitHub (OAuth Device Flow, token in the macOS Keychain) to see your recently updated repositories, browse them, clone in one click, or open a local folder. Includes *Create from a template* (Next.js / Astro / Eleventy blog templates).
- **Remote projects (v0.4)** — open any repo *without cloning*, exactly like the Pages CMS web app: read collections through the GitHub API (GraphQL tree per folder), and **Save publishes straight to the branch** — one file, one commit, with the platform's commit-message templates (`settings.commit.templates`), 409/422 conflict handling and auto-rename. Media uploads publish immediately; private-repo previews are proxied through the core (the token never reaches the webview).
- **Collections from `.pages.yml`** — the sidebar, forms, table columns and validation come from your config, like the platform does. Unknown front-matter fields survive every save.
- **The real Pages CMS editor** — the actual `components/ui/editor` from [Pages CMS](https://github.com/hunvreus/pagescms) (MIT), ported: bubble menu, slash commands (`/`), GFM tables, links, image alt text, Markdown input/output.
- **Media library** — upload any file into `media.input` (respecting `rename: false | safe | random`), pick images from any `image` field, copy public paths, delete.
- **Git the way the platform does it** — Save writes the file; commit stages **only the paths Folio touched** (never `git add -A`); push is explicit. File history dropdown marks commits that haven't reached the upstream as *local*.
- **Entry operations** — create with slug/filename templates, rename, delete — all honoring `operations` from the config.
- **Canvas (v2)** — arrange a `showcase` collection on an infinite canvas; the layout lives in a `layout.json` beside the content, committed like any file.
- **Full field set** — string, text, date, image, file, rich-text, number, boolean, select, code.

## Safety invariants

- Rust owns the disk and git; the webview only invokes commands.
- Content writes must resolve inside `content[].path` or `media.input` — anything else is rejected.
- Commits stage only paths Folio touched in the session; in remote mode every PUT/DELETE publishes exactly the path Folio touched.
- No tokens in plain files: the GitHub session lives in the macOS Keychain (ACL-scoped to the binary). Push uses the system git credentials.

## Install & run

Requires Rust (stable), Node 24+, pnpm, and git.

```bash
pnpm install
pnpm tauri dev      # development
pnpm tauri build    # production .app / .dmg
```

Point it at any local git repository with a `.pages.yml` at its root (the same file the Pages CMS platform reads).

## Stack

Tauri 2 (Rust core) · React + TypeScript + Vite · Tailwind CSS v4 · shadcn/ui + Radix (ported from Pages CMS) · TipTap (@tiptap/markdown) · git2 (local git) · system `git` for clone/push.

```text
folio/
  src-tauri/          Rust core: commands, git, .pages.yml parser, media, GitHub session, GitHub API client (remote mode)
  src/                React app: home, sidebar, table, forms, editor port
  third_party/NOTICE  MIT attribution of everything ported from Pages CMS
  fixtures/blog/      minimal repo for the test-suite
```

## Roadmap

- **Remote projects** — open a GitHub repository without cloning, editing directly against the API (the way app.pagescms.org works): Save would commit to the remote branch.

## Third-party

Folio ports UI components, the editor, design tokens and templates from [Pages CMS](https://github.com/hunvreus/pagescms) (MIT). See [`third_party/NOTICE`](third_party/NOTICE) for the full list. Nothing from its server (Next.js app, database, GitHub App) is used or embedded.

## License

MIT — same as Pages CMS.

---

#Tauri #Rust #React #TypeScript #PagesCMS #CMS #Git #Markdown #Astro #OpenSource #DesktopApp #ContentManagement #WYSIWYG #TipTap #shadcn
