import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'

// End-to-end check of the playground formatter wasm: format a messy source,
// assert the exact buffer, then format again to prove idempotence.
const here = import.meta.dirname
const bytes = readFileSync(resolve(here, '../public/playground/nova-fmt.wasm'))
const module = new WebAssembly.Module(bytes)
if (WebAssembly.Module.imports(module).length) {
  console.error('Formatter wasm has unexpected imports')
  process.exit(1)
}
const formatter = (await WebAssembly.instantiate(module, {})).exports

function format(source) {
  const input = new TextEncoder().encode(source)
  const ptr = Number(formatter.nova_input_ptr())
  new Uint8Array(formatter.memory.buffer, ptr, input.length).set(input)
  formatter.nova_input_set(BigInt(input.length))
  formatter.nova_reset_output()
  formatter.main()
  return new TextDecoder().decode(
    new Uint8Array(
      formatter.memory.buffer,
      Number(formatter.nova_output_ptr()),
      Number(formatter.nova_output_len())
    )
  )
}

const messy = 'fn main() {\nlet x = 1\nif x == 1 {\nprintln("hi")\n}\n}\n'
const expected = 'fn main() {\n    let x = 1\n    if x == 1 {\n        println("hi")\n    }\n}\n'
const once = format(messy)
if (once !== expected) {
  console.error('Formatter buffer mismatch:', JSON.stringify(once))
  process.exit(1)
}
if (format(once) !== expected) {
  console.error('Formatter is not idempotent')
  process.exit(1)
}
console.log(`Playground formatter passed: ${bytes.length} bytes, idempotent`)
