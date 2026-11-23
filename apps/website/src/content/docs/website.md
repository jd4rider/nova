---
title: Website and documentation
description: The Astro/Starlight site, its verification gates and the GitHub Pages deployment path.
---

The public site lives in `apps/website`. It is a static Astro 7 application with Starlight documentation, a shared Nova TextMate grammar for Shiki highlighting, Pagefind search and a small custom Gruvbox marketing layer.

## Local commands

```sh
cd apps/website
npm ci --no-audit --no-fund
npm run check
npm run build
npm run test
```

`npm run test` checks the shared grammar scopes, required routes, internal links, canonical URLs, sitemap, Pagefind assets and a small set of real Nova programs compiled by the CLI.

To validate a project-site base before deployment:

```sh
BASE_PATH=/nova/ SITE_URL=https://example.github.io npm run build
BASE_PATH=/nova/ SITE_URL=https://example.github.io npm run test:site
```

## Deployment

`.github/workflows/pages.yml` builds the compiler for checked snippets, installs the website with a frozen lockfile, runs Astro checks, validates a project base, builds at the repository base, uploads the static artifact and deploys it with the official GitHub Pages actions. The workflow derives `BASE_PATH` from the repository name and `SITE_URL` from the owner.

The production site is expected at `https://jd4rider.github.io/nova/` once the Pages environment is enabled. Deployment status should be read from the Actions run, not inferred from a local build.

## Content rules

- Do not publish a screenshot unless it was captured from the current application.
- Do not publish download buttons for artifacts that do not exist.
- Mark features as tested, implemented, experimental, planned or unavailable.
- Link to source or a test for claims that can be verified.
- Treat the shared grammar and checked snippets as part of the production build.
