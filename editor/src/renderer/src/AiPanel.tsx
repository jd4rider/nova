import { useCallback, useEffect, useRef, useState } from 'react'
import type * as Monaco from 'monaco-editor'
import { monaco } from './monaco'
import type {
  AiContext,
  AiMessage,
  AiProviderId,
  AiProviderSettings,
  CopilotModel,
  CopilotStatus,
  FreeBuffChatEvent,
  FreeBuffStatus
} from '../../shared/types'

interface ChatEntry {
  id: string
  role: 'user' | 'assistant'
  content: string
  pending?: boolean
  error?: boolean
}

interface FreeBuffEntry {
  id: string
  role: 'user' | 'assistant'
  content: string
  pending?: boolean
  error?: boolean
}

interface AiForm {
  provider: AiProviderId
  model: string
  baseUrl: string
}

interface EditProposal {
  entryId: string
  path: string
  original: string
  proposed: string
}

const AI_PRESETS: Array<{ label: string; provider: AiProviderId; model: string; baseUrl: string; hint: string }> = [
  { label: 'Copilot', provider: 'copilot', model: 'auto', baseUrl: 'https://github.com', hint: 'GitHub Copilot sign-in through the official SDK; no API key' },
  { label: 'Claude', provider: 'anthropic', model: 'claude-sonnet-4-5', baseUrl: 'https://api.anthropic.com', hint: 'Anthropic API key' },
  { label: 'GitHub Models', provider: 'openai', model: 'openai/gpt-4o-mini', baseUrl: 'https://models.github.ai/inference', hint: 'GitHub token with models:read — no separate AI vendor account' },
  { label: 'OpenAI', provider: 'openai', model: 'gpt-4o-mini', baseUrl: 'https://api.openai.com/v1', hint: 'OpenAI API key' },
  { label: 'Local', provider: 'openai', model: 'llama3.1', baseUrl: 'http://localhost:11434/v1', hint: 'Ollama or any local OpenAI-compatible server' }
]

function providerName(provider: AiProviderId): string {
  if (provider === 'anthropic') return 'Claude'
  if (provider === 'copilot') return 'GitHub Copilot'
  return 'OpenAI-compatible'
}

