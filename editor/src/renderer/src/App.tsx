import { useCallback, useEffect, useRef, useState } from 'react'
import type * as Monaco from 'monaco-editor'
import { Terminal } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import { monaco, registerLspProviders } from './monaco'
import { EffectsLayer, type EffectsHandle } from './effects'
import { playScroll, playTyping } from './sfx'
import type { FileEntry, FreeBuffStatus, LspDiagnostic, ThemeName } from '../../shared/types'

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
  theme: 'gruvbox-dark',
  fontSize: 14,
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
    return { ...defaultSettings, ...JSON.parse(localStorage.getItem('nova.settings') ?? '{}') }
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
  const [settings, setSettings] = useState<Settings>(loadSettings)
  const reducedMotion = usePrefersReducedMotion()
  const effectsActive = settings.effects && !reducedMotion
  const [overlay, setOverlay] = useState<'quick' | 'palette' | 'settings' | null>(null)
  const [refreshKey, setRefreshKey] = useState(0)
  const [terminalCommand, setTerminalCommand] = useState<'shell' | 'freebuff'>('shell')
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
      if (modifier && event.key.toLowerCase() === 'p') {
        event.preventDefault()
        event.stopPropagation()
        setOverlay(event.shiftKey ? 'palette' : 'quick')
      } else if (modifier && event.key.toLowerCase() === 's') {
        event.preventDefault()
        void saveActive()
      } else if (modifier && event.key === '`') {
        event.preventDefault()
        setBottomVisible((value) => !value)
      } else if (event.key === 'Escape' && overlay) {
        setOverlay(null)
      }
    }
    window.addEventListener('keydown', onKey, true)
    return () => window.removeEventListener('keydown', onKey, true)
  }, [overlay, saveActive])

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

  const openWorkspace = useCallback(async () => {
    const result = await window.nova.workspace.choose()
    if (!result) return
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
          if (root) {
            const name = window.prompt('New file path (relative to workspace)')
            if (name) {
              window.nova.fs.create(joinPath(root, name), '').then(() => {
                setRefreshKey((value) => value + 1)
                void openPath(joinPath(root, name))
              })
            }
          }
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

  const activeTab = tabs.find((tab) => tab.path === activePath) ?? null
  const activeModel = activeTab?.model ?? null

  return (
    <div className="app">
      <TitleBar />
      <div className="workbench">
        <ActivityBar view={sidebar} visible={sidebarVisible} onView={(view) => { setSidebar(view); setSidebarVisible(true) }} onToggle={() => setSidebarVisible((value) => !value)} />
        {sidebarVisible && (
          <aside className="sidebar">
            {sidebar === 'explorer' ? (
              <Explorer root={root} refreshKey={refreshKey} onOpen={(path) => void openPath(path)} onRefresh={() => setRefreshKey((value) => value + 1)} />
            ) : (
              <SearchPanel root={root} onOpen={(path, line) => void openPath(path, line)} />
            )}
          </aside>
        )}
        <main className="main">
          <div className="tabs">
            {tabs.map((tab) => (
              <button key={tab.path} className={`tab ${tab.path === activePath ? 'active' : ''}`} onClick={() => setActive(tab.path)} title={tab.path}>
                <span>{tab.dirty ? '● ' : ''}{tab.name}</span>
                <span
                  className="tab-close"
                  role="button"
                  aria-label={`Close ${tab.name}`}
                  onClick={(event) => {
                    event.stopPropagation()
                    closeTab(tab.path)
                  }}
                >
                  ×
                </span>
              </button>
            ))}
            {tabs.length === 0 && <span className="tab-empty">No open editors</span>}
            <div className="tabs-spacer" />
            <button className="toolbar-button" onClick={() => void runTask('check')} disabled={!root || busy !== null}>Check</button>
            <button className="toolbar-button" onClick={() => void runTask('build')} disabled={!root || busy !== null}>Build</button>
            <button className="toolbar-button primary" onClick={() => void runTask('run')} disabled={!root || busy !== null}>Run</button>
          </div>
          <div className="editor-area">
            <EditorPane editorReady={(instance) => { editorRef.current = instance; setEditor(instance) }} model={activeModel} settings={settings} onChange={(model) => {
              const uri = model.uri.toString()
              updateTabs((previous) => previous.map((tab) => (tab.uri === uri ? { ...tab, dirty: true } : tab)))
            }} />
            <EffectsLayer ref={effectsRef} enabled={effectsActive} />
            {!root && (
              <div className="welcome">
                <h1>Nova Studio</h1>
                <p>A native IDE for the Nova programming language.</p>
                <button className="primary" onClick={() => void openWorkspace()}>Open Folder</button>
                <p className="muted">Requires the <code>nova</code> toolchain. Set <code>NOVA_BIN</code> when it is not on PATH.</p>
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
                  </>
                )}
                <button className="icon-button" aria-label="Close panel" onClick={() => setBottomVisible(false)}>×</button>
              </div>
              <div className="panel-body">
                {bottomTab === 'problems' && <ProblemsPanel problems={problems} onOpen={(problem) => void openPath(problem.file, problem.line)} />}
                {bottomTab === 'output' && <pre className="output">{output}</pre>}
                {bottomTab === 'tests' && <pre className="output">{testsOutput}</pre>}
                {bottomTab === 'terminal' && (root ? <TerminalPanel key={`${terminalCommand}-${root}`} cwd={root} command={terminalCommand === 'freebuff' ? 'freebuff' : undefined} onMode={setTerminalPty} /> : <div className="empty-panel">Open a folder to start a terminal.</div>)}
                {bottomTab === 'docs' && <DocsPanel />}
              </div>
            </div>
          )}
        </main>
        <FreeBuffPanel status={freebuff} onOpenTerminal={() => handleMenu('freebuff-terminal')} />
      </div>
      <StatusBar cursor={cursor} lspStatus={lspStatus} vim={settings.vim} freebuff={freebuff} terminalPty={terminalPty} vimStatusRef={vimStatusRef} active={activeTab?.path ?? null} effects={effectsActive} sound={settings.sound} />
      {overlay === 'quick' && <QuickOpen root={root} onClose={() => setOverlay(null)} onOpen={(path) => { setOverlay(null); void openPath(path) }} />}
      {overlay === 'palette' && <CommandPalette onClose={() => setOverlay(null)} commands={[
        ['Open Folder…', () => handleMenu('open-folder')],
        ['Save', () => handleMenu('save')],
        ['Build', () => handleMenu('build')],
        ['Run', () => handleMenu('run-task')],
        ['Check', () => handleMenu('check')],
        ['Test', () => handleMenu('test')],
        ['Format Document', () => handleMenu('format')],
        ['Toggle Sidebar', () => handleMenu('toggle-sidebar')],
        ['Toggle Terminal', () => handleMenu('toggle-terminal')],
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

function TitleBar(): JSX.Element {
  return <header className="titlebar"><span className="brand">Nova Studio</span><span className="muted">native language tooling · FreeBuff terminal integration</span></header>
}

function ActivityBar({ view, visible, onView, onToggle }: { view: 'explorer' | 'search'; visible: boolean; onView: (view: 'explorer' | 'search') => void; onToggle: () => void }): JSX.Element {
  return (
    <nav className="activitybar">
      <button className={view === 'explorer' && visible ? 'active' : ''} title="Explorer" onClick={() => onView('explorer')}>▤</button>
      <button className={view === 'search' && visible ? 'active' : ''} title="Search" onClick={() => onView('search')}>⌕</button>
      <div className="spacer" />
      <button title="Toggle sidebar" onClick={onToggle}>▥</button>
    </nav>
  )
}

function Explorer({ root, refreshKey, onOpen, onRefresh }: { root: string | null; refreshKey: number; onOpen: (path: string) => void; onRefresh: () => void }): JSX.Element {
  const [entries, setEntries] = useState<FileEntry[]>([])
  const load = useCallback(async () => {
    if (!root) { setEntries([]); return }
    try { setEntries(await window.nova.fs.list(root)) } catch { setEntries([]) }
  }, [root])
  useEffect(() => { void load() }, [load, refreshKey])
  const create = async () => {
    if (!root) return
    const name = window.prompt('New file path')
    if (!name) return
    try { await window.nova.fs.create(joinPath(root, name), ''); onRefresh() } catch (error) { window.alert((error as Error).message) }
  }
  return (
    <div className="explorer">
      <div className="panel-header"><span>{root ? root.split('/').pop() : 'Explorer'}</span><div><button title="New file" onClick={() => void create()}>＋</button><button title="Refresh" onClick={onRefresh}>⟳</button></div></div>
      {!root && <div className="empty-panel">No folder open.</div>}
      <div className="tree">{entries.map((entry) => <TreeNode key={entry.path} entry={entry} onOpen={onOpen} refreshKey={refreshKey} onRefresh={onRefresh} />)}</div>
    </div>
  )
}

function TreeNode({ entry, onOpen, refreshKey, onRefresh }: { entry: FileEntry; onOpen: (path: string) => void; refreshKey: number; onRefresh: () => void }): JSX.Element {
  const [open, setOpen] = useState(false)
  const [children, setChildren] = useState<FileEntry[]>([])
  useEffect(() => {
    if (!entry.isDir || !open) return
    window.nova.fs.list(entry.path).then(setChildren).catch(() => setChildren([]))
  }, [entry, open, refreshKey])
  const rename = async () => {
    const name = window.prompt('Rename to', entry.name)
    const parent = entry.path.slice(0, entry.path.lastIndexOf('/'))
    if (!name || name === entry.name) return
    try { await window.nova.fs.rename(entry.path, joinPath(parent, name)); onRefresh() } catch (error) { window.alert((error as Error).message) }
  }
  const remove = async () => {
    if (!window.confirm(`Delete ${entry.name}? This cannot be undone.`)) return
    try { await window.nova.fs.remove(entry.path); onRefresh() } catch (error) { window.alert((error as Error).message) }
  }
  return (
    <div>
      <div className="tree-row" onClick={() => entry.isDir ? setOpen(!open) : onOpen(entry.path)} onContextMenu={(event) => { event.preventDefault(); const choice = window.prompt('Type r to rename, d to delete, or cancel', 'r'); if (choice === 'r') void rename(); if (choice === 'd') void remove() }}>
        <span className="tree-icon">{entry.isDir ? (open ? '▾' : '▸') : entry.name.endsWith('.nova') ? '◆' : '·'}</span>
        <span className={entry.name.endsWith('.nova') ? 'nova-file' : ''}>{entry.name}</span>
      </div>
      {entry.isDir && open && <div className="tree-children">{children.map((child) => <TreeNode key={child.path} entry={child} onOpen={onOpen} refreshKey={refreshKey} onRefresh={onRefresh} />)}</div>}
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
      fontFamily: "'JetBrains Mono', 'SF Mono', Menlo, monospace",
      fontLigatures: true,
      fontSize: settings.fontSize,
      minimap: { enabled: settings.minimap },
      lineNumbers: 'on',
      renderWhitespace: 'selection',
      bracketPairColorization: { enabled: true },
      guides: { bracketPairs: true, indentation: true },
      smoothScrolling: true,
      cursorBlinking: 'smooth',
      padding: { top: 10 },
      scrollBeyondLastLine: false,
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
          <p>Bindings, functions, structs, lists, strings, Option/Result, match and loops. Unsupported generics, traits and closures are rejected with diagnostics.</p>
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
          <p>Build the compiler with <code>cargo build --workspace</code>, then point Studio at it with <code>NOVA_BIN</code>. LSP startup needs a project manifest.</p>
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
      fontFamily: "'JetBrains Mono', Menlo, monospace",
      fontSize: 12,
      cursorBlink: true,
      theme: {
        background: '#1d2021',
        foreground: '#ebdbb2',
        cursor: '#fe8019',
        black: '#282828',
        red: '#fb4934',
        green: '#b8bb26',
        yellow: '#fabd2f',
        blue: '#83a598',
        magenta: '#d3869b',
        cyan: '#8ec07c',
        white: '#ebdbb2',
        brightBlack: '#928374',
        brightRed: '#fb4934',
        brightGreen: '#b8bb26',
        brightYellow: '#fabd2f',
        brightBlue: '#83a598',
        brightMagenta: '#d3869b',
        brightCyan: '#8ec07c',
        brightWhite: '#fbf1c7'
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
  return <div className="terminal-wrap"><div className="terminal-status">{command === 'freebuff' ? 'FreeBuff terminal' : 'Shell'} · {status}</div><div className="terminal" ref={container} /></div>
}

function FreeBuffPanel({ status, onOpenTerminal }: { status: FreeBuffStatus | null; onOpenTerminal: () => void }): JSX.Element {
  return (
    <aside className="aipanel">
      <div className="panel-header"><span>FreeBuff</span><span className={`badge ${status?.available ? 'ok' : 'warn'}`}>{status?.available ? status.version ?? 'installed' : 'unavailable'}</span></div>
      <div className="ai-body">
        <p className="ai-copy">
          {status?.reason ?? 'Checking the installed FreeBuff CLI…'}
        </p>
        <button className="primary" onClick={onOpenTerminal} disabled={!status?.available}>Open FreeBuff terminal</button>
        <div className="ai-unavailable">
          <strong>Chat and agent mode are disabled.</strong>
          <p>
            The installed FreeBuff CLI exposes an interactive terminal UI only. Its documented interface
            does not include a structured headless chat protocol, so Nova Studio does not scrape terminal
            output or automate authentication. Inline AI ghost text is also unavailable.
          </p>
          <p>All deterministic Nova LSP completion, hover, diagnostics and formatting remain enabled.</p>
        </div>
        <div className="context-note">
          <strong>Context sent to tools</strong>
          <p>No repository contents are sent by Nova Studio. The dedicated FreeBuff terminal is a normal local process you control.</p>
        </div>
      </div>
    </aside>
  )
}

function StatusBar({ cursor, lspStatus, vim, freebuff, terminalPty, vimStatusRef, active, effects, sound }: { cursor: { line: number; column: number }; lspStatus: string; vim: boolean; freebuff: FreeBuffStatus | null; terminalPty: boolean | null; vimStatusRef: React.RefObject<HTMLSpanElement>; active: string | null; effects: boolean; sound: boolean }): JSX.Element {
  return (
    <footer className="statusbar">
      <span>{active ? active.split('/').pop() : 'Nova'}</span>
      <span>Ln {cursor.line}, Col {cursor.column}</span>
      <span>Spaces: 4</span>
      <span>UTF-8</span>
      <span>LF</span>
      <span className={lspStatus === 'ready' ? 'ok' : ''}>LSP: {lspStatus}</span>
      <span>Terminal: {terminalPty === null ? 'idle' : terminalPty ? 'pty' : 'pipe'}</span>
      <span>AI: {freebuff?.available ? 'terminal only' : 'unavailable'}</span>
      <span className={effects ? 'fx-on' : ''} title="Current-line lens and sparkles">FX: {effects ? 'on' : 'off'}</span>
      <span className={sound ? 'fx-on' : ''} title="Synthesized interface sounds">SFX: {sound ? 'on' : 'off'}</span>
      <div className="spacer" />
      {vim && <span className="vim-status" ref={vimStatusRef}>NORMAL</span>}
      <span>Nova</span>
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
