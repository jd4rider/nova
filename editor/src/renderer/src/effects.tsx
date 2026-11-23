import { forwardRef, useCallback, useEffect, useImperativeHandle, useRef } from 'react'

export interface EffectsHandle {
  burst: (x: number, y: number, count?: number, kind?: 'type' | 'scroll') => void
  clear: () => void
}

const COLORS = ['var(--yellow)', 'var(--orange)', 'var(--aqua)', 'var(--purple)']

/**
 * A deliberately small DOM particle layer. It is pointer-transparent, bounded
 * and only mounted by the editor workbench; it never renders over Monaco's
 * suggestion widgets or captures input.
 */
export const EffectsLayer = forwardRef<EffectsHandle, { enabled: boolean }>(function EffectsLayer(
  { enabled },
  ref
) {
  const container = useRef<HTMLDivElement | null>(null)
  const enabledRef = useRef(enabled)
  enabledRef.current = enabled

  const clear = useCallback(() => {
    container.current?.replaceChildren()
  }, [])

  const burst = useCallback((x: number, y: number, count = 4, kind: 'type' | 'scroll' = 'type') => {
    const layer = container.current
    if (!enabledRef.current || !layer) return
    if (layer.childElementCount > 90) {
      layer.firstElementChild?.remove()
    }
    const width = layer.clientWidth
    const height = layer.clientHeight
    for (let index = 0; index < count; index += 1) {
      const spark = document.createElement('span')
      const spread = kind === 'scroll' ? 34 : 18
      const scale = kind === 'scroll' ? 1.3 : 0.85
      spark.className = 'effects-spark'
      spark.textContent = index % 3 === 0 ? '✦' : '·'
      spark.style.left = `${Math.min(width - 4, Math.max(4, x + (Math.random() - 0.5) * spread))}px`
      spark.style.top = `${Math.min(height - 4, Math.max(4, y + (Math.random() - 0.5) * spread))}px`
      spark.style.color = COLORS[Math.floor(Math.random() * COLORS.length)]
      spark.style.fontSize = `${6 + Math.random() * 7}px`
      spark.style.setProperty('--dx', `${(Math.random() - 0.5) * (kind === 'scroll' ? 120 : 46)}px`)
      spark.style.setProperty('--dy', `${-12 - Math.random() * (kind === 'scroll' ? 90 : 42)}px`)
      spark.style.setProperty('--rot', `${(Math.random() - 0.5) * 260}deg`)
      spark.style.setProperty('--scale', `${scale + Math.random() * 0.5}`)
      spark.style.animationDuration = `${kind === 'scroll' ? 720 : 560 + Math.random() * 180}ms`
      layer.appendChild(spark)
      spark.addEventListener('animationend', () => spark.remove(), { once: true })
    }
  }, [])

  useImperativeHandle(ref, () => ({ clear, burst }), [burst, clear])

  useEffect(() => {
    if (!enabled) clear()
  }, [clear, enabled])

  return <div className="effects-layer" ref={container} aria-hidden="true" />
})
