# CLAUDE.md -- Documentation Site

GitHub Pages SPA at [yongsk0066.github.io/corevoikko](https://yongsk0066.github.io/corevoikko/). Single HTML file, no build step for deployment.

## Stack

- **Carbon Design System** web components via CDN (`tag/latest/*.min.js`)
- **carbon-type.css** — Carbon type utility classes compiled from `@carbon/styles` Sass
- **marked.js** — Markdown → HTML
- **highlight.js** — Syntax highlighting (github light theme)
- **mermaid.js** — Diagram rendering

## File Layout

```
docs/
├── index.html         # SPA (HTML + CSS + JS, ~650 lines)
├── carbon-type.css    # Compiled type classes (committed, 24KB)
├── carbon-type.scss   # Sass source (gitignored)
├── package.json       # Dev deps for Sass compilation (gitignored)
└── node_modules/      # (gitignored)
```

## Carbon Components Used

| Component | CDN File | Purpose |
|-----------|----------|---------|
| `cds-header`, `cds-side-nav` | `ui-shell.min.js` | App shell + navigation |
| `cds-tile` | `tile.min.js` | Demo feature cards |
| `cds-button` | `button.min.js` | Action buttons |
| `cds-text-input`, `cds-textarea` | `text-input.min.js`, `textarea.min.js` | Demo inputs |
| `cds-code-snippet` | `code-snippet.min.js` | Code blocks (demo + docs) |
| `cds-tag` | `tag.min.js` | Function name badges |
| `cds-inline-loading` | `inline-loading.min.js` | WASM loading state |
| `cds-inline-notification` | `notification.min.js` | Load success/error |
| `cds-callout` | `notification.min.js` | Markdown blockquotes |
| `cds-skeleton-text` | `skeleton-text.min.js` | Doc loading skeleton |
| `cds-stack` | `stack.min.js` | Tile vertical spacing |
| `cds-table` | `data-table.min.js` | Markdown tables |
| `cds-link` | `link.min.js` | Footer links |
| `cds-breadcrumb` | `breadcrumb.min.js` | Doc file path navigation |

## Carbon Type Classes

Typography uses `.cds--type-*` utility classes from `carbon-type.css` instead of manual CSS:

| Element | Class |
|---------|-------|
| Page title | `cds--type-heading-05` |
| Markdown h1 | `cds--type-heading-05` |
| Markdown h2 | `cds--type-heading-03` |
| Markdown h3 | `cds--type-heading-02` |
| Body text / lists | `cds--type-body-01` |
| Tile headings | `cds--type-heading-compact-01` |
| Description text | `cds--type-body-compact-01` |

## Rebuilding carbon-type.css

Only needed when upgrading `@carbon/styles`:

```bash
cd docs
npm install
npx sass --load-path=node_modules carbon-type.scss carbon-type.css --style=compressed
```

Commit the resulting `carbon-type.css`.

## Architecture

### SPA Routing

Hash-based: `#/demo` for interactive demo, `#/doc/{path}` for markdown documents. Router updates `cds-side-nav` active states and manages view switching.

### Markdown Post-Processing Pipeline

`loadDoc()` fetches markdown from GitHub raw, then applies 5 sequential transforms:

1. **Type classes** — adds `cds--type-*` classes to h1-h3, p, ul, ol
2. **Blockquotes** — `<blockquote>` → `<cds-callout kind="info">`
3. **Mermaid** — extracts `code.language-mermaid` before hljs can corrupt them
4. **Tables** — `<table>` → `<cds-table>` with header/body/cell structure
5. **Code blocks** — `<pre><code>` → `<cds-code-snippet type="multi">` with hljs highlighting and `copy-text` attribute
6. **Mermaid render** — runs after container is visible (requires measured dimensions)

### Custom CSS (Not Replaceable by Carbon)

These use Carbon tokens but have no matching web component:

- `.result` — monospace output with colored status spans
- `.input-row` — horizontal input + button flex layout
- `.tile-header` / `.tile-body` — tile internal anatomy (Carbon tiles are intentionally flexible)
- `.content-enter` — page transition animation (Carbon motion tokens)
- `.hidden` — display:none toggle (different from `cds--visually-hidden`)

### Motion Tokens

Manually defined in `:root` because `themes.css` does not include motion:

- `--cds-duration-moderate-02`: 240ms
- `--cds-ease-entrance`: `cubic-bezier(0, 0, 0.38, 0.9)` (productive)
- `--cds-ease-standard`: `cubic-bezier(0.2, 0, 0.38, 0.9)` (productive)