function extractEdit(text: string): { path: string | null; code: string } | null {
  const blocks: Array<{ path: string | null; code: string }> = []
  const pattern = /```([^\n]*)\n([\s\S]*?)```/g
  let match: RegExpExecArray | null
  while ((match = pattern.exec(text)) !== null) {
    const info = match[1].trim()
    const pathMatch = /(?:path|file)=["']?([^"'\s]+)/i.exec(info)
    blocks.push({ path: pathMatch?.[1] ?? null, code: match[2].replace(/\s+$/, '') + '\n' })
  }
  return blocks.length > 0 ? blocks[blocks.length - 1] : null
}

export function AiPanel({
  freebuff,
  root,
  activePath,
  activeModel,
  editor,
  onStatus,
  onTerminal,
  onKeepEdit
}: {
  freebuff: FreeBuffStatus | null
  root: string | null
  activePath: string | null
  activeModel: Monaco.editor.ITextModel | null
  editor: Monaco.editor.IStandaloneCodeEditor | null
  onStatus: (label: string) => void
  onTerminal: (command: string) => void
  onKeepEdit: (path: string, content: string) => Promise<void>
}): JSX.Element {
  const [tab, setTab] = useState<'chat' | 'freebuff'>('chat')
  const [settings, setSettings] = useState<AiProviderSettings | null>(null)
  const [form, setForm] = useState<AiForm>({ provider: 'anthropic', model: '', baseUrl: '' })
  const [apiKey, setApiKey] = useState('')
  const [configOpen, setConfigOpen] = useState(false)
  const [messages, setMessages] = useState<ChatEntry[]>([])
  const [input, setInput] = useState('')
  const [busy, setBusy] = useState(false)
  const [attach, setAttach] = useState(true)
  const [editMode, setEditMode] = useState(false)
  const [notice, setNotice] = useState<string | null>(null)
  const [copilot, setCopilot] = useState<CopilotStatus | null>(null)
  const [copilotModels, setCopilotModels] = useState<CopilotModel[]>([])
  const [proposal, setProposal] = useState<EditProposal | null>(null)
  const [kept, setKept] = useState<Set<string>>(new Set())
  const [fbMessages, setFbMessages] = useState<FreeBuffEntry[]>([])
  const [fbInput, setFbInput] = useState('')
  const [fbBusy, setFbBusy] = useState(false)
  const [fbStatus, setFbStatus] = useState('idle')
  const requestRef = useRef<string | null>(null)
  const scroller = useRef<HTMLDivElement | null>(null)
  const fbScroller = useRef<HTMLDivElement | null>(null)

  useEffect(() => {
    window.nova.ai
      .settings()
      .then((next) => {
        setSettings(next)
        setForm({ provider: next.provider, model: next.model, baseUrl: next.baseUrl })
      })
      .catch((error) => setNotice((error as Error).message))
  }, [])

  useEffect(() => {
    let active = true
    window.nova.copilot
      .status()
      .then((status) => { if (active) setCopilot(status) })
      .catch(() => undefined)
    const off = window.nova.copilot.onStatus((status) => { if (active) setCopilot(status) })
    return () => {
      active = false
      off()
    }
  }, [])

  useEffect(() => {
    if (!copilot?.authenticated) {
      setCopilotModels([])
      return
    }
    window.nova.copilot
      .models()
      .then((models) => {
        setCopilotModels(models)
        if (models.length > 0 && !models.some((model) => model.id === form.model)) {
          const preferred = models.find((model) => model.default) ?? models[0]
          setForm((value) => value.provider === 'copilot' ? { ...value, model: preferred.id } : value)
          if (settings?.provider === 'copilot' && settings.model !== preferred.id) {
            void window.nova.ai
              .configure({ provider: 'copilot', model: preferred.id })
              .then(setSettings)
              .catch(() => undefined)
          }
        }
      })
      .catch(() => undefined)
  }, [copilot?.authenticated, form.model, settings?.provider, settings?.model])

  useEffect(() => {
    if (!settings) return
    const name = providerName(settings.provider)
    const detail = settings.provider === 'copilot'
      ? copilot?.authenticated ? copilot.user ?? 'signed in' : 'sign in'
      : settings.hasApiKey ? settings.keySource : 'no key'
    onStatus(`${name} · ${settings.model}${detail ? ` · ${detail}` : ''}`)
  }, [settings, copilot, onStatus])

  useEffect(
    () =>
      window.nova.ai.onEvent((event) => {
        if (event.requestId !== requestRef.current) return
        if (event.type === 'delta' && event.text) {
          setMessages((previous) =>
            previous.map((entry) =>
              entry.id === event.requestId ? { ...entry, content: entry.content + (event.text ?? '') } : entry
            )
          )
        } else if (event.type === 'done') {
          requestRef.current = null
          setBusy(false)
          setMessages((previous) =>
            previous.map((entry) =>
              entry.id === event.requestId
                ? { ...entry, pending: false, content: entry.content || '(the provider returned no text)' }
                : entry
            )
          )
        } else if (event.type === 'error') {
          requestRef.current = null
          setBusy(false)
          setMessages((previous) =>
            previous.map((entry) =>
              entry.id === event.requestId
                ? { ...entry, pending: false, error: true, content: event.message ?? 'The request failed.' }
                : entry
            )
          )
        }
      }),
    []
  )

  useEffect(() => {
    scroller.current?.scrollTo({ top: scroller.current.scrollHeight })
  }, [messages])
  useEffect(() => {
    fbScroller.current?.scrollTo({ top: fbScroller.current.scrollHeight })
  }, [fbMessages])

  useEffect(() => {
    if (tab !== 'freebuff' || !root) return
    let active = true
    const off = window.nova.freebuff.onEvent((event: FreeBuffChatEvent) => {
      if (!active) return
      if (event.type === 'screen' && event.text) {
        setFbMessages((previous) => {
          const next = [...previous]
          const last = next[next.length - 1]
          if (last?.role === 'assistant') {
            next[next.length - 1] = { ...last, content: event.text ?? '', pending: false }
          } else {
            next.push({ id: crypto.randomUUID(), role: 'assistant', content: event.text ?? '' })
          }
          return next
        })
      } else if (event.type === 'status') {
        setFbStatus(event.detail ?? event.status ?? 'idle')
        setFbBusy(event.status === 'busy' || event.status === 'starting')
      } else if (event.type === 'error' || event.type === 'exit') {
        setFbBusy(false)
        setFbStatus('stopped')
        setFbMessages((previous) => [
          ...previous,
          { id: crypto.randomUUID(), role: 'assistant', content: event.message ?? 'FreeBuff stopped.', error: true }
        ])
      }
    })
    void window.nova.freebuff.start(root).catch((error) => {
      if (active) setFbMessages((previous) => [
        ...previous,
        { id: crypto.randomUUID(), role: 'assistant', content: (error as Error).message, error: true }
      ])
    })
    return () => {
      active = false
      off()
      void window.nova.freebuff.stop()
    }
  }, [tab, root])

  const buildContext = useCallback((): AiContext | null => {
    if (!attach || !activeModel || !activePath) return null
    const selection = editor?.getSelection()
    const selected = selection && !selection.isEmpty() ? activeModel.getValueInRange(selection) : undefined
    return { path: activePath, language: 'nova', content: activeModel.getValue(), selection: selected }
  }, [activeModel, activePath, attach, editor])

  const canSend = settings?.provider === 'copilot' ? Boolean(copilot?.authenticated) : Boolean(settings?.hasApiKey)

  const send = useCallback(async () => {
    const text = input.trim()
    if (!text || busy || !settings || !canSend) return
    const context = buildContext()
    const history: AiMessage[] = [
      ...messages.filter((entry) => !entry.pending && !entry.error).map((entry) => ({ role: entry.role, content: entry.content })),
      { role: 'user', content: editMode ? `${text}\n\nReturn the complete replacement file in one fenced code block.` : text }
    ]
    const requestId = crypto.randomUUID()
    requestRef.current = requestId
    setMessages((previous) => [
      ...previous,
      { id: `user-${requestId}`, role: 'user', content: text },
      { id: requestId, role: 'assistant', content: '', pending: true }
    ])
    setInput('')
    setBusy(true)
    setNotice(null)
    try {
      await window.nova.ai.send(requestId, history, context)
    } catch (error) {
      requestRef.current = null
      setBusy(false)
      setMessages((previous) =>
        previous.map((entry) =>
          entry.id === requestId ? { ...entry, pending: false, error: true, content: (error as Error).message } : entry
        )
      )
    }
  }, [buildContext, busy, canSend, editMode, input, messages, settings])

  const sendFreeBuff = useCallback(async () => {
    const text = fbInput.trim()
    if (!text || fbBusy || !root) return
    setFbMessages((previous) => [
      ...previous,
      { id: `user-${crypto.randomUUID()}`, role: 'user', content: text },
      { id: `assistant-${crypto.randomUUID()}`, role: 'assistant', content: '', pending: true }
    ])
    setFbInput('')
    setFbBusy(true)
    try {
      await window.nova.freebuff.send(text)
    } catch (error) {
      setFbBusy(false)
      setFbMessages((previous) => [
        ...previous,
        { id: crypto.randomUUID(), role: 'assistant', content: (error as Error).message, error: true }
      ])
    }
  }, [fbBusy, fbInput, root])

  const stop = () => {
    const requestId = requestRef.current
    if (!requestId) return
    void window.nova.ai.cancel(requestId)
    requestRef.current = null
    setBusy(false)
  }

  const saveConfig = async (keyValue: string) => {
    try {
      const next = await window.nova.ai.configure({
        provider: form.provider,
        model: form.model,
        baseUrl: form.provider === 'copilot' ? 'https://github.com' : form.baseUrl,
        ...(keyValue.trim() && form.provider !== 'copilot' ? { apiKey: keyValue } : {})
      })
      setSettings(next)
      setForm({ provider: next.provider, model: next.model, baseUrl: next.baseUrl })
      setApiKey('')
      setConfigOpen(false)
      setNotice(next.provider === 'copilot'
        ? 'Saved Copilot. Sign in below if needed.'
        : next.hasApiKey ? 'Saved. The key is encrypted with the OS credential store.' : 'Saved. Add an API key to send messages.')
    } catch (error) {
      setNotice((error as Error).message)
    }
  }

  const clearKey = async () => {
    try {
      setSettings(await window.nova.ai.configure({ apiKey: null }))
      setNotice('Stored key removed.')
    } catch (error) {
      setNotice((error as Error).message)
    }
  }

  const signIn = async () => {
    try {
      const result = await window.nova.copilot.signIn()
      if (result.userCode) {
        await navigator.clipboard.writeText(result.userCode).catch(() => undefined)
        setNotice(`Code ${result.userCode} copied. Enter it at ${result.verificationUri}.`)
      } else {
        setNotice(`Complete the sign-in at ${result.verificationUri}.`)
      }
      setConfigOpen(true)
    } catch (error) {
      setNotice((error as Error).message)
    }
  }

  const signOut = async () => {
    try {
      setCopilot(await window.nova.copilot.signOut())
      setNotice('Signed out of GitHub Copilot.')
    } catch (error) {
      setNotice((error as Error).message)
    }
  }

  const reviewEdit = (entry: ChatEntry) => {
    if (!activePath || !activeModel) {
      setNotice('Open a file before reviewing an AI edit.')
      return
    }
    const parsed = extractEdit(entry.content)
    if (!parsed) {
      setNotice('The reply did not contain a fenced code block to apply.')
      return
    }
    const target = parsed.path ?? activePath
    if (target !== activePath) {
      setNotice(`This edit targets ${target}; open that file first so the review is against its current contents.`)
      return
    }
    setProposal({ entryId: entry.id, path: target, original: activeModel.getValue(), proposed: parsed.code })
  }

  return (
    <aside className="aipanel">
      <div className="panel-header ai-header">
        <span>AI</span>
        <div className="ai-tabs">
          <button className={tab === 'chat' ? 'active' : ''} onClick={() => setTab('chat')}>Chat</button>
          <button className={tab === 'freebuff' ? 'active' : ''} onClick={() => setTab('freebuff')}>FreeBuff</button>
        </div>
      </div>
      {tab === 'freebuff' ? (
        <div className="ai-freebuff">
          <div className="ai-note">
            <div className="ai-note-row">
              <strong>{freebuff?.available ? `FreeBuff ${freebuff.version}` : 'FreeBuff unavailable'}</strong>
              <button className="ai-mini" onClick={() => onTerminal('freebuff')}>Raw TUI</button>
            </div>
            <p>{freebuff?.reason ?? 'Checking the installed CLI…'}</p>
            <p className="ai-scrape-status">Scrape: {fbStatus}</p>
          </div>
          {!root ? (
            <div className="empty-panel">Open a folder to run FreeBuff.</div>
          ) : (
            <>
              <div className="ai-messages" ref={fbScroller}>
                {fbMessages.length === 0 && (
                  <div className="ai-empty">
                    <p>This panel drives the real <code>freebuff</code> CLI in a PTY and reconstructs its screen as chat.</p>
                    <p className="muted">It is a scrape, not a FreeBuff API. Raw ANSI and TUI chrome are filtered heuristically; use Raw TUI for the untouched interface.</p>
                  </div>
                )}
                {fbMessages.map((entry) => (
                  <div key={entry.id} className={`ai-message ${entry.role} ${entry.error ? 'error' : ''}`}>
                    <div className="ai-message-role">{entry.role === 'user' ? 'You' : 'FreeBuff · scraped'}{entry.pending ? ' · working…' : ''}</div>
                    <div className="ai-message-text">{entry.content || (entry.pending ? '…' : '')}</div>
                  </div>
                ))}
              </div>
              <div className="ai-composer">
                <textarea
                  rows={3}
                  value={fbInput}
                  placeholder="Ask FreeBuff… Enter sends, Shift+Enter adds a newline"
                  onChange={(event) => setFbInput(event.target.value)}
                  onKeyDown={(event) => {
                    if (event.key === 'Enter' && !event.shiftKey) {
                      event.preventDefault()
                      void sendFreeBuff()
                    }
                  }}
                />
                <div className="ai-composer-actions">
                  <span className="muted">{fbBusy ? 'FreeBuff is working…' : 'Scraped session'}</span>
                  <div className="ai-action-group">
                    <button onClick={() => void window.nova.freebuff.reset()}>New chat</button>
                    <button className="primary" onClick={() => void sendFreeBuff()} disabled={!fbInput.trim() || fbBusy}>Send</button>
                  </div>
                </div>
              </div>
            </>
          )}
        </div>
      ) : (
        <div className="ai-chat">
          <div className="ai-configbar">
            <button className="ai-config-toggle" onClick={() => setConfigOpen((value) => !value)}>
              {settings ? `${providerName(settings.provider)} · ${settings.model}` : 'Loading provider…'}
            </button>
            <span className={`badge ${canSend ? 'ok' : 'warn'}`}>
              {settings
                ? settings.provider === 'copilot'
                  ? copilot?.authenticated ? copilot.user ?? 'signed in' : 'sign in'
                  : settings.hasApiKey ? settings.keySource : 'no key'
                : '…'}
            </span>
          </div>
          {configOpen && (
            <div className="ai-config">
              <div className="ai-presets">
                {AI_PRESETS.map((preset) => (
                  <button
                    key={preset.label}
                    title={preset.hint}
                    className={form.provider === preset.provider && form.model === preset.model ? 'active' : ''}
                    onClick={() => setForm({ provider: preset.provider, model: preset.model, baseUrl: preset.baseUrl })}
                  >
                    {preset.label}
                  </button>
                ))}
              </div>
              <label>Provider
                <select
                  value={form.provider}
                  onChange={(event) => {
                    const provider = event.target.value as AiProviderId
                    const preset = AI_PRESETS.find((candidate) => candidate.provider === provider)
                    setForm({ provider, model: preset?.model ?? '', baseUrl: preset?.baseUrl ?? '' })
                  }}
                >
                  <option value="copilot">GitHub Copilot</option>
                  <option value="anthropic">Anthropic (Claude)</option>
                  <option value="openai">OpenAI-compatible</option>
                </select>
              </label>
              <label>Model
                {form.provider === 'copilot' && copilotModels.length > 0 ? (
                  <select value={form.model} onChange={(event) => setForm({ ...form, model: event.target.value })}>
                    {copilotModels.map((model) => <option key={model.id} value={model.id}>{model.label}</option>)}
                  </select>
                ) : (
                  <input value={form.model} onChange={(event) => setForm({ ...form, model: event.target.value })} />
                )}
              </label>
              {form.provider === 'copilot' ? (
                <div className="copilot-box">
                  <div className="ai-note-row">
                    <strong>{copilot?.authenticated ? `Signed in as ${copilot.user ?? 'GitHub'}` : 'GitHub Copilot sign-in'}</strong>
                    <span className={`badge ${copilot?.authenticated ? 'ok' : 'warn'}`}>{copilot?.authenticated ? 'connected' : 'not signed in'}</span>
                  </div>
                  <p>{copilot?.message ?? 'Starting the Copilot SDK…'}</p>
                  {copilot?.deviceCode && <code className="copilot-code">{copilot.deviceCode}</code>}
                  <div className="ai-config-actions">
                    <button className="primary" onClick={() => void signIn()}>{copilot?.authenticated ? 'Re-authorize' : 'Sign in with GitHub'}</button>
                    {copilot?.authenticated && <button onClick={() => void signOut()}>Sign out</button>}
                  </div>
                  <p className="settings-note">
                    Chat and model discovery use GitHub's official Copilot SDK; inline ghost text uses the Copilot
                    language server. Sign-in uses GitHub's device flow, not a stored API key. The optional{' '}
                    <code>gh copilot</code> CLI extension is separate; install it to use it in the terminal.
                  </p>
                  <div className="ai-config-actions">
                    <button onClick={() => onTerminal('cmd:gh extension install github/gh-copilot')}>Install gh Copilot CLI</button>
                    <button onClick={() => onTerminal('cmd:gh auth status')}>Check gh login</button>
                  </div>
                </div>
              ) : (
                <>
                  <label>Base URL
                    <input value={form.baseUrl} onChange={(event) => setForm({ ...form, baseUrl: event.target.value })} />
                  </label>
                  <label>API key
                    <input
                      type="password"
                      value={apiKey}
                      autoComplete="off"
                      placeholder={settings?.hasApiKey ? `set (${settings.keySource})` : 'not set'}
                      onChange={(event) => setApiKey(event.target.value)}
                    />
                  </label>
                </>
              )}
              <div className="ai-config-actions">
                <button className="primary" onClick={() => void saveConfig(apiKey)}>Save</button>
                {settings?.keySource === 'stored' && <button onClick={() => void clearKey()}>Remove stored key</button>}
              </div>
              <p className="settings-note">
                Chat runs in Studio's main process. Anthropic/OpenAI keys are encrypted with the OS credential store and
                never enter the renderer. Copilot chat runs through the official SDK; inline completions use the Copilot
                language server and the GitHub account you authorize here.
              </p>
            </div>
          )}
          <div className="ai-messages" ref={scroller}>
            {messages.length === 0 && (
              <div className="ai-empty">
                <p>Ask about the open file, a diagnostic, or a Nova program. Chat uses Copilot when signed in, or the provider you configure.</p>
                <p className="muted">Enable <strong>Edit</strong> to have replies propose a full-file change you can review with a keep/rollback diff.</p>
              </div>
            )}
            {messages.map((entry) => {
              const parsed = entry.role === 'assistant' && !entry.pending && !entry.error ? extractEdit(entry.content) : null
              return (
                <div key={entry.id} className={`ai-message ${entry.role} ${entry.error ? 'error' : ''}`}>
                  <div className="ai-message-role">
                    {entry.role === 'user' ? 'You' : settings?.provider === 'copilot' ? `Copilot${copilot?.user ? ` · ${copilot.user}` : ''}` : 'Assistant'}
                    {entry.pending ? ' · streaming…' : ''}
                  </div>
                  <div className="ai-message-text">{entry.content || (entry.pending ? '…' : '')}</div>
                  {entry.role === 'assistant' && !entry.pending && !entry.error && (
                    <div className="ai-message-actions">
                      <button onClick={() => {
                        if (!editor) {
                          void navigator.clipboard.writeText(entry.content)
                          return
                        }
                        const selection = editor.getSelection()
                        if (!selection) return
                        editor.executeEdits('nova-ai', [{ range: selection, text: entry.content, forceMoveMarkers: true }])
                        editor.focus()
                      }}>Insert</button>
                      <button onClick={() => void navigator.clipboard.writeText(entry.content)}>Copy</button>
                      {editMode && parsed && activePath && (
                        <button className="primary" onClick={() => reviewEdit(entry)} disabled={kept.has(entry.id)}>
                          {kept.has(entry.id) ? 'Kept' : 'Review edit'}
                        </button>
                      )}
                    </div>
                  )}
                </div>
              )
            })}
          </div>
          {notice && <div className="ai-notice">{notice}</div>}
          <div className="ai-composer">
            <label className="ai-context-toggle">
              <input type="checkbox" checked={attach} onChange={(event) => setAttach(event.target.checked)} />
              attach {activePath ? activePath.split('/').pop() : 'current file (open one first)'}
            </label>
            <label className="ai-context-toggle">
              <input type="checkbox" checked={editMode} onChange={(event) => setEditMode(event.target.checked)} />
              edit mode: propose a reviewable full-file change
            </label>
            <textarea
              rows={3}
              value={input}
              placeholder={
                canSend
                  ? editMode ? 'Describe the change. The reply opens a keep/rollback diff.' : 'Ask about this file… Enter sends, Shift+Enter adds a newline'
                  : settings?.provider === 'copilot' ? 'Sign in with GitHub Copilot above.' : 'Add an API key above, or set ANTHROPIC_API_KEY / OPENAI_API_KEY.'
              }
              onChange={(event) => setInput(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === 'Enter' && !event.shiftKey) {
                  event.preventDefault()
                  void send()
                }
              }}
            />
            <div className="ai-composer-actions">
              <span className="muted">{busy ? 'Streaming…' : canSend ? 'Nothing is sent until you press Send.' : 'Sign in or add a key.'}</span>
              {busy ? (
                <button onClick={stop}>Stop</button>
              ) : (
                <button className="primary" onClick={() => void send()} disabled={!input.trim() || !canSend}>Send</button>
              )}
            </div>
          </div>
        </div>
      )}
      {proposal && (
        <DiffReview
          proposal={proposal}
          onRollback={() => setProposal(null)}
          onKeep={async () => {
            try {
              await onKeepEdit(proposal.path, proposal.proposed)
              setProposal(null)
              setKept((previous) => new Set(previous).add(proposal.entryId))
              setNotice(`Kept changes to ${proposal.path.split('/').pop()}.`)
            } catch (error) {
              setNotice(`Could not keep the edit: ${(error as Error).message}`)
              throw error
            }
          }}
        />
      )}
    </aside>
  )
}

