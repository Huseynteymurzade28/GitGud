mod git;

use git::{Branch, Commit, RepoInfo, Result, Status};
use std::path::Path;

// Commands are marked `async` so git runs on a worker thread instead of
// blocking the UI thread.

/// Repository passed on the command line, e.g. `gitgud ~/code/project`.
#[tauri::command]
fn initial_repo() -> Option<String> {
    std::env::args().nth(1)
}

#[tauri::command(async)]
fn open_repo(path: String) -> Result<RepoInfo> {
    git::open(Path::new(&path))
}

#[tauri::command(async)]
fn status(repo: String) -> Result<Status> {
    git::status(Path::new(&repo))
}

#[tauri::command(async)]
fn stage(repo: String, paths: Vec<String>) -> Result<()> {
    git::stage(Path::new(&repo), &paths)
}

#[tauri::command(async)]
fn unstage(repo: String, paths: Vec<String>) -> Result<()> {
    git::unstage(Path::new(&repo), &paths)
}

#[tauri::command(async)]
fn discard(repo: String, paths: Vec<String>) -> Result<()> {
    git::discard(Path::new(&repo), &paths)
}

#[tauri::command(async)]
fn commit(repo: String, message: String) -> Result<()> {
    git::commit(Path::new(&repo), &message)
}

#[tauri::command(async)]
fn diff(repo: String, path: String, staged: bool, untracked: bool) -> Result<String> {
    git::diff(Path::new(&repo), &path, staged, untracked)
}

#[tauri::command(async)]
fn branches(repo: String) -> Result<Vec<Branch>> {
    git::branches(Path::new(&repo))
}

#[tauri::command(async)]
fn switch_branch(repo: String, name: String) -> Result<()> {
    git::switch_branch(Path::new(&repo), &name)
}

#[tauri::command(async)]
fn create_branch(repo: String, name: String) -> Result<()> {
    git::create_branch(Path::new(&repo), &name)
}

#[tauri::command(async)]
fn log(repo: String, limit: u32) -> Result<Vec<Commit>> {
    git::log(Path::new(&repo), limit)
}

#[tauri::command(async)]
fn show_commit(repo: String, hash: String) -> Result<String> {
    git::show_commit(Path::new(&repo), &hash)
}

#[tauri::command(async)]
fn fetch(repo: String) -> Result<()> {
    git::fetch(Path::new(&repo))
}

#[tauri::command(async)]
fn pull(repo: String) -> Result<()> {
    git::pull(Path::new(&repo))
}

#[tauri::command(async)]
fn push(repo: String) -> Result<()> {
    git::push(Path::new(&repo))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            initial_repo,
            open_repo,
            status,
            stage,
            unstage,
            discard,
            commit,
            diff,
            branches,
            switch_branch,
            create_branch,
            log,
            show_commit,
            fetch,
            pull,
            push,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
