export const siteName = 'Nova'
export const siteDescription =
  'A native-compiled programming language with explicit semantics, a Rust toolchain, a browser playground, the Nova Studio desktop IDE and a VS Code extension.'

export const links = {
  github: import.meta.env.PUBLIC_GITHUB_URL ?? 'https://github.com/jd4rider/nova',
  releases: import.meta.env.PUBLIC_RELEASES_URL ?? 'https://github.com/jd4rider/nova/releases',
  version: 'v0.2.0 preview',
  rust: '1.96.0',
  node: '26.13.0 / 22 CI',
  platform: 'macOS arm64 (tested)'
}
