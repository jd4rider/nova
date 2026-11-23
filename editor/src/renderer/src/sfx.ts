/**
 * Tiny synthesized interface sounds. No audio files, network requests or
 * telemetry are involved. The context is created lazily on a user gesture.
 */
let context: AudioContext | null = null
let unavailable = false

function audioContext(): AudioContext | null {
  if (unavailable || typeof window === 'undefined') return null
  try {
    const Constructor = window.AudioContext ?? (window as typeof window & { webkitAudioContext?: typeof AudioContext }).webkitAudioContext
    if (!Constructor) {
      unavailable = true
      return null
    }
    context ??= new Constructor()
    if (context.state === 'suspended') void context.resume()
    return context
  } catch {
    unavailable = true
    return null
  }
}

function tone(startHz: number, endHz: number, duration: number, volume: number): void {
  if (volume <= 0) return
  const audio = audioContext()
  if (!audio) return
  try {
    const now = audio.currentTime
    const oscillator = audio.createOscillator()
    const gain = audio.createGain()
    oscillator.type = 'sine'
    oscillator.frequency.setValueAtTime(startHz, now)
    oscillator.frequency.exponentialRampToValueAtTime(Math.max(40, endHz), now + duration)
    const level = Math.min(0.045, 0.006 + (volume / 100) * 0.035)
    gain.gain.setValueAtTime(0.0001, now)
    gain.gain.exponentialRampToValueAtTime(level, now + 0.006)
    gain.gain.exponentialRampToValueAtTime(0.0001, now + duration)
    oscillator.connect(gain)
    gain.connect(audio.destination)
    oscillator.start(now)
    oscillator.stop(now + duration + 0.02)
  } catch {
    // Audio failure must never interrupt editing.
  }
}

export function playTyping(volume: number): void {
  tone(560 + Math.random() * 180, 340 + Math.random() * 90, 0.045, volume * 0.55)
}

export function playScroll(volume: number): void {
  tone(230 + Math.random() * 40, 150 + Math.random() * 30, 0.07, volume * 0.35)
}
