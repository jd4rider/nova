'use strict'

const { execFileSync } = require('node:child_process')
const fs = require('node:fs')
const path = require('node:path')

function run(command, args) {
  execFileSync(command, args, { stdio: 'inherit' })
}

/**
 * Electron ships its framework linker-signed only, so the packed bundle's main
 * executable has an ad-hoc signature with `Sealed Resources=none`. macOS treats
 * that as damaged (`code has no resources but signature indicates they must be
 * present`) and refuses to launch the app. Re-signing the whole bundle ad hoc
 * writes `_CodeSignature/CodeResources`, sealing the main executable, the
 * helpers, the frameworks and the asar archive.
 *
 * This is not Developer ID signing or notarization, so Gatekeeper still asks the
 * user to confirm an unidentified developer. When a real identity is configured,
 * electron-builder signs the bundle itself and this hook stands down.
 */
module.exports = async function afterPack(context) {
  if (context.electronPlatformName !== 'darwin') return

  const macOptions = context.packager.platformSpecificBuildOptions ?? {}
  if (process.env.CSC_LINK || process.env.CSC_NAME || macOptions.identity) {
    console.log('[after-pack] signing identity configured; keeping electron-builder signature')
    return
  }

  const appName = context.packager.appInfo.productFilename
  const appPath = path.join(context.appOutDir, `${appName}.app`)
  if (!fs.existsSync(appPath)) {
    throw new Error(`[after-pack] missing app bundle: ${appPath}`)
  }

  console.log(`[after-pack] ad-hoc signing ${appPath}`)
  run('xattr', ['-cr', appPath])
  run('codesign', ['--force', '--deep', '--sign', '-', '--timestamp=none', appPath])
  run('codesign', ['--verify', '--deep', '--strict', '--verbose=2', appPath])
}
