# Nova website

The static public site for the Nova language, documentation, Studio guide and launch blog.

**Production:** <https://jd4rider.github.io/nova/>

Built with Astro 7, Starlight 0.43, the shared Nova TextMate grammar through Shiki, and Pagefind. It deploys to GitHub Pages under the project base `/nova/`.

## Local development

```sh
npm ci --no-audit --no-fund
npm run dev
```

The dev server uses `SITE_URL=http://localhost:4321` and base `/` unless overridden.

## Verification

```sh
npm run check      # Astro/TypeScript diagnostics
npm run build      # static production build
npm run test       # grammar scopes, routes, links, search, checked Nova snippets
```

The snippet check requires a working compiler. By default it runs `target/debug/nova` from the repository root; set `NOVA_BIN` to override it.

Project-base validation, matching Pages:

```sh
BASE_PATH=/nova/ SITE_URL=https://jd4rider.github.io npm run build
BASE_PATH=/nova/ SITE_URL=https://jd4rider.github.io npm run test:site
```

## Content

- `src/content/docs/` — Starlight documentation: getting started, language, stdlib, toolchain, Studio, internals, project and website guide.
- `src/pages/docs/index.astro` — documentation hub tab.
- `src/pages/blog/` — field notes and the Rust launch post.
- `src/pages/` — marketing, install, Studio, roadmap, status, security and license pages.
- `src/data/maturity.ts` — central feature status manifest.
- `../../grammar/nova.tmLanguage.json` — shared syntax grammar.

Do not add download buttons, screenshots or benchmark results that are not backed by a real artifact or measurement. Status labels mean **tested**, **implemented**, **experimental**, **planned** or **unavailable**.

## Deployment

`.github/workflows/pages.yml` builds the compiler, verifies documentation snippets, checks the site at root and `/nova/`, uploads the Pages artifact and deploys. In the GitHub repository, **Settings → Pages → Build and deployment → Source** must be **GitHub Actions** (the workflow also configures this where permitted).

The workflow derives `BASE_PATH` from the repository name and `SITE_URL` from the owner. All internal links go through `src/lib/url.ts` so they remain base-safe.

## License

The website source follows the repository’s undecided license state. Astro, Starlight, Shiki and Pagefind retain their own licenses.
