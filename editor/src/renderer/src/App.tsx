import { useCallback, useEffect, useRef, useState } from 'react'
import type * as Monaco from 'monaco-editor'
import { Terminal } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import { monaco, registerCopilotCompletion, registerLspProviders } from './monaco'
import { MaterialFileIcon } from './MaterialFileIcon'
import { MONO_FONT } from './fonts'
import { AiPanel } from './AiPanel'
import { EffectsLayer, type EffectsHandle } from './effects'
import { playScroll, playTyping } from './sfx'
import type {
  FileEntry,
  FreeBuffStatus,
  LspDiagnostic,
  ThemeName
} from '../../shared/types'

interface Tab {
  path: string
  name: string
  uri: string
  model: Monaco.editor.ITextModel
  mtimeMs: number
  dirty: boolean
}

interface Problem {
  file: string
  uri: string
  line: number
  column: number
  message: string
  severity: number
}

interface SearchHit {
  file: string
  line: number
  column: number
  text: string
}

interface Settings {
  theme: ThemeName
  fontSize: number
  vim: boolean
  minimap: boolean
  effects: boolean
  sound: boolean
  volume: number
}

const defaultSettings: Settings = {
  theme: 'vs-dark',
  fontSize: 13,
  vim: false,
  minimap: true,
  effects: true,
  sound: false,
  volume: 25
}
let lspProvidersRegistered = false
const pathToUri = (path: string) => 'file://' + encodeURI(path)
const uriToPath = (uri: string) => decodeURI(uri.replace('file://', ''))
const joinPath = (dir: string, name: string) => `${dir}/${name}`

function loadSettings(): Settings {
  try {
    const stored = JSON.parse(localStorage.getItem('nova.settings') ?? '{}')
    if (stored.version !== 2) {
      if (!stored.theme || stored.theme === 'gruvbox-dark') stored.theme = 'vs-dark'
      stored.version = 2
    }
    return { ...defaultSettings, ...stored }
  } catch {
    return defaultSettings
  }
}

