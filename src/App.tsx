import { useEffect, useState } from 'react'
import { open } from '@tauri-apps/plugin-dialog'
import { Download, FolderOpen, GitBranch } from 'lucide-react'
import {
  errorMessage,
  git,
  github,
  type Account,
  type RepoInfo,
} from './lib/git'
import RepoView from './components/RepoView'
import CloneDialog from './components/CloneDialog'

const LAST_REPO_KEY = 'gitgud.lastRepo'

function readLastRepo(): string | null {
  try {
    return localStorage.getItem(LAST_REPO_KEY)
  } catch {
    return null
  }
}

function writeLastRepo(path: string) {
  try {
    localStorage.setItem(LAST_REPO_KEY, path)
  } catch {
    // Remembering the repo is only a convenience.
  }
}

export default function App() {
  const [repo, setRepo] = useState<RepoInfo | null>(null)
  const [account, setAccount] = useState<Account | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [loading, setLoading] = useState(true)
  const [cloning, setCloning] = useState(false)

  function show(info: RepoInfo) {
    setRepo(info)
    setError(null)
    writeLastRepo(info.path)
  }

  async function openPath(path: string) {
    try {
      show(await git.openRepo(path))
    } catch (e) {
      setError(errorMessage(e))
    }
  }

  async function pickRepo() {
    const path = await open({ directory: true, title: 'Open repository' })
    if (typeof path === 'string') await openPath(path)
  }

  useEffect(() => {
    // Without keychain access the user just appears signed out.
    github
      .account()
      .then(setAccount)
      .catch(() => {})
  }, [])

  // Open the repo given on the command line, or else the last one used.
  useEffect(() => {
    git
      .initialRepo()
      .then(async (fromArgs) => {
        if (fromArgs) return openPath(fromArgs)
        const last = readLastRepo()
        if (last) setRepo(await git.openRepo(last).catch(() => null))
      })
      .finally(() => setLoading(false))
  }, [])

  if (loading) return null

  const cloneDialog = cloning && (
    <CloneDialog
      account={account}
      onAccountChange={setAccount}
      onClose={() => setCloning(false)}
      onCloned={(info) => {
        setCloning(false)
        show(info)
      }}
    />
  )

  if (repo)
    return (
      <>
        <RepoView
          // Remount so per-repo state (selection, diff) starts fresh.
          key={repo.path}
          repo={repo}
          account={account}
          onAccountChange={setAccount}
          onOpenOther={pickRepo}
          onClone={() => setCloning(true)}
        />
        {cloneDialog}
      </>
    )

  const button =
    'flex items-center gap-2 rounded-md px-4 py-2 font-medium hover:opacity-90'

  return (
    <main className="flex h-full flex-col items-center justify-center gap-6 p-8 text-center">
      <GitBranch className="size-12 text-accent" strokeWidth={1.5} />
      <div>
        <h1 className="text-2xl font-semibold">GitGud</h1>
        <p className="mt-1 text-muted">Open a Git repository to get started.</p>
      </div>
      <div className="flex gap-3">
        <button onClick={pickRepo} className={`${button} bg-accent text-white`}>
          <FolderOpen className="size-4" />
          Open repository
        </button>
        <button
          onClick={() => setCloning(true)}
          className={`${button} border border-line hover:bg-hover`}
        >
          <Download className="size-4" />
          Clone repository
        </button>
      </div>
      {error && <p className="max-w-md text-removed">{error}</p>}
      {cloneDialog}
    </main>
  )
}
