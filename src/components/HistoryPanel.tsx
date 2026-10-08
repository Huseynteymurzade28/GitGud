import type { Commit } from '../lib/git'
import { relativeTime } from '../lib/time'
import type { Selection } from './RepoView'

interface Props {
  commits: Commit[]
  selection: Selection | null
  onSelect: (s: Selection) => void
}

export default function HistoryPanel({ commits, selection, onSelect }: Props) {
  if (commits.length === 0)
    return <p className="p-6 text-center text-muted">No commits yet</p>

  return (
    <ul className="min-h-0 flex-1 overflow-auto">
      {commits.map((c) => {
        const selected =
          selection?.kind === 'commit' && selection.commit.hash === c.hash
        return (
          <li
            key={c.hash}
            onClick={() => onSelect({ kind: 'commit', commit: c })}
            className={`cursor-default border-b border-line px-3 py-2 ${
              selected ? 'bg-hover' : 'hover:bg-hover/60'
            }`}
          >
            <div className="truncate font-medium" title={c.subject}>
              {c.subject}
            </div>
            <div className="mt-0.5 flex gap-2 text-xs text-muted">
              <span className="truncate">{c.author}</span>
              <span>·</span>
              <span className="shrink-0">{relativeTime(c.time)}</span>
              <span className="ml-auto shrink-0 font-mono">{c.shortHash}</span>
            </div>
          </li>
        )
      })}
    </ul>
  )
}