export default function App(): JSX.Element {
  const [root, setRoot] = useState<string | null>(null)
  const [hasManifest, setHasManifest] = useState(false)
  const [tabs, setTabs] = useState<Tab[]>([])
  const tabsRef = useRef<Tab[]>([])
  const [activePath, setActivePath] = useState<string | null>(null)
  const activeRef = useRef<string | null>(null)
  const [editor, setEditor] = useState<Monaco.editor.IStandaloneCodeEditor | null>(null)
  const editorRef = useRef<Monaco.editor.IStandaloneCodeEditor | null>(null)
  const [cursor, setCursor] = useState({ line: 1, column: 1 })
  const [sidebar, setSidebar] = useState<'explorer' | 'search'>('explorer')
  const [sidebarVisible, setSidebarVisible] = useState(true)
  const [bottomTab, setBottomTab] = useState<'problems' | 'output' | 'terminal' | 'tests' | 'docs'>('problems')
  const [bottomVisible, setBottomVisible] = useState(true)
  const [problems, setProblems] = useState<Problem[]>([])
  const [output, setOutput] = useState('Ready. Open a folder to start.\n')
  const [testsOutput, setTestsOutput] = useState('No test run yet.\n')
  const [lspStatus, setLspStatus] = useState('idle')
  const [freebuff, setFreebuff] = useState<FreeBuffStatus | null>(null)
  const [aiVisible, setAiVisible] = useState(true)
  const [aiLabel, setAiLabel] = useState('not configured')
  const [settings, setSettings] = useState<Settings>(loadSettings)
  const reducedMotion = usePrefersReducedMotion()
  const effectsActive = settings.effects && !reducedMotion
  const [overlay, setOverlay] = useState<'quick' | 'palette' | 'settings' | null>(null)
  const [createDialog, setCreateDialog] = useState<'file' | 'folder' | null>(null)
  const [lineDialog, setLineDialog] = useState(false)
  const [tabMenu, setTabMenu] = useState<{ x: number; y: number; path: string } | null>(null)
  const [refreshKey, setRefreshKey] = useState(0)
  const [terminalCommand, setTerminalCommand] = useState<string>('shell')
  const [terminalPty, setTerminalPty] = useState<boolean | null>(null)
  const [busy, setBusy] = useState<string | null>(null)
  const diagnosticsRef = useRef<Map<string, LspDiagnostic[]>>(new Map())
  const effectsRef = useRef<EffectsHandle | null>(null)
  const vimStatusRef = useRef<HTMLSpanElement | null>(null)
  const vimRef = useRef<{ dispose: () => void } | null>(null)
  const changeTimers = useRef<Map<string, ReturnType<typeof setTimeout>>>(new Map())
  const saveRef = useRef<() => void>(() => undefined)
  const closeRef = useRef<() => void>(() => undefined)

  const updateTabs = useCallback((updater: (tabs: Tab[]) => Tab[]) => {
    setTabs((previous) => {
      const next = updater(previous)
      tabsRef.current = next
      return next
    })
  }, [])

  const setActive = useCallback((path: string | null) => {
    activeRef.current = path
    setActivePath(path)
  }, [])

  const applyDiagnostics = useCallback((uri: string, diagnostics: LspDiagnostic[]) => {
    diagnosticsRef.current.set(uri, diagnostics)
    const model = monaco.editor.getModel(monaco.Uri.parse(uri))
    if (model) {
      monaco.editor.setModelMarkers(
        model,
        'nova',
        diagnostics.map((diagnostic) => ({
          severity:
            diagnostic.severity === 1
              ? monaco.MarkerSeverity.Error
              : diagnostic.severity === 2
                ? monaco.MarkerSeverity.Warning
                : monaco.MarkerSeverity.Info,
          message: diagnostic.message,
          code: diagnostic.code !== undefined ? String(diagnostic.code) : undefined,
          source: 'nova',
          startLineNumber: diagnostic.range.start.line + 1,
          startColumn: diagnostic.range.start.character + 1,
          endLineNumber: diagnostic.range.end.line + 1,
          endColumn: diagnostic.range.end.character + 1
        }))
      )
    }
    const file = uriToPath(uri)
    setProblems((previous) => {
      const others = previous.filter((problem) => problem.file !== file)
      return [
        ...others,
        ...diagnostics.map((diagnostic) => ({
          file,
          uri,
          line: diagnostic.range.start.line + 1,
          column: diagnostic.range.start.character + 1,
          message: diagnostic.message,
          severity: diagnostic.severity ?? 1
        }))
      ]
    })
  }, [])

  const openPath = useCallback(
    async (path: string, line?: number) => {
      if (!root) return
      const existing = tabsRef.current.find((tab) => tab.path === path)
      if (existing) {
        setActive(path)
        if (line) {
          editorRef.current?.setPosition({ lineNumber: line, column: 1 })
          editorRef.current?.revealLineInCenter(line)
        }
        return
      }
      try {
        const { content, mtimeMs } = await window.nova.fs.read(path)
        const uri = pathToUri(path)
        const model = monaco.editor.getModel(monaco.Uri.parse(uri)) ?? monaco.editor.createModel(content, 'nova', monaco.Uri.parse(uri))
        const tab: Tab = {
          path,
          name: path.split('/').pop() ?? path,
          uri,
          model,
          mtimeMs,
          dirty: false
        }
        updateTabs((previous) => [...previous, tab])
        setActive(path)
        window.nova.lsp.notify('textDocument/didOpen', {
          textDocument: { uri, languageId: 'nova', version: 1, text: content }
        })
        const pendingDiagnostics = diagnosticsRef.current.get(uri)
        if (pendingDiagnostics) applyDiagnostics(uri, pendingDiagnostics)
        if (line) {
          editorRef.current?.setPosition({ lineNumber: line, column: 1 })
          editorRef.current?.revealLineInCenter(line)
        }
      } catch (error) {
        setOutput(`Could not open ${path}: ${(error as Error).message}\n`)
        setBottomVisible(true)
        setBottomTab('output')
      }
    },
    [applyDiagnostics, root, setActive, updateTabs]
  )

  const saveActive = useCallback(async () => {
    const path = activeRef.current
    const tab = tabsRef.current.find((candidate) => candidate.path === path)
    if (!tab) return
    try {
      let result = await window.nova.fs.write(tab.path, tab.model.getValue(), tab.mtimeMs)
      if (result.conflict) {
        if (!window.confirm('This file changed on disk. Overwrite the external version?')) return
        result = await window.nova.fs.write(tab.path, tab.model.getValue(), null)
      }
      updateTabs((previous) => previous.map((candidate) => (candidate.path === tab.path ? { ...candidate, mtimeMs: result.mtimeMs, dirty: false } : candidate)))
      window.nova.lsp.notify('textDocument/didSave', { textDocument: { uri: tab.uri } })
      setOutput((previous) => `${previous}\nSaved ${tab.path}\n`)
    } catch (error) {
      window.alert(`Save failed: ${(error as Error).message}`)
    }
  }, [updateTabs])

  const saveAll = useCallback(async () => {
    const dirty = tabsRef.current.filter((tab) => tab.dirty)
    for (const tab of dirty) {
      try {
        const result = await window.nova.fs.write(tab.path, tab.model.getValue(), tab.mtimeMs)
        if (result.conflict) {
          if (!window.confirm(`${tab.name} changed on disk. Overwrite the external version?`)) continue
          await window.nova.fs.write(tab.path, tab.model.getValue(), null)
        }
        updateTabs((previous) => previous.map((candidate) => (candidate.path === tab.path ? { ...candidate, dirty: false } : candidate)))
        window.nova.lsp.notify('textDocument/didSave', { textDocument: { uri: tab.uri } })
      } catch (error) {
        window.alert(`Could not save ${tab.name}: ${(error as Error).message}`)
      }
    }
  }, [updateTabs])

  const closeTabs = useCallback(
    (paths: string[]) => {
      const targets = tabsRef.current.filter((tab) => paths.includes(tab.path))
      if (targets.length === 0) return
      const dirty = targets.filter((tab) => tab.dirty)
      if (dirty.length > 0 && !window.confirm(`Discard unsaved changes in ${dirty.length} file(s)?`)) return
      for (const tab of targets) {
        window.nova.lsp.notify('textDocument/didClose', { textDocument: { uri: tab.uri } })
        tab.model.dispose()
        diagnosticsRef.current.delete(tab.uri)
      }
      const remaining = tabsRef.current.filter((tab) => !paths.includes(tab.path))
      updateTabs(() => remaining)
      if (activeRef.current && paths.includes(activeRef.current)) {
        setActive(remaining.length ? remaining[remaining.length - 1].path : null)
      }
    },
    [setActive, updateTabs]
  )

  const closeTab = useCallback(
    (path: string) => {
      const tab = tabsRef.current.find((candidate) => candidate.path === path)
      if (!tab) return
      if (tab.dirty && !window.confirm(`Discard unsaved changes in ${tab.name}?`)) return
      window.nova.lsp.notify('textDocument/didClose', { textDocument: { uri: tab.uri } })
      tab.model.dispose()
      diagnosticsRef.current.delete(tab.uri)
      const remaining = tabsRef.current.filter((candidate) => candidate.path !== tab.path)
      updateTabs(() => remaining)
      if (activeRef.current === path) setActive(remaining.length ? remaining[remaining.length - 1].path : null)
    },
    [setActive, updateTabs]
  )

  useEffect(() => {
    saveRef.current = () => void saveActive()
    closeRef.current = () => closeTab(activeRef.current ?? '')
  }, [saveActive, closeTab])

  // Re-read clean tabs when the workspace watcher reports external edits. Dirty
  // buffers are left alone so unsaved work is never replaced.
  const reloadFromDisk = useCallback(
    async (paths: string[]) => {
      for (const tab of tabsRef.current.filter((candidate) => paths.includes(candidate.path))) {
        try {
          const { content, mtimeMs } = await window.nova.fs.read(tab.path)
          // The tab may have been closed while the read was in flight.
          if (!tabsRef.current.some((candidate) => candidate.model === tab.model)) continue
          if (tab.dirty) continue
          if (content === tab.model.getValue()) {
            updateTabs((previous) => previous.map((candidate) => (candidate.path === tab.path ? { ...candidate, mtimeMs } : candidate)))
            continue
          }
          const position = activeRef.current === tab.path ? editorRef.current?.getPosition() : null
          tab.model.setValue(content)
          if (position && activeRef.current === tab.path) editorRef.current?.setPosition(position)
          updateTabs((previous) => previous.map((candidate) => (candidate.path === tab.path ? { ...candidate, mtimeMs, dirty: false } : candidate)))
          window.nova.lsp.notify('textDocument/didChange', {
            textDocument: { uri: tab.uri, version: tab.model.getVersionId() },
            contentChanges: [{ text: content }]
          })
        } catch {
          // Removed or unreadable: keep the buffer so unsaved text is never lost.
        }
      }
    },
    [updateTabs]
  )

  useEffect(
    () =>
      window.nova.workspace.onChanged(({ paths }) => {
        setRefreshKey((value) => value + 1)
        if (paths.length > 0) void reloadFromDisk(paths)
      }),
    [reloadFromDisk]
  )

  const goToLine = useCallback((raw: string) => {
    const line = Number.parseInt(raw, 10)
    const model = editorRef.current?.getModel()
    if (!Number.isFinite(line) || !model) return
    const target = Math.min(Math.max(1, line), model.getLineCount())
    editorRef.current?.setPosition({ lineNumber: target, column: 1 })
    editorRef.current?.revealLineInCenter(target)
    editorRef.current?.focus()
  }, [])

  useEffect(() => {
    document.documentElement.dataset.theme = settings.theme
    monaco.editor.setTheme(settings.theme)
    localStorage.setItem('nova.settings', JSON.stringify(settings))
  }, [settings])

  useEffect(() => {
    if (!editor) return
    editor.updateOptions({ fontSize: settings.fontSize, minimap: { enabled: settings.minimap } })
  }, [editor, settings.fontSize, settings.minimap])

  useEffect(() => {
    if (!editor) return
    let disposed = false
    if (settings.vim && vimStatusRef.current) {
      import('monaco-vim')
        .then((module: any) => {
          if (disposed) return
          module.Vim.defineEx('write', 'w', () => saveRef.current())
          module.Vim.defineEx('quit', 'q', () => closeRef.current())
          module.Vim.defineEx('wq', 'wq', () => {
            saveRef.current()
            closeRef.current()
          })
          let lineNumbersOn = true
          module.Vim.defineEx('set', 'set', (_cm: unknown, params: string) => {
            if (params.includes('number') && editorRef.current) {
              lineNumbersOn = !lineNumbersOn
              editorRef.current.updateOptions({ lineNumbers: lineNumbersOn ? 'on' : 'off' })
            }
          })
          vimRef.current = module.initVimMode(editor, vimStatusRef.current!)
        })
        .catch(() => {
          if (vimStatusRef.current) vimStatusRef.current.textContent = 'VIM ERROR'
        })
    }
    return () => {
      disposed = true
      vimRef.current?.dispose()
      vimRef.current = null
    }
  }, [editor, settings.vim])

  useEffect(() => {
    if (!lspProvidersRegistered) {
      lspProvidersRegistered = true
      registerLspProviders((method, params) => window.nova.lsp.request(method, params))
      registerCopilotCompletion()
    }
    const offDiagnostics = window.nova.lsp.onDiagnostics(({ uri, diagnostics }) => applyDiagnostics(uri, diagnostics))
    const offStatus = window.nova.lsp.onStatus((status) => setLspStatus(status))
    return () => {
      offDiagnostics()
      offStatus()
    }
  }, [applyDiagnostics])

  useEffect(() => {
    if (!editor) return
    const disposable = editor.onDidChangeCursorPosition((event) => {
      setCursor({ line: event.position.lineNumber, column: event.position.column })
    })
    return () => disposable.dispose()
  }, [editor])

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      const modifier = event.metaKey || event.ctrlKey
      const key = event.key.toLowerCase()
      if (modifier && key === 'p') {
        event.preventDefault()
        event.stopPropagation()
        setOverlay(event.shiftKey ? 'palette' : 'quick')
      } else if (modifier && key === 's') {
        event.preventDefault()
        if (event.altKey) void saveAll()
        else void saveActive()
      } else if (modifier && key === 'b') {
        event.preventDefault()
        setSidebarVisible((value) => !value)
      } else if (modifier && key === 'j') {
        event.preventDefault()
        setBottomVisible((value) => !value)
      } else if (modifier && key === 'w') {
        event.preventDefault()
        if (activeRef.current) closeTab(activeRef.current)
      } else if (modifier && event.shiftKey && key === 'e') {
        event.preventDefault()
        setSidebar('explorer')
        setSidebarVisible(true)
      } else if (modifier && event.shiftKey && key === 'f') {
        event.preventDefault()
        setSidebar('search')
        setSidebarVisible(true)
      } else if (event.ctrlKey && !event.metaKey && key === 'g') {
        event.preventDefault()
        setLineDialog(true)
      } else if (modifier && event.key === '`') {
        event.preventDefault()
        setBottomVisible((value) => !value)
      } else if (event.key === 'Escape' && overlay) {
        setOverlay(null)
      }
    }
    window.addEventListener('keydown', onKey, true)
    return () => window.removeEventListener('keydown', onKey, true)
  }, [overlay, saveActive, saveAll, closeTab])

  const runTask = useCallback(async (kind: 'build' | 'run' | 'check' | 'test') => {
    if (!root) {
      setOutput('Open a folder first.\n')
      setBottomVisible(true)
      setBottomTab('output')
      return
    }
    setBusy(kind)
    setBottomVisible(true)
    setBottomTab(kind === 'test' ? 'tests' : 'output')
    if (kind === 'test') setTestsOutput(`$ nova test\n\n`)
    else setOutput(`$ nova ${kind}\n\n`)
    try {
      const result = await window.nova.tasks.run(kind)
      const text = `$ nova ${kind}\n\n${result.output || '(no output)'}\n\nExit code: ${result.code}\n`
      if (kind === 'test') setTestsOutput(text)
      else setOutput(text)
      if (kind !== 'test') setBottomTab('output')
    } catch (error) {
      const text = `Task failed: ${(error as Error).message}\n`
      if (kind === 'test') setTestsOutput(text)
      else setOutput(text)
    } finally {
      setBusy(null)
    }
  }, [root])

  const createEntry = useCallback(
    async (kind: 'file' | 'folder', rawName: string) => {
      if (!root) return
      const name = rawName.trim().replace(/^\.\//, '')
      if (!name) return
      const path = joinPath(root, name)
      try {
        if (kind === 'folder') await window.nova.fs.mkdir(path)
        else await window.nova.fs.create(path, '')
        setRefreshKey((value) => value + 1)
        if (kind === 'file') void openPath(path)
      } catch (error) {
        window.alert(`Could not create ${name}: ${(error as Error).message}`)
      }
    },
    [openPath, root]
  )

  const enterWorkspace = useCallback(async (result: { root: string; hasManifest: boolean }) => {
    for (const tab of tabsRef.current) tab.model.dispose()
    tabsRef.current = []
    setTabs([])
    setActive(null)
    setProblems([])
    diagnosticsRef.current.clear()
    setRoot(result.root)
    setHasManifest(result.hasManifest)
    setRefreshKey((value) => value + 1)
    if (result.hasManifest) {
      const status = await window.nova.lsp.start(result.root)
      setLspStatus(status.ok ? 'ready' : `error: ${status.error ?? 'unknown'}`)
    } else {
      setLspStatus('no nova.toml')
    }
    setFreebuff(await window.nova.freebuff.status())
    setOutput(`Workspace: ${result.root}\n`)
  }, [setActive])

  const openWorkspace = useCallback(async () => {
    const result = await window.nova.workspace.choose()
    if (result) await enterWorkspace(result)
  }, [enterWorkspace])

  // Reopen the folder from the previous session. The main process restores its
  // root before the window loads; asking for the resolved state lets the
  // renderer start the LSP and repaint the explorer.
  useEffect(() => {
    let cancelled = false
    void (async () => {
      try {
        const restored = await window.nova.workspace.restore()
        if (!cancelled && restored) await enterWorkspace(restored)
      } catch {
        // A missing or unreadable last workspace must not block startup.
      }
    })()
    return () => {
      cancelled = true
    }
  }, [enterWorkspace])

  const handleMenu = useCallback(
    (command: string) => {
      switch (command) {
        case 'open-folder':
          void openWorkspace()
          break
        case 'save':
          void saveActive()
          break
        case 'quick-open':
          setOverlay('quick')
          break
        case 'palette':
          setOverlay('palette')
          break
        case 'settings':
          setOverlay('settings')
          break
        case 'toggle-sidebar':
          setSidebarVisible((value) => !value)
          break
        case 'toggle-terminal':
          setBottomVisible((value) => !value)
          break
        case 'toggle-ai':
          setAiVisible((value) => !value)
          break
        case 'toggle-vim':
          setSettings((value) => ({ ...value, vim: !value.vim }))
          break
        case 'toggle-effects':
          setSettings((value) => ({ ...value, effects: !value.effects }))
          break
        case 'toggle-sound':
          setSettings((value) => ({ ...value, sound: !value.sound }))
          break
        case 'docs':
          setBottomVisible(true)
          setBottomTab('docs')
          break
        case 'build':
          void runTask('build')
          break
        case 'run-task':
          void runTask('run')
          break
        case 'check':
          void runTask('check')
          break
        case 'test':
          void runTask('test')
          break
        case 'format':
          void editorRef.current?.getAction('editor.action.formatDocument')?.run()
          break
        case 'cancel-task':
          void window.nova.tasks.cancel()
          break
        case 'new-file':
          if (root) setCreateDialog('file')
          break
        case 'new-folder':
          if (root) setCreateDialog('folder')
          break
        case 'new-shell':
          setTerminalCommand('shell')
          setBottomVisible(true)
          setBottomTab('terminal')
          break
        case 'freebuff-terminal':
          setTerminalCommand('freebuff')
          setBottomVisible(true)
          setBottomTab('terminal')
          break
        case 'search':
          setSidebar('search')
          setSidebarVisible(true)
          break
        default:
          break
      }
    },
    [openPath, openWorkspace, root, runTask, saveActive]
  )

  useEffect(() => window.nova.app.onMenu(handleMenu), [handleMenu])

  useEffect(() => {
    if (!editor) return
    let lastFeedback = 0
    const disposable = editor.onDidChangeModelContent(() => {
      const model = editor.getModel()
      if (!model) return
      const uri = model.uri.toString()
      updateTabs((previous) => previous.map((tab) => (tab.uri === uri ? { ...tab, dirty: true } : tab)))
      const existing = changeTimers.current.get(uri)
      if (existing) clearTimeout(existing)
      changeTimers.current.set(
        uri,
        setTimeout(() => {
          window.nova.lsp.notify('textDocument/didChange', {
            textDocument: { uri, version: model.getVersionId() },
            contentChanges: [{ text: model.getValue() }]
          })
        }, 180)
      )

      const now = Date.now()
      if (now - lastFeedback > 38) {
        lastFeedback = now
        if (effectsActive) {
          const position = editor.getPosition()
          const visible = position ? editor.getScrolledVisiblePosition(position) : null
          if (visible) {
            effectsRef.current?.burst(visible.left + 10, visible.top + visible.height / 2, 4)
          }
        }
        if (settings.sound) playTyping(settings.volume)
      }
    })
    return () => disposable.dispose()
  }, [editor, updateTabs, effectsActive, settings.sound, settings.volume])

  useEffect(() => {
    if (!editor) return
    let lastScrollFeedback = 0
    const disposable = editor.onDidScrollChange((event) => {
      if (!event.scrollTopChanged && !event.scrollLeftChanged) return
      if (!editor.getModel()) return
      const now = Date.now()
      if (now - lastScrollFeedback < 90) return
      lastScrollFeedback = now
      if (effectsActive) {
        const layout = editor.getLayoutInfo()
        effectsRef.current?.burst(
          layout.width * (0.58 + Math.random() * 0.3),
          30 + Math.random() * Math.max(30, layout.height - 60),
          3,
          'scroll'
        )
      }
      if (settings.sound) playScroll(settings.volume)
    })
    return () => disposable.dispose()
  }, [editor, effectsActive, settings.sound, settings.volume])

  useEffect(() => {
    if (!editor) return
    let decorations: string[] = []
    const apply = () => {
      const position = editor.getPosition()
      if (!effectsActive || !position) {
        decorations = editor.deltaDecorations(decorations, [])
        return
      }
      decorations = editor.deltaDecorations(decorations, [
        {
          range: new monaco.Range(position.lineNumber, 1, position.lineNumber, 1),
          options: {
            isWholeLine: true,
            className: 'nova-line-lens',
            lineNumberClassName: 'nova-line-lens-number',
            zIndex: 5
          }
        }
      ])
    }
    apply()
    const cursor = editor.onDidChangeCursorPosition(apply)
    const model = editor.onDidChangeModel(apply)
    return () => {
      cursor.dispose()
      model.dispose()
      editor.deltaDecorations(decorations, [])
    }
  }, [editor, effectsActive])

  const keepAiEdit = useCallback(
    async (path: string, content: string) => {
      const tab = tabsRef.current.find((candidate) => candidate.path === path)
      if (!tab) throw new Error('The file is not open. Open it and review the edit again.')
      const result = await window.nova.fs.write(path, content, tab.mtimeMs)
      if (result.conflict) {
        throw new Error('The file changed on disk. Reopen it and review the edit again.')
      }
      tab.model.setValue(content)
      updateTabs((previous) => previous.map((candidate) =>
        candidate.path === path ? { ...candidate, mtimeMs: result.mtimeMs, dirty: false } : candidate
      ))
      window.nova.lsp.notify('textDocument/didChange', {
        textDocument: { uri: tab.uri, version: tab.model.getVersionId() },
        contentChanges: [{ text: content }]
      })
      window.nova.lsp.notify('textDocument/didSave', { textDocument: { uri: tab.uri } })
      setOutput((previous) => `${previous}\nKept AI edit in ${path}\n`)
    },
    [updateTabs]
  )

  const activeTab = tabs.find((tab) => tab.path === activePath) ?? null
  const activeModel = activeTab?.model ?? null

  return (
    <div className="app">
      <TitleBar
        sidebar={sidebarVisible}
        panel={bottomVisible}
        ai={aiVisible}
        onPalette={() => setOverlay('palette')}
        onToggleSidebar={() => setSidebarVisible((value) => !value)}
        onTogglePanel={() => setBottomVisible((value) => !value)}
        onToggleAi={() => setAiVisible((value) => !value)}
      />
      <div className="workbench">
        <ActivityBar view={sidebar} visible={sidebarVisible} aiVisible={aiVisible} onView={(view) => { setSidebar(view); setSidebarVisible(true) }} onToggleAi={() => setAiVisible((value) => !value)} onSettings={() => setOverlay('settings')} />
        {sidebarVisible && (
          <aside className="sidebar">
            {sidebar === 'explorer' ? (
              <Explorer
                root={root}
                refreshKey={refreshKey}
                activePath={activePath}
                onOpen={(path) => void openPath(path)}
                onRefresh={() => setRefreshKey((value) => value + 1)}
                onNewFile={() => setCreateDialog('file')}
                onNewFolder={() => setCreateDialog('folder')}
              />
            ) : (
              <SearchPanel root={root} onOpen={(path, line) => void openPath(path, line)} />
            )}
          </aside>
        )}
        <main className="main">
          <div className="tabs">
            {tabs.map((tab) => (
              <button
                key={tab.path}
                className={`tab ${tab.path === activePath ? 'active' : ''}`}
                onClick={() => setActive(tab.path)}
                onContextMenu={(event) => { event.preventDefault(); setTabMenu({ x: event.clientX, y: event.clientY, path: tab.path }) }}
                title={tab.path}
              >
                <MaterialFileIcon name={tab.name} />
                <span className="tab-label">{tab.name}</span>
                {tab.dirty ? <span className="tab-dirty" aria-label="Unsaved changes">●</span> : null}
                <span
                  className="tab-close"
                  role="button"
                  aria-label={`Close ${tab.name}`}
                  onClick={(event) => {
                    event.stopPropagation()
                    closeTab(tab.path)
                  }}
                >
                  <Icon name="close" size={14} />
                </span>
              </button>
            ))}
            {tabs.length === 0 && <span className="tab-empty">No open editors</span>}
            <div className="tabs-spacer" />
            <div className="editor-actions">
              <button title="Check workspace" aria-label="Check workspace" onClick={() => void runTask('check')} disabled={!root || busy !== null}><Icon name="check" size={15} /></button>
              <button title="Build" aria-label="Build" onClick={() => void runTask('build')} disabled={!root || busy !== null}><Icon name="build" size={15} /></button>
              <button title="Run" aria-label="Run" onClick={() => void runTask('run')} disabled={!root || busy !== null}><Icon name="play" size={15} /></button>
            </div>
          </div>
          {activeTab && root && (
            <div className="breadcrumbs" aria-label="Breadcrumbs">
              {activeTab.path.replace(root, '').split('/').filter(Boolean).map((part, index, parts) => (
                <span key={`${part}-${index}`} className={index === parts.length - 1 ? 'crumb crumb-last' : 'crumb'}>
                  {index > 0 && <span className="crumb-sep">›</span>}
                  {part}
                </span>
              ))}
            </div>
          )}
          <div className="editor-area">
            <EditorPane editorReady={(instance) => { editorRef.current = instance; setEditor(instance) }} model={activeModel} settings={settings} onChange={(model) => {
              const uri = model.uri.toString()
              updateTabs((previous) => previous.map((tab) => (tab.uri === uri ? { ...tab, dirty: true } : tab)))
            }} />
            <EffectsLayer ref={effectsRef} enabled={effectsActive} />
            {!root && (
              <div className="welcome">
                <div className="welcome-inner">
                  <div className="welcome-mark">✦</div>
                  <h1>Nova Studio</h1>
                  <p className="welcome-sub">The Nova language workbench.</p>
                  <div className="welcome-section">
                    <h2>Start</h2>
                    <button className="welcome-link" onClick={() => void openWorkspace()}><Icon name="folder" size={16} /><span>Open Folder…</span><kbd>⌘O</kbd></button>
                    <button className="welcome-link" onClick={() => setOverlay('quick')}><Icon name="search" size={16} /><span>Go to File…</span><kbd>⌘P</kbd></button>
                    <button className="welcome-link" onClick={() => setOverlay('palette')}><Icon name="command" size={16} /><span>Command Palette…</span><kbd>⇧⌘P</kbd></button>
                  </div>
                  <p className="welcome-note">Requires the <code>nova</code> toolchain. Set <code>NOVA_BIN</code> when it is not on PATH.</p>
                </div>
              </div>
            )}
            {root && tabs.length === 0 && (
              <div className="welcome compact">
                <p><strong>{root}</strong></p>
                <p className="muted">Open a file from the explorer, or press <kbd>Cmd/Ctrl+P</kbd>.</p>
                {!hasManifest && <p className="warning">No nova.toml found. Tasks and LSP are limited.</p>}
              </div>
            )}
          </div>
          {bottomVisible && (
            <div className="bottom-panel">
              <div className="panel-tabs">
                {(['problems', 'output', 'terminal', 'tests', 'docs'] as const).map((panel) => (
                  <button key={panel} className={bottomTab === panel ? 'active' : ''} onClick={() => setBottomTab(panel)}>
                    {panel === 'problems' ? `Problems${problems.length ? ` (${problems.length})` : ''}` : panel[0].toUpperCase() + panel.slice(1)}
                  </button>
                ))}
                <div className="tabs-spacer" />
                {bottomTab === 'terminal' && (
                  <>
                    <button className="toolbar-button" onClick={() => setTerminalCommand('shell')}>Shell</button>
                    <button className="toolbar-button" onClick={() => setTerminalCommand('freebuff')}>FreeBuff</button>
                    {terminalCommand.startsWith('cmd:') && <span className="terminal-command-label">{terminalCommand.slice(4)}</span>}
                  </>
                )}
                <button className="icon-button" aria-label="Close panel" onClick={() => setBottomVisible(false)}>×</button>
              </div>
              <div className="panel-body">
                {bottomTab === 'problems' && <ProblemsPanel problems={problems} onOpen={(problem) => void openPath(problem.file, problem.line)} />}
                {bottomTab === 'output' && <pre className="output">{output}</pre>}
                {bottomTab === 'tests' && <pre className="output">{testsOutput}</pre>}
                {bottomTab === 'terminal' && (root ? <TerminalPanel key={`${terminalCommand}-${root}`} cwd={root} command={terminalCommand === 'shell' ? undefined : terminalCommand} onMode={setTerminalPty} /> : <div className="empty-panel">Open a folder to start a terminal.</div>)}
                {bottomTab === 'docs' && <DocsPanel />}
              </div>
            </div>
          )}
        </main>
        {aiVisible && (
          <AiPanel
            freebuff={freebuff}
            root={root}
            activePath={activeTab?.path ?? null}
            activeModel={activeModel}
            editor={editor}
            onStatus={setAiLabel}
            onTerminal={(command) => {
              setTerminalCommand(command)
              setBottomVisible(true)
              setBottomTab('terminal')
            }}
            onKeepEdit={keepAiEdit}
          />
        )}
      </div>
      <StatusBar cursor={cursor} lspStatus={lspStatus} vim={settings.vim} aiLabel={aiLabel} terminalPty={terminalPty} vimStatusRef={vimStatusRef} active={activeTab?.path ?? null} effects={effectsActive} sound={settings.sound} problemCount={problems.length} />
      {tabMenu && (
        <div className="context-menu" style={{ left: tabMenu.x, top: tabMenu.y }} onMouseLeave={() => setTabMenu(null)}>
          <button onClick={() => { setTabMenu(null); closeTab(tabMenu.path) }}>Close</button>
          <button onClick={() => { const path = tabMenu.path; setTabMenu(null); closeTabs(tabsRef.current.filter((tab) => tab.path !== path).map((tab) => tab.path)) }}>Close Others</button>
          <button onClick={() => { setTabMenu(null); closeTabs(tabsRef.current.map((tab) => tab.path)) }}>Close All</button>
          <button onClick={() => { const path = tabMenu.path; setTabMenu(null); void navigator.clipboard.writeText(path) }}>Copy Path</button>
          <button onClick={() => { const path = tabMenu.path; setTabMenu(null); void window.nova.fs.reveal(path) }}>Reveal in {window.nova.app.platform === 'darwin' ? 'Finder' : 'File Explorer'}</button>
        </div>
      )}
      {lineDialog && (
        <InputBox
          title="Go to Line"
          placeholder="Line number"
          submitLabel="Go"
          onSubmit={(value) => { setLineDialog(false); goToLine(value) }}
          onClose={() => setLineDialog(false)}
        />
      )}
      {createDialog && (
        <InputBox
          title={createDialog === 'folder' ? 'New Folder' : 'New File'}
          placeholder={createDialog === 'folder' ? 'Folder name' : 'File name, for example main.nova'}
          submitLabel="Create"
          onSubmit={(value) => {
            const kind = createDialog
            setCreateDialog(null)
            void createEntry(kind, value)
          }}
          onClose={() => setCreateDialog(null)}
        />
      )}
      {overlay === 'quick' && <QuickOpen root={root} onClose={() => setOverlay(null)} onOpen={(path) => { setOverlay(null); void openPath(path) }} />}
      {overlay === 'palette' && <CommandPalette onClose={() => setOverlay(null)} commands={[
        ['Open Folder…', () => handleMenu('open-folder')],
        ['New File…', () => handleMenu('new-file')],
        ['New Folder…', () => handleMenu('new-folder')],
        ['Save All', () => void saveAll()],
        ['Close All Tabs', () => closeTabs(tabsRef.current.map((tab) => tab.path))],
        ['Go to Line…', () => setLineDialog(true)],
        ['Save', () => handleMenu('save')],
        ['Build', () => handleMenu('build')],
        ['Run', () => handleMenu('run-task')],
        ['Check', () => handleMenu('check')],
        ['Test', () => handleMenu('test')],
        ['Format Document', () => handleMenu('format')],
        ['Toggle Sidebar', () => handleMenu('toggle-sidebar')],
        ['Toggle Terminal', () => handleMenu('toggle-terminal')],
        ['Toggle AI Panel', () => handleMenu('toggle-ai')],
        ['Toggle Vim Mode', () => handleMenu('toggle-vim')],
        ['Toggle Visual Effects', () => handleMenu('toggle-effects')],
        ['Toggle Sound Effects', () => handleMenu('toggle-sound')],
        ['Documentation', () => handleMenu('docs')],
        ['Settings', () => handleMenu('settings')],
        ['New Shell', () => handleMenu('new-shell')],
        ['FreeBuff Terminal', () => handleMenu('freebuff-terminal')]
      ]} />}
      {overlay === 'settings' && <SettingsModal settings={settings} reducedMotion={reducedMotion} onChange={setSettings} onClose={() => setOverlay(null)} />}
    </div>
  )
}

