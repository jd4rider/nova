import { contextBridge, ipcRenderer } from 'electron'
import type { NovaApi } from '../shared/types'

const api: NovaApi = {
  workspace: {
    choose: () => ipcRenderer.invoke('workspace:choose'),
    root: () => ipcRenderer.invoke('workspace:root')
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
  freebuff: {
    status: () => ipcRenderer.invoke('freebuff:status')
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
