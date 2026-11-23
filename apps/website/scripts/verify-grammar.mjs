import { createHighlighter } from 'shiki'
import grammar from '../../../grammar/nova.tmLanguage.json' with { type: 'json' }

const fixture = `// line comment
/* block comment */
struct User {
    name: String
}

fn greet(name: String) -> String {
    let answer = 42
    return "Hello, \${name}!\\n"
}
`

const highlighter = await createHighlighter({
  themes: ['gruvbox-dark-hard'],
  langs: [{ ...grammar, name: 'nova' }]
})
const result = highlighter.codeToTokens(fixture, {
  lang: 'nova',
  theme: 'gruvbox-dark-hard',
  includeExplanation: true
})
const serialized = JSON.stringify(result.tokens)
const expected = [
  'comment.line.double-slash.nova',
  'comment.block.nova',
  'entity.name.type.struct.nova',
  'support.type.primitive.nova',
  'entity.name.function.nova',
  'constant.numeric.nova',
  'string.quoted.double.nova',
  'meta.embedded.interpolation.nova',
  'constant.character.escape.nova',
  'keyword.control.nova',
  'keyword.operator.nova'
]
const missing = expected.filter((scope) => !serialized.includes(scope))
if (missing.length) {
  console.error(`Grammar verification failed; missing scopes: ${missing.join(', ')}`)
  console.error(serialized.slice(0, 2000))
  process.exit(1)
}
console.log(`Nova grammar passed ${expected.length} scope checks.`)