type IconName = 'files' | 'search' | 'sparkle' | 'settings' | 'sidebar' | 'panel' | 'close' | 'chevron' | 'check' | 'build' | 'play' | 'folder' | 'command'

const ICON_PATHS: Record<IconName, JSX.Element> = {
  files: <><path d="M5.5 1.5h4.2L13 4.8v8.7a1.5 1.5 0 0 1-1.5 1.5H5.5A1.5 1.5 0 0 1 4 13.5v-10a2 2 0 0 1 2-2Z" /><path d="M9.5 1.5v3.4H13" /><path d="M2 4.3v9.2A1.5 1.5 0 0 0 3.5 15H10" /></>,
  search: <><circle cx="7" cy="7" r="4.2" /><path d="m10.2 10.2 3.7 3.7" /></>,
  sparkle: <path d="M8 1.4 9.6 6.4 14.6 8 9.6 9.6 8 14.6 6.4 9.6 1.4 8 6.4 6.4Z" />,
  settings: <><circle cx="8" cy="8" r="2.2" /><path d="M8 1.4v1.7M8 12.9v1.7M1.4 8h1.7M12.9 8h1.7M3.3 3.3l1.2 1.2M11.5 11.5l1.2 1.2M12.7 3.3l-1.2 1.2M4.5 11.5l-1.2 1.2" /></>,
  sidebar: <><rect x="1.5" y="2.5" width="13" height="11" rx="1.5" /><path d="M6 2.5v11" /></>,
  panel: <><rect x="1.5" y="2.5" width="13" height="11" rx="1.5" /><path d="M1.5 9.5h13" /></>,
  close: <path d="m4.2 4.2 7.6 7.6M11.8 4.2l-7.6 7.6" />,
  chevron: <path d="m5.8 3.6 4.4 4.4-4.4 4.4" />,
  check: <path d="m2.4 8.4 3.6 3.6 7.6-8" />,
  build: <><path d="m8 1.8 5.4 3.1v6.2L8 14.2l-5.4-3.1V4.9Z" /><path d="M2.6 4.9 8 8l5.4-3.1M8 8v6.2" /></>,
  play: <path d="M5.2 3.1 13 8l-7.8 4.9Z" />,
  folder: <path d="M1.8 4.2A1.2 1.2 0 0 1 3 3h3l1.5 1.8H13a1.2 1.2 0 0 1 1.2 1.2v6.4A1.2 1.2 0 0 1 13 13.6H3a1.2 1.2 0 0 1-1.2-1.2Z" />,
  command: <><path d="m3 5 3 3-3 3" /><path d="M8.5 11.5h4.5" /></>
}

