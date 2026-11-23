import { app, safeStorage } from 'electron'
import fs from 'node:fs'
import path from 'node:path'
import type { AiConfigurePatch, AiProviderId, AiProviderSettings } from '../shared/types'
import { DEFAULT_MODELS, defaultBaseUrl } from './ai'

interface StoredAi {
  provider: AiProviderId
  models: Partial<Record<AiProviderId, string>>
  baseUrls: Partial<Record<AiProviderId, string>>
  keys: Partial<Record<AiProviderId, string>>
}

const PROVIDERS: AiProviderId[] = ['anthropic', 'openai']
const ENVIRONMENT_VARIABLES: Record<AiProviderId, string[]> = {
  anthropic: ['ANTHROPIC_API_KEY'],
  openai: ['OPENAI_API_KEY', 'GITHUB_TOKEN', 'GITHUB_MODELS_TOKEN']
}

function settingsFile(): string {
  return path.join(app.getPath('userData'), 'ai.json')
}

function read(): StoredAi {
  try {
    const parsed = JSON.parse(fs.readFileSync(settingsFile(), 'utf8')) as Partial<StoredAi>
    return {
      provider: PROVIDERS.includes(parsed.provider as AiProviderId) ? (parsed.provider as AiProviderId) : 'anthropic',
      models: typeof parsed.models === 'object' && parsed.models ? parsed.models : {},
      baseUrls: typeof parsed.baseUrls === 'object' && parsed.baseUrls ? parsed.baseUrls : {},
      keys: typeof parsed.keys === 'object' && parsed.keys ? parsed.keys : {}
    }
  } catch {
    return { provider: 'anthropic', models: {}, baseUrls: {}, keys: {} }
  }
}

function write(next: StoredAi): void {
  const target = settingsFile()
  fs.mkdirSync(path.dirname(target), { recursive: true })
  const temporary = `${target}.${process.pid}.tmp`
  fs.writeFileSync(temporary, JSON.stringify(next, null, 2), { encoding: 'utf8', mode: 0o600 })
  fs.renameSync(temporary, target)
  try {
    fs.chmodSync(target, 0o600)
  } catch {
    /* best effort on filesystems without POSIX modes */
  }
}

function storedKey(stored: StoredAi, provider: AiProviderId): string | null {
  const encrypted = stored.keys[provider]
  if (!encrypted || !safeStorage.isEncryptionAvailable()) return null
  try {
    return safeStorage.decryptString(Buffer.from(encrypted, 'base64'))
  } catch {
    return null
  }
}

function environmentKey(provider: AiProviderId): string | null {
  for (const name of ENVIRONMENT_VARIABLES[provider]) {
    const value = process.env[name]?.trim()
    if (value) return value
  }
  return null
}

export function getAiSettings(): AiProviderSettings {
  const stored = read()
  const provider = stored.provider
  return {
    provider,
    model: stored.models[provider]?.trim() || DEFAULT_MODELS[provider],
    baseUrl: stored.baseUrls[provider]?.trim() || defaultBaseUrl(provider),
    hasApiKey: Boolean(storedKey(stored, provider) ?? environmentKey(provider)),
    keySource: storedKey(stored, provider) ? 'stored' : environmentKey(provider) ? 'environment' : 'none'
  }
}

export function configureAi(patch: AiConfigurePatch): AiProviderSettings {
  const stored = read()
  if (patch.provider !== undefined) {
    if (!PROVIDERS.includes(patch.provider)) throw new Error(`Unsupported provider: ${String(patch.provider)}`)
    stored.provider = patch.provider
  }
  const provider = stored.provider

  if (patch.model !== undefined) {
    const model = patch.model.trim().slice(0, 120)
    if (model) stored.models[provider] = model
    else delete stored.models[provider]
  }

  if (patch.baseUrl !== undefined) {
    const baseUrl = patch.baseUrl.trim().slice(0, 300)
    if (baseUrl && !/^https?:\/\//i.test(baseUrl)) {
      throw new Error('Base URL must start with http:// or https://')
    }
    if (baseUrl) stored.baseUrls[provider] = baseUrl
    else delete stored.baseUrls[provider]
  }

  if (patch.apiKey !== undefined) {
    const apiKey = patch.apiKey?.trim() ?? ''
    if (!apiKey) {
      delete stored.keys[provider]
    } else {
      if (!safeStorage.isEncryptionAvailable()) {
        throw new Error(
          'This system has no OS credential storage, so Nova Studio will not save the key. Set ANTHROPIC_API_KEY or OPENAI_API_KEY in the environment instead.'
        )
      }
      stored.keys[provider] = safeStorage.encryptString(apiKey).toString('base64')
    }
  }

  write(stored)
  return getAiSettings()
}

export function resolveAiConfig(): { provider: AiProviderId; model: string; baseUrl: string; apiKey: string } {
  const stored = read()
  const provider = stored.provider
  const apiKey = storedKey(stored, provider) ?? environmentKey(provider)
  if (!apiKey) {
    const variable = ENVIRONMENT_VARIABLES[provider][0]
    throw new Error(`No API key configured for ${provider}. Add one in the AI panel or set ${variable}.`)
  }
  return {
    provider,
    model: stored.models[provider]?.trim() || DEFAULT_MODELS[provider],
    baseUrl: stored.baseUrls[provider]?.trim() || defaultBaseUrl(provider),
    apiKey
  }
}
