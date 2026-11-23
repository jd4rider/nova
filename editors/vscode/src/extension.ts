import * as vscode from 'vscode'
import {
  LanguageClient,
  type LanguageClientOptions,
  type ServerOptions,
  TransportKind
} from 'vscode-languageclient/node'

let client: LanguageClient | undefined
let outputChannel: vscode.OutputChannel | undefined
let restarting = false

function configuredServerPath(): string {
  const configured = vscode.workspace
    .getConfiguration('nova')
    .get<string>('serverPath', 'nova')
  return configured.trim() || 'nova'
}

async function startClient(): Promise<void> {
  const output = outputChannel
  if (!output) return

  const command = configuredServerPath()
  const folder = vscode.workspace.workspaceFolders?.[0]
  const serverOptions: ServerOptions = {
    run: {
      command,
      args: ['lsp'],
      transport: TransportKind.stdio,
      options: { cwd: folder?.uri.fsPath }
    },
    debug: {
      command,
      args: ['lsp'],
      transport: TransportKind.stdio,
      options: { cwd: folder?.uri.fsPath }
    }
  }
  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: 'file', language: 'nova' }],
    ...(folder ? { workspaceFolder: folder } : {}),
    outputChannel: output,
    traceOutputChannel: output,
    synchronize: {
      fileEvents: vscode.workspace.createFileSystemWatcher('**/*.{nova,toml}')
    }
  }

  const next = new LanguageClient(
    'nova',
    'Nova Language Server',
    serverOptions,
    clientOptions
  )
  client = next
  output.appendLine(`[nova] starting "${command} lsp"`)
  next.onDidChangeState((event) => {
    output.appendLine(`[nova] state ${event.oldState} -> ${event.newState}`)
  })

  try {
    await next.start()
    output.appendLine('[nova] language server started')
  } catch (error) {
    if (client === next) client = undefined
    const message = error instanceof Error ? error.message : String(error)
    output.appendLine(`[nova] language server failed: ${message}`)
    const action = await vscode.window.showErrorMessage(
      `Nova language server failed to start: ${message}. Install the nova CLI or set nova.serverPath.`,
      'Open Settings',
      'Install Guide'
    )
    if (action === 'Open Settings') {
      await vscode.commands.executeCommand(
        'workbench.action.openSettings',
        'nova.serverPath'
      )
    } else if (action === 'Install Guide') {
      await vscode.env.openExternal(
        vscode.Uri.parse(
          'https://jd4rider.github.io/nova/getting-started/installation/'
        )
      )
    }
  }
}

async function stopClient(): Promise<void> {
  const current = client
  client = undefined
  if (current) await current.stop()
}

async function restartClient(): Promise<void> {
  if (restarting) return
  restarting = true
  try {
    await stopClient()
    await startClient()
  } finally {
    restarting = false
  }
}

export async function activate(context: vscode.ExtensionContext): Promise<void> {
  outputChannel = vscode.window.createOutputChannel('Nova Language Server')
  context.subscriptions.push(outputChannel)

  context.subscriptions.push(
    vscode.commands.registerCommand('nova.restartServer', async () => {
      await restartClient()
    })
  )
  context.subscriptions.push(
    vscode.workspace.onDidChangeConfiguration(async (event) => {
      if (event.affectsConfiguration('nova.serverPath')) {
        await restartClient()
      }
    })
  )

  await startClient()
}

export function deactivate(): Thenable<void> | undefined {
  return client?.stop()
}
