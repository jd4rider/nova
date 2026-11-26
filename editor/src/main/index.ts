import {
  app,
  BrowserWindow,
  dialog,
  ipcMain,
  Menu,
  session,
  shell
} from 'electron'
import type { IpcMainInvokeEvent } from 'electron'
import type { ChildProcess } from 'node:child_process'
import { execFile, spawn } from 'node:child_process'
import fs from 'node:fs/promises'
import fsSync from 'node:fs'
import path from 'node:path'
import { LspClient } from './lsp'
import { isNovaProject, novaExecutable } from './toolchain'
import { isAbortError, streamChat } from './ai'
import { configureAi, getAiSettings, resolveAiConfig } from './credentials'
import { CopilotService } from './copilot'
import { CopilotSdkService } from './copilot-sdk'
import { FreeBuffChat } from './freebuff-chat'
import { FreeBuffCompletion } from './freebuff-completion'
import type {
  AiConfigurePatch,
  AiContext,
  AiMessage,
  FileEntry,
  FreeBuffStatus,
  LspDiagnostic,
  TaskResult,
  TerminalInfo
} from '../shared/types'

let mainWindow: BrowserWindow | null = null
let workspaceRoot: string | null = null
let workspaceReal: string | null = null
let lspClient: LspClient | null = null
let runningTask: ChildProcess | null = null

let ptyModule: any = null
try {
  // Optional native dependency. If it is unavailable the terminal still works
  // in honest pipe mode, and the FreeBuff TUI reports that a PTY is required.
  ptyModule = require('node-pty')
} catch {
  ptyModule = null
}

interface TerminalSession {
  pty?: any
  child?: ChildProcess
  write: (data: string) => void
  resize: (cols: number, rows: number) => void
  kill: () => void
}

const terminals = new Map<number, TerminalSession>()
let terminalCounter = 1
const aiRequests = new Map<string, AbortController>()

function send(channel: string, payload: unknown): void {
  if (!mainWindow || mainWindow.isDestroyed() || mainWindow.webContents.isDestroyed()) return
  mainWindow.webContents.send(channel, payload)
}

const copilot = new CopilotSdkService((state) => send('copilot:status', state))
const copilotCompletions = new CopilotService(() => undefined)
const freebuffChat = new FreeBuffChat((event) => send('freebuff:event', event))
const freebuffCompletion = new FreeBuffCompletion()

function guard(event: IpcMainInvokeEvent): void {
  if (!mainWindow || event.sender !== mainWindow.webContents) {
    throw new Error('unauthorized IPC sender')
  }
}

function workspaceStateFile(): string {
  return path.join(app.getPath('userData'), 'workspace.json')
}

async function readLastWorkspace(): Promise<string | null> {
  try {
    const state = JSON.parse(await fs.readFile(workspaceStateFile(), 'utf8')) as { root?: unknown }
    return typeof state.root === 'string' && state.root.length > 0 ? state.root : null
  } catch {
    return null
  }
}

async function rememberWorkspace(root: string): Promise<void> {
  try {
    await fs.mkdir(path.dirname(workspaceStateFile()), { recursive: true })
    await fs.writeFile(workspaceStateFile(), JSON.stringify({ root }), 'utf8')
  } catch {
    // Best-effort: a read-only profile must not prevent opening the folder.
  }
}

async function restoreWorkspace(): Promise<{ root: string; hasManifest: boolean } | null> {
  const saved = await readLastWorkspace()
  if (!saved) return null
  try {
    const stat = await fs.stat(saved)
    if (!stat.isDirectory()) return null
    await setWorkspace(saved)
    return { root: saved, hasManifest: isNovaProject(saved) }
  } catch {
    return null
  }
}

async function setWorkspace(root: string): Promise<void> {
  workspaceRoot = root
  workspaceReal = await fs.realpath(root)
  startWatching(root)
  await rememberWorkspace(root)
}

function ensureInside(candidate: string): string {
  if (!workspaceRoot || !workspaceReal) throw new Error('no workspace is open')
  const absolute = path.resolve(candidate)
  const relative = path.relative(workspaceRoot, absolute)
  if (relative.startsWith('..') || path.isAbsolute(relative)) {
    throw new Error('path is outside the workspace')
  }
  return absolute
}

