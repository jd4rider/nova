// Nova Wasm shell: loads the module, renders once, then re-renders on every
// element carrying data-action. The action code is written into state slot 1;
// Nova reads it in `main` and clears it after building the next view.

export async function createApp(wasmUrl, target, afterRender) {
  const response = await fetch(wasmUrl)
  if (!response.ok) throw new Error(`${response.status} ${response.statusText} for ${wasmUrl}`)
  const { instance } = await WebAssembly.instantiate(await response.arrayBuffer(), {})
  const exports = instance.exports
  const decoder = new TextDecoder()

  function render(action = 0) {
    exports.nova_reset_output()
    if (action) exports.nova_state_set(9n, BigInt(action))
    exports.main()
    const ptr = Number(exports.nova_output_ptr())
    const len = Number(exports.nova_output_len())
    target.innerHTML = decoder.decode(new Uint8Array(exports.memory.buffer, ptr, len))
    if (afterRender) afterRender(target)
  }

  target.addEventListener('click', (event) => {
    const el = event.target.closest('[data-action]')
    if (!el) return
    event.preventDefault()
    render(Number(el.dataset.action))
  })

  render()
  return render
}