function Icon({ name, size = 16, className }: { name: IconName; size?: number; className?: string }): JSX.Element {
  return (
    <svg className={className} width={size} height={size} viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      {ICON_PATHS[name]}
    </svg>
  )
}

function TitleBar({ sidebar, panel, ai, onPalette, onToggleSidebar, onTogglePanel, onToggleAi }: {
  sidebar: boolean
  panel: boolean
  ai: boolean
  onPalette: () => void
  onToggleSidebar: () => void
  onTogglePanel: () => void
  onToggleAi: () => void
}): JSX.Element {
  return (
    <header className="titlebar">
      <div className="titlebar-center">
        <button className="command-center" title="Command Palette (⇧⌘P)" onClick={onPalette}>
          <span className="cc-mark">✦</span>
          <span className="cc-title">Nova Studio</span>
        </button>
      </div>
      <div className="titlebar-actions">
        <button aria-pressed={sidebar} title="Toggle Primary Side Bar" aria-label="Toggle Primary Side Bar" onClick={onToggleSidebar}><Icon name="sidebar" size={16} /></button>
        <button aria-pressed={panel} title="Toggle Panel" aria-label="Toggle Panel" onClick={onTogglePanel}><Icon name="panel" size={16} /></button>
        <button aria-pressed={ai} title="Toggle AI Panel" aria-label="Toggle AI Panel" onClick={onToggleAi}><Icon name="sparkle" size={15} /></button>
      </div>
    </header>
  )
}

