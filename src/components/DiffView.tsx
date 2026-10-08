import { useMemo } from 'react'

type LineKind = 'add' | 'remove' | 'context' | 'hunk' | 'meta'

interface Line {
  kind: LineKind
  text: string
  oldNo?: number
  newNo?: number
}

const HUNK = /^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@/

/** Turns unified diff text into lines with old/new line numbers. */
function parse(diff: string): Line[] {
  const lines: Line[] = []
  let oldNo = 0
  let newNo = 0
  let inHunk = false

  for (const text of diff.split('\n')) {
    const hunk = HUNK.exec(text)
    if (hunk) {
      oldNo = Number(hunk[1])
      newNo = Number(hunk[2])
      inHunk = true
      lines.push({ kind: 'hunk', text })
    } else if (text.startsWith('diff --git')) {
      inHunk = false
      lines.push({ kind: 'meta', text })
    } else if (!inHunk) {
      lines.push({ kind: 'meta', text })
    } else if (text.startsWith('+')) {
      lines.push({ kind: 'add', text: text.slice(1), newNo: newNo++ })
    } else if (text.startsWith('-')) {
      lines.push({ kind: 'remove', text: text.slice(1), oldNo: oldNo++ })
    } else if (text.startsWith('\\')) {
      lines.push({ kind: 'meta', text })
    } else {
      lines.push({
        kind: 'context',
        text: text.slice(1),
        oldNo: oldNo++,
        newNo: newNo++,
      })
    }
  }

  while (lines.length && lines[lines.length - 1].text === '') lines.pop()
  return lines
}

const ROW: Record<LineKind, string> = {
  add: 'bg-added-bg',
  remove: 'bg-removed-bg',
  context: '',
  hunk: 'bg-panel text-accent',
  meta: 'text-muted',
}

const SIGN: Record<LineKind, string> = {
  add: '+',
  remove: '-',
  context: ' ',
  hunk: '',
  meta: '',
}

export default function DiffView({ diff }: { diff: string }) {
  const lines = useMemo(() => parse(diff), [diff])

  if (!diff.trim())
    return (
      <p className="p-6 text-muted">
        No textual changes (binary file or mode change).
      </p>
    )

  return (
    <table className="w-full border-collapse font-mono text-xs leading-5 select-text">
      <tbody>
        {lines.map((line, i) => (
          <tr key={i} className={ROW[line.kind]}>
            <td className="w-12 pr-2 text-right text-muted/70 select-none">
              {line.oldNo}
            </td>
            <td className="w-12 border-r border-line pr-2 text-right text-muted/70 select-none">
              {line.newNo}
            </td>
            <td
              className={`w-5 text-center select-none ${
                line.kind === 'add'
                  ? 'text-added'
                  : line.kind === 'remove'
                    ? 'text-removed'
                    : ''
              }`}
            >
              {SIGN[line.kind]}
            </td>
            <td className="pr-4 whitespace-pre">{line.text}</td>
          </tr>
        ))}
      </tbody>
    </table>
  )
}
