export const siteName = 'Nova'
export const siteDescription =
  'A native-compiled programming language with explicit semantics, a Rust toolchain and the Nova Studio desktop IDE.'

export const links = {
  github: import.meta.env.PUBLIC_GITHUB_URL ?? 'https://github.com/jd4rider/nova',
  releases: import.meta.env.PUBLIC_RELEASES_URL ?? 'https://github.com/jd4rider/nova/releases',
  version: 'v0.1.0 preview',
  rust: '1.96.0',
  node: '26.13.0 / 22 CI',
  platform: 'macOS arm64 (tested)'
}