function ActivityBar({ view, visible, aiVisible, onView, onToggleAi, onSettings }: { view: 'explorer' | 'search'; visible: boolean; aiVisible: boolean; onView: (view: 'explorer' | 'search') => void; onToggleAi: () => void; onSettings: () => void }): JSX.Element {
  return (
    <nav className="activitybar">
      <button className={view === 'explorer' && visible ? 'active' : ''} title="Explorer (⌘⇧E)" aria-label="Explorer" onClick={() => onView('explorer')}><Icon name="files" size={24} /></button>
      <button className={view === 'search' && visible ? 'active' : ''} title="Search (⌘⇧F)" aria-label="Search" onClick={() => onView('search')}><Icon name="search" size={23} /></button>
      <button className={aiVisible ? 'active' : ''} title="AI panel: FreeBuff and chat" aria-label="AI panel" onClick={onToggleAi}><Icon name="sparkle" size={22} /></button>
      <div className="spacer" />
      <button title="Settings" aria-label="Settings" onClick={onSettings}><Icon name="settings" size={22} /></button>
    </nav>
  )
}

function Explorer({ root, refreshKey, activePath, onOpen, onRefresh, onNewFile, onNewFolder }: { root: string | null; refreshKey: number; activePath: string | null; onOpen: (path: string) => void; onRefresh: () => void; onNewFile: () => void; onNewFolder: () => void }): JSX.Element {
  const [entries, setEntries] = useState<FileEntry[]>([])
  const [expanded, setExpanded] = useState<Set<string>>(new Set())
  const load = useCallback(async () => {
    if (!root) { setEntries([]); return }
    try { setEntries(await window.nova.fs.list(root)) } catch { setEntries([]) }
  }, [root])
  useEffect(() => { void load() }, [load, refreshKey])

  // Reveal the active file by expanding its ancestor folders, like VS Code.
  useEffect(() => {
    if (!root || !activePath) return
    setExpanded((previous) => {
      const next = new Set(previous)
      let dir = root
      for (const part of activePath.slice(root.length + 1).split('/').slice(0, -1)) {
        dir = joinPath(dir, part)
        next.add(dir)
      }
      return next.size === previous.size ? previous : next
    })
  }, [activePath, root])

  const toggle = useCallback((path: string) => {
    setExpanded((previous) => {
      const next = new Set(previous)
      if (next.has(path)) next.delete(path)
      else next.add(path)
      return next
    })
  }, [])
  return (
    <div className="explorer">
      <div className="panel-header">
        <span>{root ? root.split('/').pop() : 'Explorer'}</span>
        <div className="panel-header-actions">
          <button title="New File" aria-label="New File" onClick={onNewFile}><Icon name="files" size={14} /></button>
          <button title="New Folder" aria-label="New Folder" onClick={onNewFolder}><Icon name="folder" size={14} /></button>
          <button title="Refresh" aria-label="Refresh" onClick={onRefresh}>⟳</button>
        </div>
      </div>
      {!root && <div className="empty-panel">No folder open.</div>}
      <div className="tree">{entries.map((entry) => <TreeNode key={entry.path} entry={entry} activePath={activePath} expanded={expanded} onToggle={toggle} onOpen={onOpen} refreshKey={refreshKey} onRefresh={onRefresh} />)}</div>
    </div>
  )
}

