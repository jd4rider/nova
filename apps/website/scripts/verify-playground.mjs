import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'

const here = import.meta.dirname
const wasmPath = resolve(here, '../public/playground/nova.wasm')
const bytes = readFileSync(wasmPath)
const module = new WebAssembly.Module(bytes)
const imports = WebAssembly.Module.imports(module)
if (imports.length) {
  console.error(`Playground wasm has unexpected imports: ${imports.map((i) => i.name).join(', ')}`)
  process.exit(1)
}

const instance = await WebAssembly.instantiate(module, {})
const exports = instance.exports
if (!(exports.memory instanceof WebAssembly.Memory)) {
  console.error('Playground wasm is missing exported memory')
  process.exit(1)
}

function check(source) {
  const input = new TextEncoder().encode(source)
  const allocation = Math.max(input.length, 1)
  const ptr = exports.nova_alloc(allocation)
  if (!ptr) throw new Error('nova_alloc returned a null pointer')
  if (input.length) {
    new Uint8Array(exports.memory.buffer, ptr, input.length).set(input)
  }
  const resultPtr = exports.nova_check(ptr, input.length)
  const resultLen = exports.nova_result_len()
  const result = JSON.parse(
    new TextDecoder().decode(
      new Uint8Array(exports.memory.buffer, resultPtr, resultLen)
    )
  )
  exports.nova_free(ptr, allocation)
  return result
}

const validSource = `import std.io { println }

fn main() {
    let answer = 6 * 7
    println("answer: \${answer}")
}
`
const valid = check(validSource)
if (!valid.ok || valid.stats.functions !== 1) {
  console.error('Valid Nova source did not pass the wasm front end:', valid)
  process.exit(1)
}
const fmtBytes = readFileSync(resolve(here, '../public/playground/nova-fmt.wasm'))
const fmtModule = new WebAssembly.Module(fmtBytes)
if (WebAssembly.Module.imports(fmtModule).length) {
  console.error('Formatter wasm has unexpected imports')
  process.exit(1)
}
const formatter = (await WebAssembly.instantiate(fmtModule, {})).exports
const fmtInput = new TextEncoder().encode(validSource)
const fmtPtr = Number(formatter.nova_input_ptr())
new Uint8Array(formatter.memory.buffer, fmtPtr, fmtInput.length).set(fmtInput)
formatter.nova_input_set(BigInt(fmtInput.length))
formatter.nova_reset_output()
formatter.main()
const formatted = new TextDecoder().decode(
  new Uint8Array(
    formatter.memory.buffer,
    Number(formatter.nova_output_ptr()),
    Number(formatter.nova_output_len())
  )
)
if (!formatted.includes('    let answer')) {
  console.error('The Nova formatter produced unexpected output:', formatted)
  process.exit(1)
}

const invalid = check('fn main() { let x: Int = "no" }')
const typeError = invalid.diagnostics.find((diagnostic) => diagnostic.code === 'E0308')
if (invalid.ok || !typeError) {
  console.error('Invalid Nova source did not produce E0308:', invalid)
  process.exit(1)
}
if (typeError.start.line !== 0 || typeError.start.character < 1) {
  console.error('Diagnostic did not carry usable UTF-16 positions:', typeError)
  process.exit(1)
}

console.log(
  `Playground wasm passed: ${bytes.length} bytes, valid check ${valid.stats.lines} lines, diagnostic ${typeError.code}.`
)
