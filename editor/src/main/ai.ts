import type { AiMessage, AiProviderId } from '../shared/types'

export interface ProviderConfig {
  provider: AiProviderId
  model: string
  baseUrl: string
  apiKey: string
}

export const DEFAULT_MODELS: Record<AiProviderId, string> = {
  anthropic: 'claude-sonnet-4-5',
  openai: 'gpt-4o-mini',
  copilot: 'auto'
}

const DEFAULT_BASE_URLS: Record<AiProviderId, string> = {
  anthropic: 'https://api.anthropic.com',
  openai: 'https://api.openai.com/v1',
  copilot: 'https://github.com'
}

export function defaultBaseUrl(provider: AiProviderId): string {
  return DEFAULT_BASE_URLS[provider]
}

export function providerLabel(provider: AiProviderId): string {
  if (provider === 'anthropic') return 'Anthropic'
  if (provider === 'copilot') return 'GitHub Copilot'
  return 'OpenAI-compatible endpoint'
}

/**
 * Accepts either a host (`https://api.anthropic.com`), a versioned root
 * (`https://api.openai.com/v1`), or an inference root such as GitHub Models'
 * `https://models.github.ai/inference`, and returns the streaming endpoint.
 */
export function endpointFor(config: Pick<ProviderConfig, 'provider' | 'baseUrl'>): string {
  const base = (config.baseUrl || DEFAULT_BASE_URLS[config.provider]).replace(/\/+$/, '')
  if (config.provider === 'anthropic') {
    return base.endsWith('/v1') ? `${base}/messages` : `${base}/v1/messages`
  }
  return /\/(v1|inference)$/.test(base) ? `${base}/chat/completions` : `${base}/v1/chat/completions`
}

export function isAbortError(error: unknown): boolean {
  if (!(error instanceof Error)) return false
  return error.name === 'AbortError' || error.message === 'aborted' || error.message === 'The operation was aborted.'
}

interface ParsedChunk {
  text?: string
  model?: string
  error?: string
}

function parseChunk(provider: AiProviderId, payload: any): ParsedChunk {
  if (payload?.error) {
    const detail = typeof payload.error === 'string' ? payload.error : payload.error?.message
    return { error: String(detail ?? 'The provider reported an error.') }
  }
  if (provider === 'anthropic') {
    if (payload?.type === 'content_block_delta') {
      return { text: typeof payload.delta?.text === 'string' ? payload.delta.text : undefined, model: payload.message?.model }
    }
    return { model: payload?.message?.model }
  }
  const choice = payload?.choices?.[0]
  return {
    text: typeof choice?.delta?.content === 'string' ? choice.delta.content : undefined,
    model: typeof payload?.model === 'string' ? payload.model : undefined
  }
}

export async function streamChat(
  config: ProviderConfig,
  system: string,
  messages: AiMessage[],
  signal: AbortSignal,
  onDelta: (text: string) => void
): Promise<{ model: string }> {
  const headers: Record<string, string> = {
    'content-type': 'application/json',
    accept: 'text/event-stream'
  }
  let body: unknown
  if (config.provider === 'anthropic') {
    headers['x-api-key'] = config.apiKey
    headers['anthropic-version'] = '2023-06-01'
    body = {
      model: config.model,
      max_tokens: 4096,
      stream: true,
      system,
      messages: messages.map((message) => ({ role: message.role, content: message.content }))
    }
  } else {
    headers.authorization = `Bearer ${config.apiKey}`
    body = {
      model: config.model,
      stream: true,
      messages: [
        { role: 'system', content: system },
        ...messages.map((message) => ({ role: message.role, content: message.content }))
      ]
    }
  }

  const response = await fetch(endpointFor(config), {
    method: 'POST',
    headers,
    body: JSON.stringify(body),
    signal
  })

  if (!response.ok) {
    const detail = (await response.text().catch(() => '')).replace(/\s+/g, ' ').trim().slice(0, 400)
    throw new Error(
      `${providerLabel(config.provider)} rejected the request (HTTP ${response.status})${detail ? `: ${detail}` : ''}`
    )
  }
  if (!response.body) throw new Error('The provider returned an empty response body.')

  const reader = response.body.getReader()
  const decoder = new TextDecoder()
  let buffer = ''
  let model = config.model
  let streamError: string | null = null

  while (true) {
    const { value, done } = await reader.read()
    if (done) break
    buffer += decoder.decode(value, { stream: true })
    let newline = buffer.indexOf('\n')
    while (newline >= 0) {
      const line = buffer.slice(0, newline).replace(/\r$/, '')
      buffer = buffer.slice(newline + 1)
      newline = buffer.indexOf('\n')
      if (!line.startsWith('data:')) continue
      const payloadText = line.slice(5).trim()
      if (!payloadText || payloadText === '[DONE]') continue
      let payload: any
      try {
        payload = JSON.parse(payloadText)
      } catch {
        continue
      }
      const parsed = parseChunk(config.provider, payload)
      if (parsed.error) {
        streamError = parsed.error
        continue
      }
      if (parsed.model) model = parsed.model
      if (parsed.text) onDelta(parsed.text)
    }
  }

  if (streamError) throw new Error(streamError)
  return { model }
}
