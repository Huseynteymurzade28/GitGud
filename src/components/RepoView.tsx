import { useCallback, useEffect, useState } from 'react'
import {
  errorMessage,
  git,
  github,
  isStaged,
  isUnstaged,
  type Account,
  type Branch,
  type Commit,
  type FileChange,
  type RepoInfo,
  type Status,
} from '../lib/git'
import Toolbar from './Toolbar'
import ChangesPanel from './ChangesPanel'
import HistoryPanel from './HistoryPanel'
import DiffView from './DiffView'
import SignInDialog from './SignInDialog'
import PublishDialog from './PublishDialog'

type Tab = 'changes' | 'history'

/** What the right-hand pane is showing. */
export type Selection =
  | { kind: 'file'; file: FileChange; staged: boolean }
  | { kind: 'commit'; commit: Commit }

interface Props {
  repo: RepoInfo
  account: Account | null
  onAccountChange: (account: Account | null) => void
  onOpenOther: () => void
}

export default function RepoView({
  repo,
  account,
  onAccountChange,
  onOpenOther,
}: Props) {
  const [tab, setTab] = useState<Tab>('changes')
  const [status, setStatus] = useState<Status | null>(null)
  const [branches, setBranches] = useState<Branch[]>([])
  const [commits, setCommits] = useState<Commit[]>([])
  const [originUrl, setOriginUrl] = useState<string | null>(null)
  const [dialog, setDialog] = useState<'signIn' | 'publish' | null>(null)
  const [selection, setSelection] = useState<Selection | null>(null)
  const [diff, setDiff] = useState('')
  const [busy, setBusy] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)

  const refresh = useCallback(async () => {
    try {
      const [s, b, c, o] = await Promise.all([
        git.status(repo.path),
        git.branches(repo.path),
        git.log(repo.path),
        git.originUrl(repo.path),
      ])
      setStatus(s)
      setBranches(b)
      setCommits(c)
      setOriginUrl(o)
    } catch (e) {
      setError(errorMessage(e))
    }
  }, [repo.path])

  // Pick up changes made outside the app (editor saves, terminal commands).
  useEffect(() => {
    refresh()
    window.addEventListener('focus', refresh)
    return () => window.removeEventListener('focus', refresh)
  }, [refresh])

  // Drop the selection when the file is no longer in that list.
  useEffect(() => {
    if (selection?.kind !== 'file' || !status) return
    const { file, staged } = selection
    const stillThere = status.files.some(
      (f) => f.path === file.path && (staged ? isStaged(f) : isUnstaged(f)),
    )
    if (!stillThere) setSelection(null)
  }, [status, selection])

  useEffect(() => {
    let cancelled = false
    const load = async () => {
      if (!selection) return ''
      if (selection.kind === 'commit')
        return git.showCommit(repo.path, selection.commit.hash)
      const { file, staged } = selection
      return git.diff(repo.path, file.path, staged, file.untracked)
    }
    load()
      .then((d) => !cancelled && setDiff(d))
      .catch(
        (e) =>
          !cancelled && setDiff(`Could not load diff:\n${errorMessage(e)}`),
      )
    return () => {
      cancelled = true
    }
  }, [repo.path, selection, status])

  /** Runs a git action, shows its errors, and refreshes afterwards. */
  const act = useCallback(
    async (label: string, action: () => Promise<unknown>) => {
      setBusy(label)
      setError(null)
      try {
        await action()
        return true
      } catch (e) {
        setError(errorMessage(e))
        return false
      } finally {
        setBusy(null)
        await refresh()
      }
    },
    [refresh],
  )

  return (
    <div className="flex h-full flex-col">
      <Toolbar
        repo={repo}
        status={status}
        branches={branches}
        originUrl={originUrl}
        account={account}
        busy={busy}
        onOpenOther={onOpenOther}
        onSignIn={() => setDialog('signIn')}
        onSignOut={() =>
          act('signOut', async () => {
            await github.signOut()
            onAccountChange(null)
          })
        }
        onPublish={() => setDialog(account ? 'publish' : 'signIn')}
        act={act}
      />

      {dialog === 'signIn' && (
        <SignInDialog
          onClose={() => setDialog(null)}
          onSignedIn={(a) => {
            onAccountChange(a)
            setDialog(null)
          }}
        />
      )}
      {dialog === 'publish' && (
        <PublishDialog
          repo={repo}
          onClose={() => setDialog(null)}
          onPublished={() => {
            setDialog(null)
            refresh()
          }}
        />
      )}

      <div className="flex min-h-0 flex-1">
        <aside className="flex w-80 shrink-0 flex-col border-r border-line bg-panel">
          <div className="flex border-b border-line">
            {(['changes', 'history'] as const).map((t) => (
              <button
                key={t}
                onClick={() => {
                  setTab(t)
                  setSelection(null)
                }}
                className={`flex-1 py-2 font-medium capitalize ${
                  tab === t
                    ? 'border-b-2 border-accent text-fg'
                    : 'text-muted hover:text-fg'
                }`}
              >
                {t}
                {t === 'changes' && status && status.files.length > 0 && (
                  <span className="ml-1.5 rounded-full bg-hover px-1.5 text-xs">
                    {status.files.length}
                  </span>
                )}
              </button>
            ))}
          </div>

          {tab === 'changes' ? (
            <ChangesPanel
              repo={repo}
              files={status?.files ?? []}
              selection={selection}
              onSelect={setSelection}
              busy={busy !== null}
              act={act}
            />
          ) : (
            <HistoryPanel
              commits={commits}
              selection={selection}
              onSelect={setSelection}
            />
          )}
        </aside>

        <section className="min-w-0 flex-1 overflow-auto">
          {selection ? (
            <DiffView diff={diff} />
          ) : (
            <div className="flex h-full items-center justify-center text-muted">
              {tab === 'changes'
                ? 'Select a file to see its changes'
                : 'Select a commit to see what changed'}
            </div>
          )}
        </section>
      </div>

      {error && (
        <div className="flex items-start gap-3 border-t border-line bg-removed-bg px-4 py-2 text-removed">
          <pre className="flex-1 font-mono text-xs whitespace-pre-wrap select-text">
            {error}
          </pre>
          <button onClick={() => setError(null)} className="hover:underline">
            Dismiss
          </button>
        </div>
      )}
    </div>
  )
}
