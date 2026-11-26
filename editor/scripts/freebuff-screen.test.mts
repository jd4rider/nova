import assert from 'node:assert/strict'
import { assistantText, transcriptLines } from '../src/main/freebuff-screen.ts'

const initial = [
  ' ╭────────────────────────────────────────────╮ ',
  ' │ You started Freebuff in a subdirectory.     │ ',
  ' ╰────────────────────────────────────────────╯ ',
  ' ███ Freebuff ███',
  ' ┌────────────────────────────────────────────┐ ',
  ' │ May use data for AI training                │ ',
  ' │ Your first message starts the session.      │ ',
  ' └────────────────────────────────────────────┘ ',
  ' ╭──────────────────────────────────────────╮ ',
  ' │ Your Freebuff chats, in your pocket       │ ',
  ' │ Get the mobile app  freebuff.com          │ ',
  ' ╰──────────────────────────────────────────╯ ',
  '╭──────────────────────────────────────────────╮',
  '│                                              │',
  '│  ▍Enter a coding task or / for commands      │',
  '│                                              │',
  '╰──────────────────────────────────────────────╯',
  ' DeepSeek V4.1 Flash Fast • high · /model to change · Chat: New chat',
  ' ← for history · ? for help'
]
assert.equal(transcriptLines(initial).length, 12)
assert.equal(assistantText(initial, '', []), '')

const chat = [
  ' You started Freebuff in a subdirectory.',
  ' ███ Freebuff ███',
  ' > Say OK',
  ' OK',
  '╭──────────────────────────────────────────────╮',
  '│  ▍Enter a coding task or / for commands      │',
  '╰──────────────────────────────────────────────╯',
  ' DeepSeek V4.1 Flash Fast · Chat: New chat',
  ' ← for history · ? for help'
]
assert.equal(assistantText(chat, 'Say OK', []), 'OK')
assert.equal(assistantText(chat, 'different prompt', chat.slice(0, 3)), 'OK')
console.log('freebuff-screen: 4 assertions passed')
