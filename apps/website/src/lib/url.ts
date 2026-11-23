export const base = import.meta.env.BASE_URL.endsWith('/')
  ? import.meta.env.BASE_URL
  : `${import.meta.env.BASE_URL}/`

/** Build a URL that works both at `/` and under a GitHub Pages project base. */
export function href(path = ''): string {
  return `${base}${path.replace(/^\/+/, '')}`
}
