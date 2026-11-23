import { mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const here = dirname(fileURLToPath(import.meta.url))
const source = resolve(here, '../../../grammar/nova.tmLanguage.json')
const target = resolve(here, '../syntaxes/nova.tmLanguage.json')

mkdirSync(dirname(target), { recursive: true })
writeFileSync(target, readFileSync(source))
console.log(`synced ${source} -> ${target}`)
