import theme from './fileIcons.json'

const NOVA_ICON =
  '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32"><rect width="32" height="32" rx="5" fill="#1f6feb"/><path fill="#fff" d="M9 25V7h4l6 10.5V7h4v18h-4l-6-10.5V25Z"/></svg>'

type IconTheme = {
  icons: Record<string, string>
  extensions: Record<string, string>
  names: Record<string, string>
  folders: Record<string, string>
  foldersExpanded: Record<string, string>
  file: string
  folder: string
  folderExpanded: string
}

const data = theme as IconTheme

function iconForFile(name: string): string | undefined {
  const lower = name.toLowerCase()
  const exact = data.names[lower] ?? data.names[name]
  if (exact) return exact
  const suffixes = lower.split('.')
  for (let index = 1; index < suffixes.length; index++) {
    const key = suffixes.slice(index).join('.')
    const match = data.extensions[key]
    if (match) return match
  }
  return data.extensions[lower]
}

function iconForFolder(name: string, open: boolean): string {
  const lower = name.toLowerCase()
  const map = open ? data.foldersExpanded : data.folders
  return map[lower] ?? data.folders[name] ?? (open ? data.folderExpanded : data.folder)
}

export function MaterialFileIcon({ name, isDir = false, open = false }: {
  name: string
  isDir?: boolean
  open?: boolean
}): JSX.Element {
  const key = isDir ? iconForFolder(name, open) : iconForFile(name)
  const svg = !isDir && name.toLowerCase().endsWith('.nova')
    ? NOVA_ICON
    : data.icons[key ?? '']
  return (
    <span
      className="file-glyph material"
      aria-hidden="true"
      dangerouslySetInnerHTML={{ __html: svg ?? data.icons[data.file] }}
    />
  )
}
