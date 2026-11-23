# Public website

`apps/website` is a static Astro 7 + Starlight site. It contains the public product pages and the versioned documentation under `src/content/docs`.

## Local commands

```sh
cd apps/website
npm install --cache /tmp/nova-npm-cache
npm run dev
npm run check
npm run build
npm run test
```

`npm run test` verifies:

1. Nova TextMate grammar scopes (comments, strings, interpolation, numbers, functions, types, operators).
2. Internal links, canonical paths, required routes, sitemap, robots and Pagefind assets.
3. Every documentation fence marked ` ```nova check ` compiles with `nova check`, plus `examples/hello`.

## Base paths

The site is designed for both a root site and a project Pages path:

```sh
BASE_PATH=/nova/ SITE_URL=https://jd4rider.github.io npm run build
BASE_PATH=/nova/ SITE_URL=https://jd4rider.github.io npm run test:site
```

No absolute internal link is hardcoded outside the `href()` helper or Starlight's own base-aware routing. The verifier rejects links that escape the configured base.

## Deployment

`.github/workflows/pages.yml` builds on PRs and deploys on pushes to `main` once GitHub Pages is configured to use **GitHub Actions** as its source. It derives `BASE_PATH` from the repository name (including `<owner>.github.io` root sites) and derives `SITE_URL` from the owner.

The public repository is <https://github.com/jd4rider/nova>. GitHub Pages is configured with **GitHub Actions** as its source, and the workflow has deployed successfully to <https://jd4rider.github.io/nova/>. The live site is verified at the home, documentation hub, blog, website guide and Pagefind routes. A custom domain and signed application releases are not claimed.

Action tags are used with versions visible in the workflow; pin them to reviewed commit SHAs before a production release.

## Content rules

- Central maturity data: `src/data/maturity.ts`.
- No conceptual screenshots presented as shipped UI.
- No invented download links; releases page says none exist.
- Documentation distinguishes normative reference from tutorials and marks planned syntax.
- Real code samples only. Complete examples use `nova check`; fragments are not presented as verified programs.
- Shared Nova grammar: `grammar/nova.tmLanguage.json`.