async function safeExisting(candidate: string): Promise<string> {
  const absolute = ensureInside(candidate)
  const real = await fs.realpath(absolute)
  const relative = path.relative(workspaceReal!, real)
  if (relative.startsWith('..') || path.isAbsolute(relative)) {
    throw new Error('path escapes the workspace through a symlink')
  }
  return real
}

async function safeNew(candidate: string): Promise<string> {
  const absolute = ensureInside(candidate)
  const parent = await fs.realpath(path.dirname(absolute))
  const relative = path.relative(workspaceReal!, parent)
  if (relative.startsWith('..') || path.isAbsolute(relative)) {
    throw new Error('path escapes the workspace through a symlink')
  }
  return absolute
}

const IGNORED = new Set(['.git', 'node_modules', 'target', 'dist', 'out', 'release', '.DS_Store'])

let workspaceWatcher: fsSync.FSWatcher | null = null
let watcherTimer: ReturnType<typeof setTimeout> | null = null
const watcherPaths = new Set<string>()

function stopWatching(): void {
  workspaceWatcher?.close()
  workspaceWatcher = null
  if (watcherTimer) clearTimeout(watcherTimer)
  watcherTimer = null
  watcherPaths.clear()
}

// Report coalesced filesystem changes so the renderer can refresh the explorer
// and reload clean tabs. Watcher events are advisory: the renderer re-reads
// each path and compares mtimes before touching an editor model.
function startWatching(root: string): void {
  stopWatching()
  try {
    workspaceWatcher = fsSync.watch(root, { recursive: true }, (_event, filename) => {
      const relative = filename ? String(filename) : ''
      if (relative && relative.split(/[\\/]/).some((part) => IGNORED.has(part))) return
      if (relative) watcherPaths.add(path.join(root, relative))
      if (watcherTimer) clearTimeout(watcherTimer)
      watcherTimer = setTimeout(() => {
        watcherTimer = null
        const paths = [...watcherPaths]
        watcherPaths.clear()
        send('workspace:changed', { paths })
      }, 120)
    })
    workspaceWatcher.on('error', () => stopWatching())
  } catch {
    // Recursive watching is unavailable on this platform; the explorer still
    // has a manual refresh and editors re-read on tab activation.
    workspaceWatcher = null
  }
}

async function chooseWorkspace(): Promise<{ root: string; hasManifest: boolean } | null> {
  const result = await dialog.showOpenDialog({
    properties: ['openDirectory'],
    title: 'Open a Nova workspace'
  })
  if (result.canceled || result.filePaths.length === 0) return null
  await setWorkspace(result.filePaths[0])
  lspClient?.stop()
  lspClient = null
  copilot.stop()
  copilotCompletions.stop()
  freebuffChat.stop()
  return { root: result.filePaths[0], hasManifest: isNovaProject(result.filePaths[0]) }
}

async function freebuffStatus(): Promise<FreeBuffStatus> {
  const version = await new Promise<string | null>((resolve) => {
    execFile('freebuff', ['--version'], { timeout: 3000 }, (error, stdout) => {
      resolve(error ? null : stdout.trim())
    })
  })
  if (!version) {
    return {
      available: false,
      mode: 'scraped-chat',
      headless: false,
      pty: Boolean(ptyModule),
      reason: 'FreeBuff CLI was not found on PATH. Install it, or use Copilot or another chat provider in the AI panel.'
    }
  }
  // Probe the documented surface instead of hard-coding a version's capabilities.
  const help = await new Promise<string>((resolve) => {
    execFile('freebuff', ['--help'], { timeout: 3000 }, (error, stdout) => {
      resolve(error ? '' : stdout)
    })
  })
  const headless = /--(print|json|headless)\b/.test(help)
  return {
    available: true,
    version,
    mode: 'scraped-chat',
    headless,
    pty: Boolean(ptyModule),
    reason: ptyModule
      ? 'FreeBuff exposes an interactive TUI only. Studio drives the real CLI in a PTY, reconstructs its screen with a headless terminal, and exposes that scrape as an experimental chat. It is not a supported FreeBuff API.'
      : 'FreeBuff requires a PTY. Install the optional node-pty dependency to use the scraped chat; the raw terminal needs it too.'
  }
}

