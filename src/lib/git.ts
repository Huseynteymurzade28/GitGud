// Typed wrappers around the Rust commands in src-tauri/src/lib.rs.
import { invoke } from '@tauri-apps/api/core'

export interface RepoInfo {
  path: string
  name: string
}

export interface FileChange {
  path: string
  origPath: string | null
  index: string
  worktree: string
  untracked: boolean
  conflicted: boolean
}

export interface Status {
  branch: string | null
  upstream: string | null
  ahead: number
  behind: number
  files: FileChange[]
}

export interface Branch {
  name: string
  current: boolean
  upstream: string | null
}

export interface Commit {
  hash: string
  shortHash: string
  author: string
  email: string
  time: number
  subject: string
}

export const git = {
  initialRepo: () => invoke<string | null>('initial_repo'),
  openRepo: (path: string) => invoke<RepoInfo>('open_repo', { path }),
  status: (repo: string) => invoke<Status>('status', { repo }),
  stage: (repo: string, paths: string[]) =>
    invoke<void>('stage', { repo, paths }),
  unstage: (repo: string, paths: string[]) =>
    invoke<void>('unstage', { repo, paths }),
  discard: (repo: string, paths: string[]) =>
    invoke<void>('discard', { repo, paths }),
  commit: (repo: string, message: string) =>
    invoke<void>('commit', { repo, message }),
  diff: (repo: string, path: string, staged: boolean, untracked: boolean) =>
    invoke<string>('diff', { repo, path, staged, untracked }),
  branches: (repo: string) => invoke<Branch[]>('branches', { repo }),
  switchBranch: (repo: string, name: string) =>
    invoke<void>('switch_branch', { repo, name }),
  createBranch: (repo: string, name: string) =>
    invoke<void>('create_branch', { repo, name }),
  log: (repo: string, limit = 200) => invoke<Commit[]>('log', { repo, limit }),
  showCommit: (repo: string, hash: string) =>
    invoke<string>('show_commit', { repo, hash }),
  fetch: (repo: string) => invoke<void>('fetch', { repo }),
  pull: (repo: string) => invoke<void>('pull', { repo }),
  push: (repo: string) => invoke<void>('push', { repo }),
}

/** A change shown in the "Staged" list, i.e. something in the index. */
export const isStaged = (f: FileChange) =>
  !f.untracked && !f.conflicted && f.index !== '.'

/** A change shown in the "Changes" list, i.e. not yet in the index. */
export const isUnstaged = (f: FileChange) =>
  f.untracked || f.conflicted || f.worktree !== '.'

export function errorMessage(e: unknown): string {
  return typeof e === 'string' ? e : e instanceof Error ? e.message : String(e)
}
