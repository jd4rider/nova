import type { APIRoute } from 'astro'
import { href } from '../lib/url'

export const GET: APIRoute = ({ site }) => {
  const sitemap = new URL(href('sitemap-index.xml'), site ?? 'http://localhost:4321')
  return new Response(`User-agent: *\nAllow: /\n\nSitemap: ${sitemap}\n`, {
    headers: { 'Content-Type': 'text/plain; charset=utf-8' }
  })
}