function systemPrompt(context: AiContext | null): string {
  const lines = [
    'You are the coding assistant inside Nova Studio, a desktop IDE for the Nova programming language.',
    'Nova is statically typed with a Rust front end, an LLVM backend and a clang link step.',
    'Supported today: functions, structs, enums with unit variants or multi-field payloads, exhaustive match, let/var, if, while, `for i in 0..n`, Option/Result, lists, string interpolation, generic functions/types/methods, static traits, `dyn Trait` objects, `impl` methods, tuples/arrays/slices, capturing closures, expression/statement macros, `extern "C"`, synchronous async/await, map/set types, path imports and std imports such as `import std.io { println }`.',
    'Variant constructors are global (`Circle(2)`, `Empty`) and match patterns are `Circle(radius)`, `Empty` or `_`. Not supported: trait impls on generic types, real async scheduling/channels, namespaced modules, packages and named/default arguments. Say so instead of inventing syntax.',
    'Prefer complete, compilable Nova and keep explanations short. Tag code blocks as nova.'
  ]
  if (!context) {
    lines.push('No file is attached to this message.')
    return lines.join('\n')
  }
  lines.push(`The user is editing ${context.path}.`)
  if (context.selection) {
    lines.push(`Selected code:\n\`\`\`nova\n${context.selection.slice(0, 12_000)}\n\`\`\``)
  }
  lines.push(`Current file contents:\n\`\`\`nova\n${context.content.slice(0, 60_000)}\n\`\`\``)
  return lines.join('\n')
}

function createTerminal(cwd: string, cols: number, rows: number, command?: string): TerminalInfo {
  const id = terminalCounter++
  const shell = process.env.SHELL ?? '/bin/bash'
  const custom = command?.startsWith('cmd:') ? command.slice(4) : null
  const executable = custom ? shell : command ?? shell
  const args = custom ? ['-lc', custom] : []
  if (command === 'freebuff' && !ptyModule) {
    throw new Error('FreeBuff requires a PTY. Install the optional node-pty dependency.')
  }
  if (ptyModule) {
    const pty = ptyModule.spawn(executable, args, {
      cwd,
      cols,
      rows,
      env: process.env,
      name: 'xterm-256color'
    })
    pty.onData((data: string) => send('terminal:data', { id, data }))
    pty.onExit(() => {
      terminals.delete(id)
      send('terminal:exit', { id })
    })
    terminals.set(id, {
      pty,
      write: (data) => pty.write(data),
      resize: (cols, rows) => pty.resize(cols, rows),
      kill: () => pty.kill()
    })
    return { id, pty: true }
  }
  const child = spawn(executable, args, { cwd, env: process.env, stdio: ['pipe', 'pipe', 'pipe'] })
  child.stdout?.on('data', (chunk: Buffer) => send('terminal:data', { id, data: chunk.toString('utf8') }))
  child.stderr?.on('data', (chunk: Buffer) => send('terminal:data', { id, data: chunk.toString('utf8') }))
  child.on('exit', () => {
    terminals.delete(id)
    send('terminal:exit', { id })
  })
  terminals.set(id, {
    child,
    write: (data) => child.stdin?.write(data),
    resize: () => undefined,
    kill: () => child.kill()
  })
  return { id, pty: false }
}

function taskArgs(kind: string): string[] {
  switch (kind) {
    case 'build':
      return ['build']
    case 'run':
      return ['run']
    case 'check':
      return ['check']
    case 'test':
      return ['test']
    default:
      throw new Error('unsupported task')
  }
}

async function runTask(kind: string): Promise<TaskResult> {
  if (!workspaceRoot) return { code: 1, output: 'No workspace is open.' }
  runningTask?.kill()
  const executable = novaExecutable()
  return new Promise((resolve) => {
    const child = spawn(executable, taskArgs(kind), {
      cwd: workspaceRoot!,
      env: process.env
    })
    runningTask = child
    let output = ''
    const collect = (chunk: Buffer) => {
      output += chunk.toString('utf8')
      if (output.length > 400_000) output = output.slice(-400_000)
    }
    child.stdout?.on('data', collect)
    child.stderr?.on('data', collect)
    child.on('error', (error) => {
      runningTask = null
      resolve({ code: 127, output: `${output}\n${error.message}`.trim() })
    })
    child.on('exit', (code) => {
      runningTask = null
      resolve({ code: code ?? 1, output })
    })
  })
}