function TreeNode({ entry, activePath, expanded, onToggle, onOpen, refreshKey, onRefresh }: { entry: FileEntry; activePath: string | null; expanded: Set<string>; onToggle: (path: string) => void; onOpen: (path: string) => void; refreshKey: number; onRefresh: () => void }): JSX.Element {
  const open = expanded.has(entry.path)
  const [children, setChildren] = useState<FileEntry[]>([])
  const [menu, setMenu] = useState<{ x: number; y: number } | null>(null)
  const [renameOpen, setRenameOpen] = useState(false)
  useEffect(() => {
    if (!entry.isDir || !open) return
    window.nova.fs.list(entry.path).then(setChildren).catch(() => setChildren([]))
  }, [entry, open, refreshKey])
  const rename = async (name: string) => {
    const trimmed = name.trim()
    const parent = entry.path.slice(0, entry.path.lastIndexOf('/'))
    if (!trimmed || trimmed === entry.name) return
    try { await window.nova.fs.rename(entry.path, joinPath(parent, trimmed)); onRefresh() } catch (error) { window.alert((error as Error).message) }
  }
  const remove = async () => {
    if (!window.confirm(`Delete ${entry.name}? This cannot be undone.`)) return
    try { await window.nova.fs.remove(entry.path); onRefresh() } catch (error) { window.alert((error as Error).message) }
  }
  return (
    <div>
      <div className={`tree-row ${activePath === entry.path ? 'active' : ''}`} onClick={() => entry.isDir ? onToggle(entry.path) : onOpen(entry.path)} onContextMenu={(event) => { event.preventDefault(); setMenu({ x: event.clientX, y: event.clientY }) }}>
        {entry.isDir ? <><span className="tree-icon"><Icon name="chevron" size={12} className={open ? 'chevron chevron-open' : 'chevron'} /></span><MaterialFileIcon name={entry.name} isDir open={open} /></> : <MaterialFileIcon name={entry.name} />}
        <span className={entry.name.endsWith('.nova') ? 'nova-file' : ''}>{entry.name}</span>
      </div>
      {entry.isDir && open && <div className="tree-children">{children.map((child) => <TreeNode key={child.path} entry={child} activePath={activePath} expanded={expanded} onToggle={onToggle} onOpen={onOpen} refreshKey={refreshKey} onRefresh={onRefresh} />)}</div>}
      {menu && (
        <div className="context-menu" style={{ left: menu.x, top: menu.y }} onMouseLeave={() => setMenu(null)}>
          <button onClick={() => { setMenu(null); setRenameOpen(true) }}>Rename…</button>
          <button onClick={() => { setMenu(null); void remove() }}>Delete</button>
        </div>
      )}
      {renameOpen && (
        <InputBox
          title={`Rename ${entry.name}`}
          initial={entry.name}
          placeholder="New name"
          submitLabel="Rename"
          onSubmit={(value) => { setRenameOpen(false); void rename(value) }}
          onClose={() => setRenameOpen(false)}
        />
      )}
    </div>
  )
}

function SearchPanel({ root, onOpen }: { root: string | null; onOpen: (path: string, line: number) => void }): JSX.Element {
  const [query, setQuery] = useState('')
  const [regex, setRegex] = useState(false)
  const [caseSensitive, setCaseSensitive] = useState(false)
  const [wholeWord, setWholeWord] = useState(false)
  const [hits, setHits] = useState<SearchHit[]>([])
  const [message, setMessage] = useState('')
  const run = async () => {
    if (!root || !query.trim()) return
    setMessage('Searching…')
    const results = await window.nova.search.run(query, { regex, caseSensitive, wholeWord })
    setHits(results)
    setMessage(`${results.length} result(s)`)
  }
  return (
    <div className="search-panel">
      <div className="panel-header"><span>Search</span></div>
      <div className="search-form">
        <input value={query} placeholder="Search workspace" onChange={(event) => setQuery(event.target.value)} onKeyDown={(event) => { if (event.key === 'Enter') void run() }} />
        <div className="search-options">
          <label><input type="checkbox" checked={regex} onChange={(event) => setRegex(event.target.checked)} /> regex</label>
          <label><input type="checkbox" checked={caseSensitive} onChange={(event) => setCaseSensitive(event.target.checked)} /> case</label>
          <label><input type="checkbox" checked={wholeWord} onChange={(event) => setWholeWord(event.target.checked)} /> word</label>
        </div>
        <button className="primary" onClick={() => void run()} disabled={!root}>Search</button>
        <span className="muted">{message}</span>
      </div>
      <div className="search-results">
        {hits.map((hit, index) => (
          <button key={`${hit.file}:${hit.line}:${index}`} className="search-hit" onClick={() => onOpen(joinPath(root!, hit.file), hit.line)} title={`${hit.file}:${hit.line}:${hit.column}`}>
            <span className="hit-file">{hit.file}:{hit.line}</span>
            <span className="hit-text">{hit.text.trim()}</span>
          </button>
        ))}
      </div>
    </div>
  )
}

