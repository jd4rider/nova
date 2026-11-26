import { contextBridge, ipcRenderer } from 'electron'
import type { AiStreamEvent, NovaApi } from '../shared/types'

const api: NovaApi = {
  workspace: {
    choose: () => ipcRenderer.invoke('workspace:choose'),
    restore: () => ipcRenderer.invoke('workspace:restore'),
    root: () => ipcRenderer.invoke('workspace:root'),
    onChanged: (callback) => {
      const listener = (_event: Electron.IpcRendererEvent, payload: { paths: string[] }) => callback(payload)
      ipcRenderer.on('workspace:changed', listener)
      return () => ipcRenderer.removeListener('workspace:changed', listener)
    }
  },
  fs: {
    list: (dir) => ipcRenderer.invoke('fs:list', dir),
    read: (file) => ipcRenderer.invoke('fs:read', file),
    write: (file, content, expectedMtimeMs) => ipcRenderer.invoke('fs:write', file, content, expectedMtimeMs),
    create: (file, content) => ipcRenderer.invoke('fs:create', file, content),
    mkdir: (dir) => ipcRenderer.invoke('fs:mkdir', dir),
    rename: (from, to) => ipcRenderer.invoke('fs:rename', from, to),
    remove: (target) => ipcRenderer.invoke('fs:remove', target),
    reveal: (target) => ipcRenderer.invoke('fs:reveal', target),
    files: () => ipcRenderer.invoke('fs:files')
  },
  search: {
    run: (query, options) => ipcRenderer.invoke('search:run', query, options)
  },
  tasks: {
    run: (kind) => ipcRenderer.invoke('task:run', kind),
    cancel: () => ipcRenderer.invoke('task:cancel')
  },
  terminal: {
    create: (cwd, cols, rows, command) => ipcRenderer.invoke('terminal:create', cwd, cols, rows, command),
    write: (id, data) => ipcRenderer.invoke('terminal:write', id, data),
    resize: (id, cols, rows) => ipcRenderer.invoke('terminal:resize', id, cols, rows),
    dispose: (id) => ipcRenderer.invoke('terminal:dispose', id),
    onData: (callback) => {
      const listener = (_event: Electron.IpcRendererEvent, payload: { id: number; data: string }) => callback(payload)
      ipcRenderer.on('terminal:data', listener)
      return () => ipcRenderer.removeListener('terminal:data', listener)
    }
  },
  lsp: {
    start: (root) => ipcRenderer.invoke('lsp:start', root),
    stop: () => ipcRenderer.invoke('lsp:stop'),
    request: (method, params) => ipcRenderer.invoke('lsp:request', method, params),
    notify: (method, params) => ipcRenderer.invoke('lsp:notify', method, params),
    onDiagnostics: (callback) => {
      const listener = (_event: Electron.IpcRendererEvent, payload: { uri: string; diagnostics: any[] }) => callback(payload)
      ipcRenderer.on('lsp:diagnostics', listener)
      return () => ipcRenderer.removeListener('lsp:diagnostics', listener)
    },
    onStatus: (callback) => {
      const listener = (_event: Electron.IpcRendererEvent, status: string) => callback(status)
      ipcRenderer.on('lsp:status', listener)
      return () => ipcRenderer.removeListener('lsp:status', listener)
    }
  },
  ai: {
    settings: () => ipcRenderer.invoke('ai:settings'),
    configure: (patch) => ipcRenderer.invoke('ai:configure', patch),
    send: (requestId, messages, context) => ipcRenderer.invoke('ai:send', requestId, messages, context),
    cancel: (requestId) => ipcRenderer.invoke('ai:cancel', requestId),
    onEvent: (callback) => {
      const listener = (_event: Electron.IpcRendererEvent, payload: AiStreamEvent) => callback(payload)
      ipcRenderer.on('ai:event', listener)
      return () => ipcRenderer.removeListener('ai:event', listener)
    }
  },
  freebuff: {
    status: () => ipcRenderer.invoke('freebuff:status'),
    start: (root) => ipcRenderer.invoke('freebuff:start', root),
    send: (text) => ipcRenderer.invoke('freebuff:send', text),
    reset: () => ipcRenderer.invoke('freebuff:reset'),
    stop: () => ipcRenderer.invoke('freebuff:stop'),
    complete: (input) => ipcRenderer.invoke('freebuff:complete', input),
    onEvent: (callback) => {
      const listener = (_event: Electron.IpcRendererEvent, payload: any) => callback(payload)
      ipcRenderer.on('freebuff:event', listener)
      return () => ipcRenderer.removeListener('freebuff:event', listener)
    }
  },
  copilot: {
    status: () => ipcRenderer.invoke('copilot:status'),
    signIn: () => ipcRenderer.invoke('copilot:sign-in'),
    signOut: () => ipcRenderer.invoke('copilot:sign-out'),
    models: () => ipcRenderer.invoke('copilot:models'),
    complete: (input) => ipcRenderer.invoke('copilot:complete', input),
    onStatus: (callback) => {
      const listener = (_event: Electron.IpcRendererEvent, payload: any) => callback(payload)
      ipcRenderer.on('copilot:status', listener)
      return () => ipcRenderer.removeListener('copilot:status', listener)
    }
  },
  app: {
    version: () => ipcRenderer.invoke('app:version'),
    platform: process.platform,
    onMenu: (callback) => {
      const listener = (_event: Electron.IpcRendererEvent, command: string) => callback(command)
      ipcRenderer.on('menu:command', listener)
      return () => ipcRenderer.removeListener('menu:command', listener)
    }
  }
}

contextBridge.exposeInMainWorld('nova', api)
