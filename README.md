# GitGud

A fast, lightweight, open-source Git GUI for Linux, Windows and macOS.

GitGud aims to be as approachable as GitHub Desktop while exposing more of Git's power: history, branches and (soon) rebase, stash and conflict resolution, without hiding what Git is doing.

> **Status:** early development (v0.1). Expect rough edges.

## Features

- Open any local repository (or pass it on the command line: `gitgud ~/code/project`)
- See changed files, stage and unstage per file or all at once, and discard changes
- Inline diff viewer with line numbers
- Commit (`Ctrl+Enter` in the message box)
- Create and switch branches
- Browse history and see each commit's changes
- Fetch, pull (fast-forward only) and push, including publishing new branches
- Sign in with GitHub (device flow, token kept in the OS keychain) to push over HTTPS
- Publish a local repository to GitHub in one step
- Light and dark themes that follow your system

## Tech stack

- **[Tauri 2](https://tauri.app)** — native shell, small binaries
- **Rust** backend (`src-tauri/`) that drives the `git` CLI without a shell, so file names and commit messages can never be run as commands
- **React + TypeScript + Tailwind CSS** frontend (`src/`), built with Vite

## Development

Prerequisites:

- [Rust](https://www.rust-lang.org/tools/install) (stable)
- [Node.js](https://nodejs.org) 20+
- `git` on your `PATH`
- Tauri's system dependencies for your OS: see [tauri.app/start/prerequisites](https://tauri.app/start/prerequisites/)

```bash
git clone https://github.com/Huseynteymurzade28/GitGud.git
cd GitGud
npm install
npm run tauri dev
```

To open a specific repository during development:

```bash
npm run tauri dev -- -- /path/to/repo
```

Other commands:

| Command                                           | What it does                       |
| ------------------------------------------------- | ---------------------------------- |
| `npm run tauri build`                             | Build a release bundle for your OS |
| `cargo test --manifest-path src-tauri/Cargo.toml` | Run the Rust tests                 |
| `npm run build`                                   | Type-check and build the frontend  |
| `npm run format`                                  | Format the frontend with Prettier  |

## Project layout

```
src/                  React frontend
  lib/git.ts          Typed wrappers for the Rust commands
  components/         UI components
src-tauri/
  src/git.rs          Git operations (runs the git CLI, parses its output)
  src/github.rs       GitHub sign-in, token storage and API calls
  src/lib.rs          Tauri commands exposed to the frontend
```

## Roadmap

- [ ] Clone repositories
- [ ] Stage individual lines and hunks
- [ ] Stash management
- [ ] Merge and conflict resolution
- [ ] Commit graph
- [ ] Interactive rebase with drag and drop
- [ ] Multiple open repositories
- [ ] GitHub / GitLab integration

## Contributing

Contributions are welcome. Open an issue to discuss larger changes before starting on them.

1. Fork the repository and create a branch: `git switch -c feature/my-feature`
2. Make your change; run `cargo test` and `npm run build`
3. Open a pull request

## License

[MIT](LICENSE)
