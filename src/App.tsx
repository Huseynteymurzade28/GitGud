import { useEffect, useState } from 'react'
import { open } from '@tauri-apps/plugin-dialog'
import { FolderOpen, GitBranch } from 'lucide-react'
import {
  errorMessage,
  git,
  github,
  type Account,
  type RepoInfo,
} from './lib/git'
import RepoView from './components/RepoView'

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

  async function openPath(path: string) {
    try {
      const info = await git.openRepo(path)
      setRepo(info)
      setError(null)
      writeLastRepo(info.path)
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

  if (repo)
    return (
      <RepoView
        repo={repo}
        account={account}
        onAccountChange={setAccount}
        onOpenOther={pickRepo}
      />
    )

  return (
    <main className="flex h-full flex-col items-center justify-center gap-6 p-8 text-center">
      <GitBranch className="size-12 text-accent" strokeWidth={1.5} />
      <div>
        <h1 className="text-2xl font-semibold">GitGud</h1>
        <p className="mt-1 text-muted">Open a Git repository to get started.</p>
      </div>
      <button
        onClick={pickRepo}
        className="flex items-center gap-2 rounded-md bg-accent px-4 py-2 font-medium text-white hover:opacity-90"
      >
        <FolderOpen className="size-4" />
        Open repository
      </button>
      {error && <p className="max-w-md text-removed">{error}</p>}
    </main>
  )
}