function DiffReview({ proposal, onKeep, onRollback }: {
  proposal: EditProposal
  onKeep: () => Promise<void>
  onRollback: () => void
}): JSX.Element {
  const container = useRef<HTMLDivElement | null>(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    if (!container.current) return
    const stamp = `${Date.now()}-${Math.random().toString(16).slice(2)}`
    const original = monaco.editor.createModel(proposal.original, 'nova', monaco.Uri.parse(`inmemory://ai/original-${stamp}.nova`))
    const modified = monaco.editor.createModel(proposal.proposed, 'nova', monaco.Uri.parse(`inmemory://ai/modified-${stamp}.nova`))
    const diff = monaco.editor.createDiffEditor(container.current, {
      automaticLayout: true,
      readOnly: true,
      renderSideBySide: true,
      fontSize: 12,
      minimap: { enabled: false },
      scrollBeyondLastLine: false,
      renderOverviewRuler: false
    })
    diff.setModel({ original, modified })
    return () => {
      diff.dispose()
      original.dispose()
      modified.dispose()
    }
  }, [proposal])

  return (
    <div className="overlay ai-diff-overlay" onMouseDown={(event) => { if (event.target === event.currentTarget) onRollback() }}>
      <div className="ai-diff-card">
        <div className="ai-diff-header">
          <div>
            <strong>Review AI edit</strong>
            <span>{proposal.path}</span>
          </div>
          <span className="muted">Original ↔ proposed</span>
        </div>
        <div className="ai-diff-editor" ref={container} />
        {error && <div className="ai-notice">{error}</div>}
        <div className="ai-diff-actions">
          <button onClick={onRollback} disabled={busy}>Rollback</button>
          <button
            className="primary"
            disabled={busy}
            onClick={() => {
              setBusy(true)
              setError(null)
              void onKeep().catch((reason) => {
                setError((reason as Error).message)
                setBusy(false)
              })
            }}
          >
            {busy ? 'Keeping…' : 'Keep changes'}
          </button>
        </div>
      </div>
    </div>
  )
}