async function walkFiles(dir: string, prefix = '', out: string[] = []): Promise<string[]> {
  if (out.length > 5000) return out
  const entries = await fs.readdir(dir, { withFileTypes: true })
  for (const entry of entries) {
    if (IGNORED.has(entry.name) || entry.name.startsWith('.')) continue
    const relative = prefix ? `${prefix}/${entry.name}` : entry.name
    if (entry.isDirectory()) await walkFiles(path.join(dir, entry.name), relative, out)
    else out.push(relative)
    if (out.length > 5000) break
  }
  return out
}

async function runSearch(query: string, options: { regex: boolean; caseSensitive: boolean; wholeWord: boolean }): Promise<Array<{ file: string; line: number; column: number; text: string }>> {
  if (!workspaceRoot) return []
  const args = ['--line-number', '--column', '--no-heading', '--color', 'never', '--max-count', '200']
  if (!options.regex) args.push('--fixed-strings')
  if (options.caseSensitive) args.push('--case-sensitive')
  else args.push('--smart-case')
  if (options.wholeWord) args.push('--word-regexp')
  args.push(query, '.')
  return new Promise((resolve) => {
    execFile('rg', args, { cwd: workspaceRoot!, maxBuffer: 4_000_000 }, (error, stdout) => {
      if (error && !stdout) {
        resolve([])
        return
      }
      const results = stdout
        .split('\n')
        .filter(Boolean)
        .slice(0, 300)
        .map((line) => {
          const match = /^\.\/(.*?):(\d+):(\d+):(.*)$/.exec(line)
          if (!match) return null
          return {
            file: match[1],
            line: Number(match[2]),
            column: Number(match[3]),
            text: match[4]
          }
        })
        .filter((item): item is { file: string; line: number; column: number; text: string } => item !== null)
      resolve(results)
    })
  })
}

function buildMenu(): void {
  const command = (name: string) => () => send('menu:command', name)
  const template: Electron.MenuItemConstructorOptions[] = [
    {
      label: 'File',
      submenu: [
        { label: 'Open Folder…', accelerator: 'CmdOrCtrl+O', click: command('open-folder') },
        { label: 'New File', accelerator: 'CmdOrCtrl+N', click: command('new-file') },
        { label: 'Save', accelerator: 'CmdOrCtrl+S', click: command('save') },
        { type: 'separator' },
        { role: 'close' },
        { role: 'quit' }
      ]
    },
    {
      label: 'Edit',
      submenu: [
        { role: 'undo' },
        { role: 'redo' },
        { type: 'separator' },
        { role: 'cut' },
        { role: 'copy' },
        { role: 'paste' },
        { role: 'selectAll' },
        { type: 'separator' },
        { label: 'Find in Files', accelerator: 'CmdOrCtrl+Shift+F', click: command('search') },
        { label: 'Quick Open…', accelerator: 'CmdOrCtrl+P', click: command('quick-open') },
        { label: 'Command Palette…', accelerator: 'CmdOrCtrl+Shift+P', click: command('palette') }
      ]
    },
    {
      label: 'View',
      submenu: [
        { label: 'Toggle Sidebar', accelerator: 'CmdOrCtrl+B', click: command('toggle-sidebar') },
        { label: 'Toggle Terminal', accelerator: 'CmdOrCtrl+`', click: command('toggle-terminal') },
        { label: 'Toggle AI Panel', click: command('toggle-ai') },
        { label: 'Toggle Vim Mode', click: command('toggle-vim') },
        { label: 'Toggle Visual Effects', click: command('toggle-effects') },
        { label: 'Toggle Sound Effects', click: command('toggle-sound') },
        { label: 'Documentation', click: command('docs') },
        { label: 'Settings', click: command('settings') },
        { type: 'separator' },
        { role: 'resetZoom' },
        { role: 'zoomIn' },
        { role: 'zoomOut' },
        { type: 'separator' },
        { role: 'togglefullscreen' },
        { role: 'toggleDevTools' }
      ]
    },
    {
      label: 'Run',
      submenu: [
        { label: 'Build', accelerator: 'CmdOrCtrl+Shift+B', click: command('build') },
        { label: 'Run', accelerator: 'F5', click: command('run-task') },
        { label: 'Check', click: command('check') },
        { label: 'Test', click: command('test') },
        { label: 'Format Document', accelerator: 'Shift+Alt+F', click: command('format') },
        { label: 'Cancel Task', click: command('cancel-task') }
      ]
    },
    {
      label: 'Terminal',
      submenu: [
        { label: 'Toggle Terminal', click: command('toggle-terminal') },
        { label: 'New Shell', click: command('new-shell') },
        { label: 'FreeBuff Terminal', click: command('freebuff-terminal') }
      ]
    },
    {
      label: 'Help',
      submenu: [
        { label: 'Nova Documentation', click: () => shell.openExternal('https://jd4rider.github.io/nova/') },
        { label: 'GitHub Repository', click: () => shell.openExternal('https://github.com/jd4rider/nova') },
        { label: 'Report an Issue', click: () => shell.openExternal('https://github.com/jd4rider/nova/issues') }
      ]
    }
  ]
  Menu.setApplicationMenu(Menu.buildFromTemplate(template))
}


