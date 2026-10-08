import { useState } from 'react'
import { Minus, Plus, Undo2 } from 'lucide-react'
import {
  git,
  isStaged,
  isUnstaged,
  type FileChange,
  type RepoInfo,
} from '../lib/git'
import type { Selection } from './RepoView'

interface Props {
  repo: RepoInfo
  files: FileChange[]
  selection: Selection | null
  onSelect: (s: Selection) => void
  busy: boolean
  act: (label: string, action: () => Promise<unknown>) => Promise<boolean>
}

export default function ChangesPanel({
  repo,
  files,
  selection,
  onSelect,
  busy,
  act,
}: Props) {
  const [message, setMessage] = useState('')
  const staged = files.filter(isStaged)
  const unstaged = files.filter(isUnstaged)
  const paths = (list: FileChange[]) => list.map((f) => f.path)

  async function commit() {
    if (!message.trim() || staged.length === 0) return
    if (await act('commit', () => git.commit(repo.path, message)))
      setMessage('')
  }

  function discard(file: FileChange) {
    if (
      window.confirm(`Discard changes to ${file.path}? This cannot be undone.`)
    )
      act('discard', () => git.discard(repo.path, [file.path]))
  }

  const isSelected = (f: FileChange, inStaged: boolean) =>
    selection?.kind === 'file' &&
    selection.staged === inStaged &&
    selection.file.path === f.path

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="min-h-0 flex-1 overflow-auto">
        <FileList
          title="Staged"
          files={staged}
          staged
          isSelected={isSelected}
          onSelect={(file) => onSelect({ kind: 'file', file, staged: true })}
          bulkLabel="Unstage all"
          onBulk={() =>
            act('unstage', () => git.unstage(repo.path, paths(staged)))
          }
          onToggle={(f) =>
            act('unstage', () => git.unstage(repo.path, [f.path]))
          }
        />
        <FileList
          title="Changes"
          files={unstaged}
          staged={false}
          isSelected={isSelected}
          onSelect={(file) => onSelect({ kind: 'file', file, staged: false })}
          bulkLabel="Stage all"
          onBulk={() =>
            act('stage', () => git.stage(repo.path, paths(unstaged)))
          }
          onToggle={(f) => act('stage', () => git.stage(repo.path, [f.path]))}
          onDiscard={discard}
        />
        {files.length === 0 && (
          <p className="p-6 text-center text-muted">No local changes</p>
        )}
      </div>

      <div className="border-t border-line p-3">
        <textarea
          value={message}
          onChange={(e) => setMessage(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) commit()
          }}
          placeholder="Commit message"
          rows={3}
          className="w-full resize-none rounded border border-line bg-bg p-2 outline-none select-text focus:border-accent"
        />
        <button
          onClick={commit}
          disabled={busy || !message.trim() || staged.length === 0}
          title="Ctrl+Enter"
          className="mt-2 w-full rounded bg-accent py-1.5 font-medium text-white hover:opacity-90 disabled:opacity-40"
        >
          Commit{' '}
          {staged.length > 0 &&
            `${staged.length} file${staged.length > 1 ? 's' : ''}`}
        </button>
      </div>
    </div>
  )
}

interface FileListProps {
  title: string
  files: FileChange[]
  staged: boolean
  isSelected: (f: FileChange, staged: boolean) => boolean
  onSelect: (f: FileChange) => void
  bulkLabel: string
  onBulk: () => void
  onToggle: (f: FileChange) => void
  onDiscard?: (f: FileChange) => void
}

function FileList({
  title,
  files,
  staged,
  isSelected,
  onSelect,
  bulkLabel,
  onBulk,
  onToggle,
  onDiscard,
}: FileListProps) {
  if (files.length === 0) return null
  return (
    <section>
      <div className="flex items-center justify-between px-3 pt-3 pb-1 text-xs font-semibold tracking-wide text-muted uppercase">
        <span>
          {title} ({files.length})
        </span>
        <button
          onClick={onBulk}
          className="font-normal tracking-normal normal-case hover:text-fg"
        >
          {bulkLabel}
        </button>
      </div>
      <ul>
        {files.map((f) => (
          <li
            key={f.path}
            onClick={() => onSelect(f)}
            className={`group flex cursor-default items-center gap-2 px-3 py-1 ${
              isSelected(f, staged) ? 'bg-hover' : 'hover:bg-hover/60'
            }`}
          >
            <StatusBadge file={f} staged={staged} />
            <span className="min-w-0 flex-1 truncate" title={f.path}>
              <span>{basename(f.path)}</span>
              <span className="ml-1.5 text-xs text-muted">
                {dirname(f.path)}
              </span>
            </span>
            {onDiscard && !f.untracked && !f.conflicted && (
              <RowButton title="Discard changes" onClick={() => onDiscard(f)}>
                <Undo2 className="size-3.5" />
              </RowButton>
            )}
            <RowButton
              title={staged ? 'Unstage' : 'Stage'}
              onClick={() => onToggle(f)}
            >
              {staged ? (
                <Minus className="size-3.5" />
              ) : (
                <Plus className="size-3.5" />
              )}
            </RowButton>
          </li>
        ))}
      </ul>
    </section>
  )
}

function RowButton({
  title,
  onClick,
  children,
}: {
  title: string
  onClick: () => void
  children: React.ReactNode
}) {
  return (
    <button
      title={title}
      onClick={(e) => {
        e.stopPropagation()
        onClick()
      }}
      className="invisible rounded p-0.5 text-muted group-hover:visible hover:bg-line hover:text-fg"
    >
      {children}
    </button>
  )
}

const LABELS: Record<string, [string, string]> = {
  M: ['M', 'text-modified'],
  A: ['A', 'text-added'],
  D: ['D', 'text-removed'],
  R: ['R', 'text-accent'],
  C: ['C', 'text-accent'],
  T: ['T', 'text-modified'],
  '?': ['U', 'text-added'],
}

function StatusBadge({ file, staged }: { file: FileChange; staged: boolean }) {
  const [letter, color] = file.conflicted
    ? ['!', 'text-removed']
    : (LABELS[staged ? file.index : file.worktree] ?? ['?', 'text-muted'])
  return (
    <span className={`w-3 text-center font-mono text-xs font-bold ${color}`}>
      {letter}
    </span>
  )
}

function basename(path: string) {
  return path.slice(path.lastIndexOf('/') + 1)
}

function dirname(path: string) {
  const i = path.lastIndexOf('/')
  return i === -1 ? '' : path.slice(0, i)
}