function ProblemsPanel({ problems, onOpen }: { problems: Problem[]; onOpen: (problem: Problem) => void }): JSX.Element {
  if (problems.length === 0) return <div className="empty-panel">No problems. Diagnostics arrive from the Nova language server.</div>
  return (
    <div className="problems">
      {problems.map((problem, index) => (
        <button key={`${problem.uri}:${problem.line}:${problem.column}:${index}`} className={`problem severity-${problem.severity}`} onClick={() => onOpen(problem)}>
          <span className="problem-icon">{problem.severity === 1 ? '✕' : '!'}</span>
          <span className="problem-message">{problem.message}</span>
          <span className="problem-location">{problem.file.split('/').pop()}:{problem.line}:{problem.column}</span>
        </button>
      ))}
    </div>
  )
}

function EditorPane({ editorReady, model, settings, onChange }: { editorReady: (editor: Monaco.editor.IStandaloneCodeEditor) => void; model: Monaco.editor.ITextModel | null; settings: Settings; onChange: (model: Monaco.editor.ITextModel) => void }): JSX.Element {
  const container = useRef<HTMLDivElement | null>(null)
  const instance = useRef<Monaco.editor.IStandaloneCodeEditor | null>(null)
  useEffect(() => {
    if (!container.current) return
    const editor = monaco.editor.create(container.current, {
      theme: settings.theme,
      automaticLayout: true,
      fontFamily: MONO_FONT,
      fontLigatures: true,
      fontSize: settings.fontSize,
      lineHeight: 22,
      minimap: { enabled: settings.minimap, showSlider: 'mouseover', renderCharacters: false },
      lineNumbers: 'on',
      lineNumbersMinChars: 4,
      renderLineHighlight: 'line',
      renderWhitespace: 'selection',
      bracketPairColorization: { enabled: true },
      guides: { bracketPairs: true, indentation: true, highlightActiveIndentation: true },
      smoothScrolling: false,
      cursorBlinking: 'blink',
      padding: { top: 4 },
      scrollBeyondLastLine: false,
      stickyScroll: { enabled: true },
      scrollbar: { verticalScrollbarSize: 14, horizontalScrollbarSize: 14 },
      tabSize: 4,
      insertSpaces: true,
      formatOnPaste: true
    })
    instance.current = editor
    editorReady(editor)
    const disposable = editor.onDidChangeModelContent(() => {
      const current = editor.getModel()
      if (current) onChange(current)
    })
    return () => {
      disposable.dispose()
      editor.dispose()
    }
    // The editor is created once; model changes are handled below.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])
  useEffect(() => {
    if (instance.current && instance.current.getModel() !== model) {
      instance.current.setModel(model)
    }
  }, [model])
  return <div className="editor" ref={container} />
}

function DocsPanel(): JSX.Element {
  return (
    <div className="docs-panel">
      <div className="docs-intro">
        <div className="docs-kicker">Nova documentation</div>
        <h2>Language, Studio and toolchain</h2>
        <p>This quick reference ships with the editor. The canonical, searchable documentation is on the public site.</p>
        <code>https://jd4rider.github.io/nova/</code>
      </div>
      <div className="docs-grid">
        <section>
          <h3>Language</h3>
          <p>Bindings, functions, structs, lists, strings, Option/Result, match and loops, plus generic functions, static traits, methods, tuples/arrays/slices, capturing closures and expression/statement macros. Trait impls on generic types and real async scheduling are rejected with diagnostics.</p>
          <span className="docs-key">docs/language/reference</span>
        </section>
        <section>
          <h3>Studio</h3>
          <p>Open a folder with nova.toml; use Cmd/Ctrl+P to open files, Cmd/Ctrl+Shift+P for commands, and Settings for themes, Vim, visual effects and sound.</p>
          <span className="docs-key">docs/studio/overview</span>
        </section>
        <section>
          <h3>CLI</h3>
          <p><code>nova new</code>, <code>init</code>, <code>run</code>, <code>build</code>, <code>check</code>, <code>test</code>, <code>fmt</code>, <code>lsp</code>, <code>tree</code> and <code>doctor</code> are the implemented commands.</p>
          <span className="docs-key">docs/toolchain/cli</span>
        </section>
        <section>
          <h3>Building Nova</h3>
          <p>Build the compiler with <code>(cd examples/nova-tool &amp;&amp; ../../bootstrap/nova build)</code>, then point Studio at it with <code>NOVA_BIN</code>. LSP startup needs a project manifest.</p>
          <span className="docs-key">README.md</span>
        </section>
      </div>
      <div className="docs-foot">
        <span>Docs tab · offline quick reference</span>
        <span>Repository: github.com/jd4rider/nova</span>
      </div>
    </div>
  )
}

function TerminalPanel({ cwd, command, onMode }: { cwd: string; command?: string; onMode: (pty: boolean) => void }): JSX.Element {
  const container = useRef<HTMLDivElement | null>(null)
  const [status, setStatus] = useState('starting…')
  useEffect(() => {
    if (!container.current) return
    const term = new Terminal({
      fontFamily: MONO_FONT,
      fontSize: 12,
      cursorBlink: true,
      theme: {
        background: '#1e1e1e',
        foreground: '#d4d4d4',
        cursor: '#aeafad',
        black: '#000000',
        red: '#cd3131',
        green: '#0dbc79',
        yellow: '#e5e510',
        blue: '#2472c8',
        magenta: '#bc3fbc',
        cyan: '#11a8cd',
        white: '#e5e5e5',
        brightBlack: '#666666',
        brightRed: '#f14c4c',
        brightGreen: '#23d18b',
        brightYellow: '#f5f543',
        brightBlue: '#3b8eea',
        brightMagenta: '#d670d6',
        brightCyan: '#29b8db',
        brightWhite: '#e5e5e5'
      }
    })
    const fit = new FitAddon()
    term.loadAddon(fit)
    term.open(container.current)
    fit.fit()
    let session: number | null = null
    let disposed = false
    const offData = window.nova.terminal.onData(({ id, data }) => {
      if (id === session) term.write(data)
    })
    window.nova.terminal
      .create(cwd, term.cols, term.rows, command)
      .then(({ id, pty }) => {
        if (disposed) { void window.nova.terminal.dispose(id); return }
        session = id
        setStatus(pty ? 'pty' : 'pipe mode (install node-pty for a full TTY)')
        onMode(pty)
        term.onData((data) => { if (session !== null) void window.nova.terminal.write(session, data) })
        term.onResize(({ cols, rows }) => { if (session !== null) void window.nova.terminal.resize(session, cols, rows) })
        window.addEventListener('resize', resize)
      })
      .catch((error) => setStatus(`error: ${(error as Error).message}`))
    const resize = () => { try { fit.fit() } catch { /* hidden */ } }
    const observer = new ResizeObserver(resize)
    observer.observe(container.current)
    return () => {
      disposed = true
      observer.disconnect()
      offData()
      window.removeEventListener('resize', resize)
      if (session !== null) void window.nova.terminal.dispose(session)
      term.dispose()
    }
  }, [command, cwd, onMode])
  const label = command === 'freebuff' ? 'FreeBuff terminal' : command?.startsWith('cmd:') ? command.slice(4) : 'Shell'
  return <div className="terminal-wrap"><div className="terminal-status">{label} · {status}</div><div className="terminal" ref={container} /></div>
}

function StatusBar({ cursor, lspStatus, vim, aiLabel, terminalPty, vimStatusRef, active, effects, sound, problemCount }: { cursor: { line: number; column: number }; lspStatus: string; vim: boolean; aiLabel: string; terminalPty: boolean | null; vimStatusRef: React.RefObject<HTMLSpanElement>; active: string | null; effects: boolean; sound: boolean; problemCount: number }): JSX.Element {
  return (
    <footer className="statusbar">
      <div className="status-left">
        <span className="status-item" title={active ?? 'No file open'}>{active ? active.split('/').pop() : 'Nova Studio'}</span>
        <span className="status-item" title="Nova language server"><Icon name="check" size={12} /> LSP {lspStatus}</span>
        {problemCount > 0 && <span className="status-item" title="Problems reported by the language server">{problemCount} problem{problemCount === 1 ? '' : 's'}</span>}
        <span className="status-item">Terminal {terminalPty === null ? 'idle' : terminalPty ? 'PTY' : 'pipe'}</span>
        <span className="status-item" title="AI provider">AI {aiLabel}</span>
      </div>
      <div className="status-right">
        {vim && <span className="status-item vim-status" ref={vimStatusRef}>NORMAL</span>}
        {effects && <span className="status-item" title="Effects enabled">FX</span>}
        {sound && <span className="status-item" title="Sound enabled">SFX</span>}
        <span className="status-item">Ln {cursor.line}, Col {cursor.column}</span>
        <span className="status-item">Spaces: 4</span>
        <span className="status-item">UTF-8</span>
        <span className="status-item">LF</span>
        <span className="status-item">Nova</span>
      </div>
    </footer>
  )
}

function QuickOpen({ root, onClose, onOpen }: { root: string | null; onClose: () => void; onOpen: (path: string) => void }): JSX.Element {
  const [files, setFiles] = useState<string[]>([])
  const [query, setQuery] = useState('')
  const [index, setIndex] = useState(0)
  useEffect(() => {
    if (root) window.nova.fs.files().then(setFiles).catch(() => setFiles([]))
  }, [root])
  const matches = files.filter((file) => file.toLowerCase().includes(query.toLowerCase())).slice(0, 60)
  return (
    <Overlay onClose={onClose}>
      <input
        autoFocus
        className="overlay-input"
        placeholder="Search files by name…"
        value={query}
        onChange={(event) => { setQuery(event.target.value); setIndex(0) }}
        onKeyDown={(event) => {
          if (event.key === 'Escape') onClose()
          if (event.key === 'ArrowDown') { event.preventDefault(); setIndex((value) => Math.min(value + 1, matches.length - 1)) }
          if (event.key === 'ArrowUp') { event.preventDefault(); setIndex((value) => Math.max(value - 1, 0)) }
          if (event.key === 'Enter' && matches[index] && root) onOpen(joinPath(root, matches[index]))
        }}
      />
      <div className="overlay-list">
        {matches.map((file, itemIndex) => (
          <button key={file} className={itemIndex === index ? 'selected' : ''} onClick={() => root && onOpen(joinPath(root, file))}>{file}</button>
        ))}
      </div>
    </Overlay>
  )
}

function CommandPalette({ commands, onClose }: { commands: Array<[string, () => void]>; onClose: () => void }): JSX.Element {
  const [query, setQuery] = useState('')
  const [index, setIndex] = useState(0)
  const matches = commands.filter(([label]) => label.toLowerCase().includes(query.toLowerCase()))
  const execute = (command?: () => void) => { if (!command) return; onClose(); command() }
  return (
    <Overlay onClose={onClose}>
      <input
        autoFocus
        className="overlay-input"
        placeholder="Type a command…"
        value={query}
        onChange={(event) => { setQuery(event.target.value); setIndex(0) }}
        onKeyDown={(event) => {
          if (event.key === 'Escape') onClose()
          if (event.key === 'ArrowDown') { event.preventDefault(); setIndex((value) => Math.min(value + 1, matches.length - 1)) }
          if (event.key === 'ArrowUp') { event.preventDefault(); setIndex((value) => Math.max(value - 1, 0)) }
          if (event.key === 'Enter') execute(matches[index]?.[1])
        }}
      />
      <div className="overlay-list">
        {matches.map(([label, action], itemIndex) => (
          <button key={label} className={itemIndex === index ? 'selected' : ''} onClick={() => execute(action)}>{label}</button>
        ))}
      </div>
    </Overlay>
  )
}

function InputBox({ title, placeholder, initial = '', submitLabel, onSubmit, onClose }: {
  title: string
  placeholder?: string
  initial?: string
  submitLabel: string
  onSubmit: (value: string) => void
  onClose: () => void
}): JSX.Element {
  const [value, setValue] = useState(initial)
  const submit = () => { if (value.trim()) onSubmit(value) }
  return (
    <div className="overlay" onMouseDown={(event) => { if (event.target === event.currentTarget) onClose() }}>
      <div className="overlay-card inputbox-card">
        <div className="inputbox">
          <div className="inputbox-title">{title}</div>
          <input
            autoFocus
            value={value}
            placeholder={placeholder}
            spellCheck={false}
            onChange={(event) => setValue(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === 'Enter') { event.preventDefault(); submit() }
              if (event.key === 'Escape') onClose()
            }}
          />
          <div className="inputbox-actions">
            <button onClick={onClose}>Cancel</button>
            <button className="primary" onClick={submit} disabled={!value.trim()}>{submitLabel}</button>
          </div>
        </div>
      </div>
    </div>
  )
}

function Overlay({ children, onClose }: { children: React.ReactNode; onClose: () => void }): JSX.Element {
  return <div className="overlay" onMouseDown={(event) => { if (event.target === event.currentTarget) onClose() }}><div className="overlay-card">{children}</div></div>
}

function SettingsModal({ settings, reducedMotion, onChange, onClose }: { settings: Settings; reducedMotion: boolean; onChange: (settings: Settings) => void; onClose: () => void }): JSX.Element {
  return (
    <Overlay onClose={onClose}>
      <div className="settings">
        <h2>Settings</h2>
        <label>Theme
          <select value={settings.theme} onChange={(event) => onChange({ ...settings, theme: event.target.value as ThemeName })}>
            <option value="vs-dark">Dark+ (Visual Studio Code)</option>
            <option value="gruvbox-dark">Gruvbox Dark</option>
            <option value="gruvbox-light">Gruvbox Light</option>
            <option value="gruvbox-hc">Gruvbox High Contrast</option>
          </select>
        </label>
        <label>Font size
          <input type="number" min={10} max={28} value={settings.fontSize} onChange={(event) => onChange({ ...settings, fontSize: Number(event.target.value) })} />
        </label>
        <label className="check"><input type="checkbox" checked={settings.vim} onChange={(event) => onChange({ ...settings, vim: event.target.checked })} /> Vim mode (NORMAL/INSERT/VISUAL; see docs for limitations)</label>
        <label className="check"><input type="checkbox" checked={settings.minimap} onChange={(event) => onChange({ ...settings, minimap: event.target.checked })} /> Minimap</label>
        <div className="settings-group">
          <h3>Visual effects &amp; sound</h3>
          <label className="check"><input type="checkbox" checked={settings.effects} onChange={(event) => onChange({ ...settings, effects: event.target.checked })} /> Current-line lens, typing sparkles and scroll sparkles</label>
          <label className="check"><input type="checkbox" checked={settings.sound} onChange={(event) => onChange({ ...settings, sound: event.target.checked })} /> Synthesized interface sounds</label>
          <label className="range-label">Volume <span>{settings.volume}%</span>
            <input type="range" min={0} max={100} step={5} value={settings.volume} onChange={(event) => onChange({ ...settings, volume: Number(event.target.value) })} />
          </label>
          <p className="settings-note">Effects are local only: DOM particles and Web Audio, with no assets or telemetry. The current-line lens follows Vim motions. System reduced-motion disables animated effects.</p>
          {reducedMotion && <p className="settings-note warning">Reduced motion is enabled in the OS, so animation is currently suppressed.</p>}
        </div>
        <button className="primary" onClick={onClose}>Close</button>
      </div>
    </Overlay>
  )
}

function usePrefersReducedMotion(): boolean {
  const [reduced, setReduced] = useState(() => window.matchMedia?.('(prefers-reduced-motion: reduce)').matches ?? false)
  useEffect(() => {
    const query = window.matchMedia('(prefers-reduced-motion: reduce)')
    const update = () => setReduced(query.matches)
    query.addEventListener('change', update)
    return () => query.removeEventListener('change', update)
  }, [])
  return reduced
}