function createWindow(): void {
  if (process.platform === 'darwin' && app.dock && !app.isPackaged) {
    try {
      app.dock.setIcon(path.join(app.getAppPath(), 'resources/icon.png'))
    } catch {
      // The dock icon is cosmetic; a missing file must not stop the window.
    }
  }

  mainWindow = new BrowserWindow({
    width: 1440,
    height: 900,
    minWidth: 900,
    minHeight: 560,
    backgroundColor: '#1e1e1e',
    title: 'Nova Studio',
    ...(process.platform === 'darwin' ? { titleBarStyle: 'hiddenInset' as const, trafficLightPosition: { x: 14, y: 11 } } : {}),
    webPreferences: {
      preload: path.join(__dirname, '../preload/index.js'),
      contextIsolation: true,
      sandbox: true,
      nodeIntegration: false,
      webSecurity: true
    }
  })

  mainWindow.on('closed', () => { mainWindow = null })
  mainWindow.webContents.setWindowOpenHandler(() => ({ action: 'deny' }))
  mainWindow.webContents.on('will-navigate', (event, url) => {
    if (!url.startsWith('http://localhost') && !url.startsWith('file://')) event.preventDefault()
  })

  if (process.env.ELECTRON_RENDERER_URL) {
    mainWindow.loadURL(process.env.ELECTRON_RENDERER_URL)
  } else {
    mainWindow.loadFile(path.join(__dirname, '../renderer/index.html'))
  }
}

