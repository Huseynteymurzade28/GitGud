import { useEffect, useRef, useState } from 'react'
import {
  ArrowDown,
  ArrowUp,
  Check,
  ChevronDown,
  FolderGit2,
  GitBranch,
  Loader2,
  Plus,
  RefreshCw,
} from 'lucide-react'
import { git, type Branch, type RepoInfo, type Status } from '../lib/git'

interface Props {
  repo: RepoInfo
  status: Status | null
  branches: Branch[]
  busy: string | null
  onOpenOther: () => void
  act: (label: string, action: () => Promise<unknown>) => Promise<boolean>
}

export default function Toolbar({
  repo,
  status,
  branches,
  busy,
  onOpenOther,
  act,
}: Props) {
  const ahead = status?.ahead ?? 0
  const behind = status?.behind ?? 0
  const hasUpstream = Boolean(status?.upstream)

  return (
    <header className="flex h-12 shrink-0 items-stretch border-b border-line">
      <ToolbarButton
        onClick={onOpenOther}
        label="Repository"
        value={repo.name}
        icon={<FolderGit2 className="size-4" />}
        title={repo.path}
      />
      <BranchMenu repo={repo} status={status} branches={branches} act={act} />

      <div className="flex-1" />

      <ToolbarButton
        onClick={() => act('fetch', () => git.fetch(repo.path))}
        disabled={busy !== null}
        label="Fetch"
        value="All remotes"
        icon={<Spin active={busy === 'fetch'} icon={RefreshCw} />}
      />
      <ToolbarButton
        onClick={() => act('pull', () => git.pull(repo.path))}
        disabled={busy !== null || !hasUpstream}
        label="Pull"
        value={behind > 0 ? `${behind} behind` : 'Up to date'}
        icon={<Spin active={busy === 'pull'} icon={ArrowDown} />}
      />
      <ToolbarButton
        onClick={() => act('push', () => git.push(repo.path))}
        disabled={busy !== null || !status?.branch}
        label={hasUpstream ? 'Push' : 'Publish'}
        value={
          hasUpstream
            ? ahead > 0
              ? `${ahead} ahead`
              : 'Up to date'
            : 'No upstream'
        }
        icon={<Spin active={busy === 'push'} icon={ArrowUp} />}
      />
    </header>
  )
}

function Spin({
  active,
  icon: Icon,
}: {
  active: boolean
  icon: typeof ArrowUp
}) {
  return active ? (
    <Loader2 className="size-4 animate-spin" />
  ) : (
    <Icon className="size-4" />
  )
}

interface ToolbarButtonProps {
  label: string
  value: string
  icon: React.ReactNode
  onClick: () => void
  disabled?: boolean
  title?: string
  trailing?: React.ReactNode
}

function ToolbarButton({
  label,
  value,
  icon,
  onClick,
  disabled,
  title,
  trailing,
}: ToolbarButtonProps) {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      title={title}
      className="flex min-w-36 items-center gap-3 border-r border-line px-4 text-left hover:bg-hover disabled:opacity-50 disabled:hover:bg-transparent"
    >
      <span className="text-muted">{icon}</span>
      <span className="flex min-w-0 flex-1 flex-col leading-tight">
        <span className="text-xs text-muted">{label}</span>
        <span className="truncate font-medium">{value}</span>
      </span>
      {trailing}
    </button>
  )
}

function BranchMenu({
  repo,
  status,
  branches,
  act,
}: Pick<Props, 'repo' | 'status' | 'branches' | 'act'>) {
  const [open, setOpen] = useState(false)
  const [newName, setNewName] = useState('')
  const ref = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!open) return
    const close = (e: MouseEvent) => {
      if (!ref.current?.contains(e.target as Node)) setOpen(false)
    }
    window.addEventListener('mousedown', close)
    return () => window.removeEventListener('mousedown', close)
  }, [open])

  async function switchTo(name: string) {
    setOpen(false)
    await act('switch', () => git.switchBranch(repo.path, name))
  }

  async function create(e: React.FormEvent) {
    e.preventDefault()
    const name = newName.trim()
    if (!name) return
    if (await act('branch', () => git.createBranch(repo.path, name))) {
      setNewName('')
      setOpen(false)
    }
  }

  return (
    <div ref={ref} className="relative flex">
      <ToolbarButton
        onClick={() => setOpen((o) => !o)}
        label="Current branch"
        value={status?.branch ?? 'Detached HEAD'}
        icon={<GitBranch className="size-4" />}
        trailing={<ChevronDown className="size-4 text-muted" />}
      />
      {open && (
        <div className="absolute top-full left-0 z-10 mt-1 w-72 overflow-hidden rounded-md border border-line bg-panel shadow-lg">
          <form
            onSubmit={create}
            className="flex gap-1 border-b border-line p-2"
          >
            <input
              autoFocus
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
              placeholder="New branch name"
              className="min-w-0 flex-1 rounded border border-line bg-bg px-2 py-1 outline-none focus:border-accent"
            />
            <button
              type="submit"
              title="Create branch"
              className="rounded px-2 text-muted hover:bg-hover hover:text-fg"
            >
              <Plus className="size-4" />
            </button>
          </form>
          <ul className="max-h-80 overflow-auto py-1">
            {branches.map((b) => (
              <li key={b.name}>
                <button
                  onClick={() => !b.current && switchTo(b.name)}
                  className="flex w-full items-center gap-2 px-3 py-1.5 text-left hover:bg-hover"
                >
                  <span className="w-4">
                    {b.current && <Check className="size-4 text-accent" />}
                  </span>
                  <span className="truncate">{b.name}</span>
                </button>
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  )
}