function registerIpc(): void {
  ipcMain.handle('workspace:choose', async () => chooseWorkspace())
  ipcMain.handle('workspace:restore', async () => restoreWorkspace())
  ipcMain.handle('workspace:root', async () => workspaceRoot)

  ipcMain.handle('fs:list', async (event, dir: string): Promise<FileEntry[]> => {
    guard(event)
    const absolute = await safeExisting(dir)
    const entries = await fs.readdir(absolute, { withFileTypes: true })
    return entries
      .filter((entry) => !IGNORED.has(entry.name))
      .map((entry) => ({
        name: entry.name,
        path: path.join(absolute, entry.name),
        isDir: entry.isDirectory()
      }))
      .sort((a, b) => Number(b.isDir) - Number(a.isDir) || a.name.localeCompare(b.name))
  })

  ipcMain.handle('fs:read', async (event, file: string) => {
    guard(event)
    const absolute = await safeExisting(file)
    const [content, stat] = await Promise.all([fs.readFile(absolute, 'utf8'), fs.stat(absolute)])
    return { content, mtimeMs: stat.mtimeMs }
  })

  ipcMain.handle('fs:write', async (event, file: string, content: string, expectedMtimeMs: number | null) => {
    guard(event)
    const absolute = await safeExisting(file)
    if (expectedMtimeMs !== null) {
      const stat = await fs.stat(absolute)
      if (Math.abs(stat.mtimeMs - expectedMtimeMs) > 1) {
        return { mtimeMs: stat.mtimeMs, conflict: true }
      }
    }
    const temporary = path.join(path.dirname(absolute), `.${path.basename(absolute)}.nova-tmp-${process.pid}`)
    await fs.writeFile(temporary, content, 'utf8')
    await fs.rename(temporary, absolute)
    const stat = await fs.stat(absolute)
    return { mtimeMs: stat.mtimeMs }
  })

  ipcMain.handle('fs:create', async (event, file: string, content: string) => {
    guard(event)
    const absolute = await safeNew(file)
    await fs.writeFile(absolute, content, { encoding: 'utf8', flag: 'wx' })
  })

  ipcMain.handle('fs:mkdir', async (event, dir: string) => {
    guard(event)
    const absolute = await safeNew(dir)
    await fs.mkdir(absolute)
  })

  ipcMain.handle('fs:rename', async (event, from: string, to: string) => {
    guard(event)
    const source = await safeExisting(from)
    const destination = await safeNew(to)
    await fs.rename(source, destination)
  })

  ipcMain.handle('fs:remove', async (event, target: string) => {
    guard(event)
    const absolute = await safeExisting(target)
    const stat = await fs.stat(absolute)
    if (stat.isDirectory()) await fs.rm(absolute, { recursive: true })
    else await fs.unlink(absolute)
  })

  ipcMain.handle('fs:reveal', async (event, target: string) => {
    guard(event)
    const absolute = await safeExisting(target)
    shell.showItemInFolder(absolute)
  })

  ipcMain.handle('fs:files', async (event) => {
    guard(event)
    if (!workspaceRoot) return []
    return walkFiles(workspaceRoot)
  })

  ipcMain.handle('search:run', async (event, query: string, options: { regex: boolean; caseSensitive: boolean; wholeWord: boolean }) => {
    guard(event)
    return runSearch(query, options)
  })

  ipcMain.handle('task:run', async (event, kind: string) => {
    guard(event)
    return runTask(kind)
  })

  ipcMain.handle('task:cancel', async (event) => {
    guard(event)
    runningTask?.kill()
    runningTask = null
  })

  ipcMain.handle('terminal:create', async (event, cwd: string, cols: number, rows: number, command?: string) => {
    guard(event)
    if (!workspaceRoot) throw new Error('no workspace is open')
    const absolute = await safeExisting(cwd)
    return createTerminal(absolute, cols, rows, command)
  })

  ipcMain.handle('terminal:write', async (event, id: number, data: string) => {
    guard(event)
    terminals.get(id)?.write(data)
  })

  ipcMain.handle('terminal:resize', async (event, id: number, cols: number, rows: number) => {
    guard(event)
    terminals.get(id)?.resize(cols, rows)
  })

  ipcMain.handle('terminal:dispose', async (event, id: number) => {
    guard(event)
    terminals.get(id)?.kill()
    terminals.delete(id)
  })

  ipcMain.handle('lsp:start', async (event, root: string) => {
    guard(event)
    if (!workspaceRoot) return { ok: false, error: 'no workspace is open' }
    lspClient?.stop()
    lspClient = new LspClient(
      (uri: string, diagnostics: LspDiagnostic[]) => send('lsp:diagnostics', { uri, diagnostics }),
      (status: string) => send('lsp:status', status)
    )
    try {
      await lspClient.start(workspaceRoot)
      return { ok: true }
    } catch (error) {
      return { ok: false, error: (error as Error).message }
    }
  })

  ipcMain.handle('lsp:stop', async (event) => {
    guard(event)
    lspClient?.stop()
    lspClient = null
  })

  ipcMain.handle('lsp:request', async (event, method: string, params: unknown) => {
    guard(event)
    if (!lspClient) throw new Error('language server is not ready')
    return lspClient.request(method, params)
  })

  ipcMain.handle('lsp:notify', async (event, method: string, params: unknown) => {
    guard(event)
    lspClient?.notify(method, params)
  })

  ipcMain.handle('freebuff:status', async (event) => {
    guard(event)
    return freebuffStatus()
  })

  ipcMain.handle('freebuff:start', async (event, root: string) => {
    guard(event)
    const absolute = await safeExisting(root)
    freebuffChat.start(absolute)
  })

  ipcMain.handle('freebuff:send', async (event, text: string) => {
    guard(event)
    if (typeof text !== 'string' || !text.trim() || text.length > 40_000) {
      throw new Error('invalid FreeBuff message')
    }
    freebuffChat.send(text)
  })

  ipcMain.handle('freebuff:reset', async (event) => {
    guard(event)
    freebuffChat.reset()
  })

  ipcMain.handle('freebuff:stop', async (event) => {
    guard(event)
    freebuffChat.stop()
  })

  ipcMain.handle('freebuff:complete', async (event, input: { uri: string; version: number; text: string; line: number; character: number }) => {
    guard(event)
    if (!workspaceRoot || !input || typeof input.text !== 'string' || input.text.length > 120_000) return null
    return freebuffCompletion.complete({
      uri: String(input.uri ?? ''),
      text: input.text,
      line: Math.max(0, Number(input.line) || 0),
      character: Math.max(0, Number(input.character) || 0),
      root: workspaceRoot
    })
  })

  ipcMain.handle('copilot:status', async (event) => {
    guard(event)
    await copilot.start(workspaceRoot)
    return copilot.status()
  })

  ipcMain.handle('copilot:sign-in', async (event) => {
    guard(event)
    return copilot.signIn()
  })

  ipcMain.handle('copilot:sign-out', async (event) => {
    guard(event)
    return copilot.signOut()
  })

  ipcMain.handle('copilot:models', async (event) => {
    guard(event)
    return copilot.models()
  })

  ipcMain.handle('copilot:complete', async (event, input: { uri: string; version: number; text: string; line: number; character: number }) => {
    guard(event)
    if (!input || typeof input.uri !== 'string' || typeof input.text !== 'string' || input.text.length > 300_000) {
      return null
    }
    return copilotCompletions.complete({
      uri: input.uri,
      version: Number(input.version) || 1,
      text: input.text,
      line: Math.max(0, Number(input.line) || 0),
      character: Math.max(0, Number(input.character) || 0)
    })
  })

  ipcMain.handle('ai:settings', async (event) => {
    guard(event)
    return getAiSettings()
  })

  ipcMain.handle('ai:configure', async (event, patch: AiConfigurePatch) => {
    guard(event)
    return configureAi(patch)
  })

  ipcMain.handle('ai:send', async (event, requestId: string, messages: AiMessage[], context: AiContext | null) => {
    guard(event)
    if (typeof requestId !== 'string' || !requestId || requestId.length > 100) {
      throw new Error('invalid request id')
    }
    if (!Array.isArray(messages)) throw new Error('invalid message list')
    const clean: AiMessage[] = messages
      .slice(-24)
      .map((message): AiMessage => ({
        role: message?.role === 'assistant' ? 'assistant' : 'user',
        content: String(message?.content ?? '').slice(0, 40_000)
      }))
      .filter((message) => message.content.trim().length > 0)
    if (clean.length === 0) throw new Error('nothing to send')

    aiRequests.get(requestId)?.abort()
    const controller = new AbortController()
    aiRequests.set(requestId, controller)
    const safeContext: AiContext | null =
      context && typeof context === 'object'
        ? {
            path: String(context.path ?? ''),
            language: String(context.language ?? 'nova'),
            content: String(context.content ?? ''),
            selection: typeof context.selection === 'string' ? context.selection : undefined
          }
        : null

    void (async () => {
      try {
        const config = resolveAiConfig()
        send('ai:event', { requestId, type: 'start', model: config.model })
        if (config.provider === 'copilot') {
          await copilot.chat(clean, safeContext, config.model, (text) =>
            send('ai:event', { requestId, type: 'delta', text }), controller.signal
          )
          send('ai:event', { requestId, type: 'done', model: config.model })
        } else {
          const result = await streamChat(config, systemPrompt(safeContext), clean, controller.signal, (text) =>
            send('ai:event', { requestId, type: 'delta', text })
          )
          send('ai:event', { requestId, type: 'done', model: result.model })
        }
      } catch (error) {
        send('ai:event', {
          requestId,
          type: 'error',
          message: isAbortError(error) ? 'Stopped.' : (error as Error).message
        })
      } finally {
        aiRequests.delete(requestId)
      }
    })()
  })

  ipcMain.handle('ai:cancel', async (event, requestId: string) => {
    guard(event)
    aiRequests.get(requestId)?.abort()
    aiRequests.delete(requestId)
  })

  ipcMain.handle('app:version', async () => app.getVersion())
}

app.whenReady().then(async () => {
  session.defaultSession.webRequest.onHeadersReceived((details, callback) => {
    callback({
      responseHeaders: {
        ...details.responseHeaders,
        'Content-Security-Policy': [
          "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; worker-src 'self' blob:; img-src 'self' data:; font-src 'self' data:; connect-src 'self' ws: http://localhost:*"
        ]
      }
    })
  })
  registerIpc()
  buildMenu()
  await restoreWorkspace()
  createWindow()
  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) createWindow()
  })
})

app.on('window-all-closed', () => {
  lspClient?.stop()
  copilot.stop()
  copilotCompletions.stop()
  freebuffChat.stop()
  for (const [, terminal] of terminals) terminal.kill()
  for (const [, controller] of aiRequests) controller.abort()
  if (process.platform !== 'darwin') app.quit()
})
