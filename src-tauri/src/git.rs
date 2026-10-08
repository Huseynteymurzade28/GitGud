//! Thin wrapper around the `git` CLI.
//!
//! Every call goes through [`run`], which passes arguments directly to the
//! process (no shell), so user input such as commit messages or file names
//! can never be interpreted as commands.

use serde::Serialize;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

#[derive(Debug, thiserror::Error)]
pub enum GitError {
    #[error("git could not be started: {0}")]
    Spawn(#[from] std::io::Error),
    #[error("{0}")]
    Failed(String),
    #[error("not a git repository: {0}")]
    NotARepo(String),
}

// Tauri sends command errors to the frontend as JSON, so serialize as a plain message.
impl Serialize for GitError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, GitError>;

fn command(repo: &Path, args: &[&str]) -> Command {
    let mut cmd = Command::new("git");
    cmd.current_dir(repo)
        .args(args)
        // Fail instead of hanging on an invisible credential prompt.
        .env("GIT_TERMINAL_PROMPT", "0")
        // Don't take locks for read-only commands like `status`.
        .env("GIT_OPTIONAL_LOCKS", "0")
        // Stable, parseable output regardless of the user's locale.
        .env("LC_ALL", "C");

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    cmd
}

fn output(repo: &Path, args: &[&str]) -> Result<Output> {
    Ok(command(repo, args).output()?)
}

/// Runs git and returns stdout, or stderr as the error when git exits non-zero.
fn run(repo: &Path, args: &[&str]) -> Result<String> {
    run_with_env(repo, args, &[])
}

/// Like [`run`], with extra environment variables (used for credentials).
fn run_with_env(repo: &Path, args: &[&str], env: &[(String, String)]) -> Result<String> {
    let out = command(repo, args).envs(env.iter().cloned()).output()?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        Err(GitError::Failed(if stderr.is_empty() {
            format!("git {} failed", args.first().unwrap_or(&""))
        } else {
            stderr
        }))
    }
}

fn has_head(repo: &Path) -> Result<bool> {
    Ok(output(repo, &["rev-parse", "--verify", "-q", "HEAD"])?
        .status
        .success())
}

// ---------------------------------------------------------------------------
// Repository

#[derive(Debug, Serialize)]
pub struct RepoInfo {
    pub path: String,
    pub name: String,
}

pub fn open(path: &Path) -> Result<RepoInfo> {
    let root = run(path, &["rev-parse", "--show-toplevel"])
        .map_err(|_| GitError::NotARepo(path.display().to_string()))?;
    let root = root.trim().to_string();
    let name = Path::new(&root)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| root.clone());
    Ok(RepoInfo { path: root, name })
}

// ---------------------------------------------------------------------------
// Status

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FileChange {
    pub path: String,
    /// Original path for renames and copies.
    pub orig_path: Option<String>,
    /// Status in the index (staged), e.g. 'M', 'A', 'D', 'R', or '.' for unchanged.
    pub index: char,
    /// Status in the working tree (unstaged), same codes as `index`.
    pub worktree: char,
    pub untracked: bool,
    pub conflicted: bool,
}

#[derive(Debug, Serialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    /// `None` when HEAD is detached.
    pub branch: Option<String>,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub files: Vec<FileChange>,
}

pub fn status(repo: &Path) -> Result<Status> {
    let raw = run(
        repo,
        &[
            "status",
            "--porcelain=v2",
            "--branch",
            "-z",
            "--untracked-files=all",
        ],
    )?;
    Ok(parse_status(&raw))
}

fn xy(field: &str) -> (char, char) {
    let mut chars = field.chars();
    (chars.next().unwrap_or('.'), chars.next().unwrap_or('.'))
}

fn parse_status(raw: &str) -> Status {
    let mut status = Status::default();
    let mut entries = raw.split('\0').filter(|e| !e.is_empty());

    while let Some(entry) = entries.next() {
        if let Some(header) = entry.strip_prefix("# ") {
            let (key, value) = header.split_once(' ').unwrap_or((header, ""));
            match key {
                "branch.head" if value != "(detached)" => status.branch = Some(value.into()),
                "branch.upstream" => status.upstream = Some(value.into()),
                "branch.ab" => {
                    for part in value.split(' ') {
                        if let Some(n) = part.strip_prefix('+') {
                            status.ahead = n.parse().unwrap_or(0);
                        } else if let Some(n) = part.strip_prefix('-') {
                            status.behind = n.parse().unwrap_or(0);
                        }
                    }
                }
                _ => {}
            }
            continue;
        }

        let kind = entry.as_bytes()[0];
        match kind {
            // 1 XY sub mH mI mW hH hI path
            b'1' => {
                let f: Vec<&str> = entry.splitn(9, ' ').collect();
                if f.len() == 9 {
                    let (index, worktree) = xy(f[1]);
                    status.files.push(FileChange {
                        path: f[8].into(),
                        orig_path: None,
                        index,
                        worktree,
                        untracked: false,
                        conflicted: false,
                    });
                }
            }
            // 2 XY sub mH mI mW hH hI Xscore path, followed by origPath as its own entry
            b'2' => {
                let f: Vec<&str> = entry.splitn(10, ' ').collect();
                let orig = entries.next().map(String::from);
                if f.len() == 10 {
                    let (index, worktree) = xy(f[1]);
                    status.files.push(FileChange {
                        path: f[9].into(),
                        orig_path: orig,
                        index,
                        worktree,
                        untracked: false,
                        conflicted: false,
                    });
                }
            }
            // u XY sub m1 m2 m3 mW h1 h2 h3 path
            b'u' => {
                let f: Vec<&str> = entry.splitn(11, ' ').collect();
                if f.len() == 11 {
                    let (index, worktree) = xy(f[1]);
                    status.files.push(FileChange {
                        path: f[10].into(),
                        orig_path: None,
                        index,
                        worktree,
                        untracked: false,
                        conflicted: true,
                    });
                }
            }
            b'?' => status.files.push(FileChange {
                path: entry[2..].into(),
                orig_path: None,
                index: '.',
                worktree: '?',
                untracked: true,
                conflicted: false,
            }),
            _ => {}
        }
    }

    status
}

// ---------------------------------------------------------------------------
// Staging and committing

pub fn stage(repo: &Path, paths: &[String]) -> Result<()> {
    let mut args = vec!["add", "-A", "--"];
    args.extend(paths.iter().map(String::as_str));
    run(repo, &args).map(drop)
}

pub fn unstage(repo: &Path, paths: &[String]) -> Result<()> {
    // `restore --staged` needs a HEAD to restore from; before the first
    // commit, removing the paths from the index has the same effect.
    let mut args = if has_head(repo)? {
        vec!["restore", "--staged", "--"]
    } else {
        vec!["rm", "--cached", "-r", "-q", "--"]
    };
    args.extend(paths.iter().map(String::as_str));
    run(repo, &args).map(drop)
}

pub fn discard(repo: &Path, paths: &[String]) -> Result<()> {
    let mut args = vec!["restore", "--worktree", "--"];
    args.extend(paths.iter().map(String::as_str));
    run(repo, &args).map(drop)
}

pub fn commit(repo: &Path, message: &str) -> Result<()> {
    if message.trim().is_empty() {
        return Err(GitError::Failed("Commit message cannot be empty".into()));
    }
    run(repo, &["commit", "-m", message]).map(drop)
}

// ---------------------------------------------------------------------------
// Diff

pub fn diff(repo: &Path, path: &str, staged: bool, untracked: bool) -> Result<String> {
    if untracked {
        // `--no-index` exits with 1 when the files differ, which is always the
        // case here, so read the output regardless of the exit code.
        let out = output(
            repo,
            &["diff", "--no-color", "--no-index", "--", "/dev/null", path],
        )?;
        return Ok(String::from_utf8_lossy(&out.stdout).into_owned());
    }
    let mut args = vec!["diff", "--no-color", "--no-ext-diff"];
    if staged {
        args.push("--cached");
    }
    args.extend(["--", path]);
    run(repo, &args)
}

// ---------------------------------------------------------------------------
// Branches

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Branch {
    pub name: String,
    pub current: bool,
    pub upstream: Option<String>,
}

pub fn branches(repo: &Path) -> Result<Vec<Branch>> {
    let raw = run(
        repo,
        &[
            "for-each-ref",
            "--sort=-committerdate",
            "--format=%(refname:short)%1f%(HEAD)%1f%(upstream:short)",
            "refs/heads",
        ],
    )?;
    Ok(parse_branches(&raw))
}

fn parse_branches(raw: &str) -> Vec<Branch> {
    raw.lines()
        .filter_map(|line| {
            let mut f = line.split('\x1f');
            let name = f.next()?.to_string();
            let current = f.next()? == "*";
            let upstream = f.next().filter(|u| !u.is_empty()).map(String::from);
            Some(Branch {
                name,
                current,
                upstream,
            })
        })
        .collect()
}

pub fn switch_branch(repo: &Path, name: &str) -> Result<()> {
    run(repo, &["switch", "--", name]).map(drop)
}

pub fn create_branch(repo: &Path, name: &str) -> Result<()> {
    run(repo, &["check-ref-format", "--branch", name])
        .map_err(|_| GitError::Failed(format!("'{name}' is not a valid branch name")))?;
    run(repo, &["switch", "-c", name]).map(drop)
}

// ---------------------------------------------------------------------------
// History

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Commit {
    pub hash: String,
    pub short_hash: String,
    pub author: String,
    pub email: String,
    /// Unix timestamp in seconds.
    pub time: i64,
    pub subject: String,
}

pub fn log(repo: &Path, limit: u32) -> Result<Vec<Commit>> {
    if !has_head(repo)? {
        return Ok(Vec::new());
    }
    let limit = format!("-n{limit}");
    let raw = run(
        repo,
        &[
            "log",
            &limit,
            "--format=%H%x1f%h%x1f%an%x1f%ae%x1f%at%x1f%s%x1e",
        ],
    )?;
    Ok(parse_log(&raw))
}

/// Full patch for a single commit, with a short header.
pub fn show_commit(repo: &Path, hash: &str) -> Result<String> {
    if hash.is_empty() || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(GitError::Failed(format!("invalid commit hash: {hash}")));
    }
    run(
        repo,
        &["show", "--no-color", "--no-ext-diff", "--format=%B", hash],
    )
}

fn parse_log(raw: &str) -> Vec<Commit> {
    raw.split('\x1e')
        .filter_map(|record| {
            let mut f = record.trim_start_matches('\n').split('\x1f');
            Some(Commit {
                hash: f.next().filter(|h| !h.is_empty())?.into(),
                short_hash: f.next()?.into(),
                author: f.next()?.into(),
                email: f.next()?.into(),
                time: f.next()?.parse().ok()?,
                subject: f.next()?.into(),
            })
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Remotes

// `env` carries credentials for hosts the user signed in to (see `github::git_env`).

pub fn fetch(repo: &Path, env: &[(String, String)]) -> Result<()> {
    run_with_env(repo, &["fetch", "--all", "--prune"], env).map(drop)
}

pub fn pull(repo: &Path, env: &[(String, String)]) -> Result<()> {
    run_with_env(repo, &["pull", "--ff-only"], env).map(drop)
}

pub fn push(repo: &Path, env: &[(String, String)]) -> Result<()> {
    // Publish branches that don't have an upstream yet.
    let branch = run(repo, &["symbolic-ref", "--short", "HEAD"])?;
    let has_upstream = output(repo, &["rev-parse", "--abbrev-ref", "@{upstream}"])?
        .status
        .success();
    if has_upstream {
        run_with_env(repo, &["push"], env).map(drop)
    } else {
        run_with_env(repo, &["push", "-u", "origin", branch.trim()], env).map(drop)
    }
}

/// URL of the `origin` remote, if there is one.
pub fn origin_url(repo: &Path) -> Result<Option<String>> {
    let out = output(repo, &["remote", "get-url", "origin"])?;
    Ok(out
        .status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string()))
}

pub fn add_origin(repo: &Path, url: &str) -> Result<()> {
    run(repo, &["remote", "add", "origin", url]).map(drop)
}

// ---------------------------------------------------------------------------
// Clone

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct CloneProgress {
    /// e.g. "Receiving objects"
    pub phase: String,
    pub percent: u8,
}

/// Accepts the URL forms people actually paste, and nothing that could make
/// git run a helper program (such as `ext::`) or be read as an option.
fn is_allowed_clone_url(url: &str) -> bool {
    let schemes = ["https://", "http://", "ssh://", "git://"];
    if schemes.iter().any(|s| url.starts_with(s)) {
        return true;
    }
    // scp-like syntax: user@host:path
    match url.split_once(':') {
        Some((user_host, path)) => {
            !path.is_empty()
                && user_host.contains('@')
                && !user_host.starts_with('-')
                && !user_host.contains('/')
        }
        None => false,
    }
}

/// Parses one line of `git clone --progress` output, e.g.
/// `Receiving objects:  45% (450/1000), 1.20 MiB | 2.00 MiB/s`.
fn parse_progress(line: &str) -> Option<CloneProgress> {
    let line = line.trim().trim_start_matches("remote: ");
    let (phase, rest) = line.split_once(':')?;
    let percent = rest.trim_start().split('%').next()?.trim().parse().ok()?;
    Some(CloneProgress {
        phase: phase.trim().to_string(),
        percent,
    })
}

/// Clones `url` into `parent/name` and returns the new repository's path.
/// `on_progress` is called whenever git reports a new percentage.
pub fn clone(
    url: &str,
    parent: &Path,
    name: &str,
    env: &[(String, String)],
    on_progress: impl FnMut(CloneProgress),
) -> Result<PathBuf> {
    let url = url.trim();
    if !is_allowed_clone_url(url) {
        return Err(GitError::Failed(format!(
            "Unsupported repository URL: {url}"
        )));
    }
    clone_unchecked(url, parent, name, env, on_progress)
}

/// [`clone`] without the URL allow-list; tests use it with `file://` URLs.
fn clone_unchecked(
    url: &str,
    parent: &Path,
    name: &str,
    env: &[(String, String)],
    mut on_progress: impl FnMut(CloneProgress),
) -> Result<PathBuf> {
    if name.is_empty() || name.contains(['/', '\\']) || name == "." || name == ".." {
        return Err(GitError::Failed(format!("Invalid folder name: {name}")));
    }
    let dest = parent.join(name);
    if dest.exists() {
        return Err(GitError::Failed(format!(
            "{} already exists",
            dest.display()
        )));
    }

    let mut child = command(parent, &["clone", "--progress", "--", url, name])
        .envs(env.iter().cloned())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()?;

    // git rewrites progress lines in place with '\r', so split on both.
    let mut stderr = child.stderr.take().expect("stderr is piped");
    let mut messages = Vec::new();
    let mut line = Vec::new();
    let mut last: Option<CloneProgress> = None;
    let mut buf = [0u8; 4096];
    loop {
        let n = stderr.read(&mut buf)?;
        if n == 0 {
            break;
        }
        for &byte in &buf[..n] {
            if byte != b'\r' && byte != b'\n' {
                line.push(byte);
                continue;
            }
            let text = String::from_utf8_lossy(&line).into_owned();
            line.clear();
            match parse_progress(&text) {
                Some(p) if last.as_ref() != Some(&p) => {
                    on_progress(p.clone());
                    last = Some(p);
                }
                Some(_) => {}
                None if !text.trim().is_empty() => messages.push(text),
                None => {}
            }
        }
    }

    if child.wait()?.success() {
        Ok(dest)
    } else {
        let tail = messages[messages.len().saturating_sub(5)..].join("\n");
        Err(GitError::Failed(if tail.is_empty() {
            "git clone failed".into()
        } else {
            tail
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_status() {
        let raw = concat!(
            "# branch.oid abc\0",
            "# branch.head main\0",
            "# branch.upstream origin/main\0",
            "# branch.ab +2 -1\0",
            "1 M. N... 100644 100644 100644 aaa bbb src/a file.rs\0",
            "1 .M N... 100644 100644 100644 aaa bbb b.rs\0",
            "2 R. N... 100644 100644 100644 aaa bbb R100 new.rs\0old.rs\0",
            "u UU N... 100644 100644 100644 100644 a b c conflict.rs\0",
            "? untracked.txt\0",
        );
        let s = parse_status(raw);
        assert_eq!(s.branch.as_deref(), Some("main"));
        assert_eq!(s.upstream.as_deref(), Some("origin/main"));
        assert_eq!((s.ahead, s.behind), (2, 1));
        assert_eq!(s.files.len(), 5);
        assert_eq!(s.files[0].path, "src/a file.rs");
        assert_eq!((s.files[0].index, s.files[0].worktree), ('M', '.'));
        assert_eq!(s.files[2].path, "new.rs");
        assert_eq!(s.files[2].orig_path.as_deref(), Some("old.rs"));
        assert!(s.files[3].conflicted);
        assert!(s.files[4].untracked);
    }

    #[test]
    fn parses_detached_head() {
        let s = parse_status("# branch.head (detached)\0");
        assert_eq!(s.branch, None);
    }

    #[test]
    fn parses_branches() {
        let b = parse_branches("main\x1f*\x1forigin/main\nfeature\x1f \x1f\n");
        assert_eq!(b.len(), 2);
        assert!(b[0].current);
        assert_eq!(b[0].upstream.as_deref(), Some("origin/main"));
        assert!(!b[1].current);
        assert_eq!(b[1].upstream, None);
    }

    /// Exercises the real `git` binary in a throwaway repository.
    #[test]
    fn end_to_end_flow() {
        let dir = std::env::temp_dir().join(format!("gitgud-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let repo = dir.as_path();
        run(repo, &["init", "-q", "-b", "main"]).unwrap();
        run(repo, &["config", "user.name", "Test"]).unwrap();
        run(repo, &["config", "user.email", "test@example.com"]).unwrap();
        run(repo, &["config", "commit.gpgsign", "false"]).unwrap();

        // Unborn branch: empty log, untracked file, stage/unstage without HEAD.
        assert!(log(repo, 10).unwrap().is_empty());
        std::fs::write(dir.join("a b.txt"), "hello\n").unwrap();
        assert!(status(repo).unwrap().files[0].untracked);
        assert!(diff(repo, "a b.txt", false, true)
            .unwrap()
            .contains("+hello"));
        stage(repo, &["a b.txt".into()]).unwrap();
        assert_eq!(status(repo).unwrap().files[0].index, 'A');
        unstage(repo, &["a b.txt".into()]).unwrap();
        assert!(status(repo).unwrap().files[0].untracked);

        // Shell metacharacters in the message must be stored verbatim.
        stage(repo, &["a b.txt".into()]).unwrap();
        let message = r#"first "quoted" $(touch pwned) `x`"#;
        commit(repo, message).unwrap();
        assert!(!dir.join("pwned").exists());
        let commits = log(repo, 10).unwrap();
        assert_eq!(commits[0].subject, message);
        assert!(status(repo).unwrap().files.is_empty());
        let shown = show_commit(repo, &commits[0].hash).unwrap();
        assert!(shown.contains("+hello"));
        assert!(show_commit(repo, "--help").is_err());

        create_branch(repo, "feature/x").unwrap();
        assert!(create_branch(repo, "bad..name").is_err());
        let b = branches(repo).unwrap();
        assert!(b.iter().any(|b| b.name == "feature/x" && b.current));
        switch_branch(repo, "main").unwrap();
        assert_eq!(status(repo).unwrap().branch.as_deref(), Some("main"));

        assert_eq!(origin_url(repo).unwrap(), None);
        add_origin(repo, "https://github.com/example/repo.git").unwrap();
        assert_eq!(
            origin_url(repo).unwrap().as_deref(),
            Some("https://github.com/example/repo.git")
        );

        assert!(matches!(
            open(&std::env::temp_dir()),
            Err(GitError::NotARepo(_))
        ));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn checks_clone_urls() {
        for ok in [
            "https://github.com/a/b.git",
            "git@github.com:a/b.git",
            "ssh://git@host/a/b",
        ] {
            assert!(is_allowed_clone_url(ok), "{ok}");
        }
        for bad in [
            "ext::sh -c touch% /tmp/pwned",
            "--upload-pack=touch /tmp/pwned",
            "/local/path",
            "file:///etc",
            "-u@host:path",
            "github.com/a/b",
        ] {
            assert!(!is_allowed_clone_url(bad), "{bad}");
        }
    }

    #[test]
    fn parses_clone_progress() {
        assert_eq!(
            parse_progress("Receiving objects:  45% (450/1000), 1.20 MiB | 2.00 MiB/s"),
            Some(CloneProgress {
                phase: "Receiving objects".into(),
                percent: 45
            })
        );
        assert_eq!(
            parse_progress("remote: Counting objects: 100% (12/12), done."),
            Some(CloneProgress {
                phase: "Counting objects".into(),
                percent: 100
            })
        );
        assert_eq!(parse_progress("Cloning into 'x'..."), None);
        assert_eq!(parse_progress("remote: Total 12 (delta 0)"), None);
    }

    #[test]
    fn clones_with_progress() {
        let base = std::env::temp_dir().join(format!("gitgud-clone-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let src = base.join("src");
        std::fs::create_dir_all(&src).unwrap();
        run(&src, &["init", "-q", "-b", "main"]).unwrap();
        std::fs::write(src.join("f.txt"), "x").unwrap();
        run(&src, &["add", "."]).unwrap();
        run(
            &src,
            &[
                "-c",
                "user.name=T",
                "-c",
                "user.email=t@x",
                "-c",
                "commit.gpgsign=false",
                "commit",
                "-qm",
                "c",
            ],
        )
        .unwrap();

        let url = format!("file://{}", src.display());
        assert!(
            clone(&url, &base, "dst", &[], |_| {}).is_err(),
            "file:// is not user-facing"
        );

        let mut events = Vec::new();
        let dest = clone_unchecked(&url, &base, "dst", &[], |p| events.push(p)).unwrap();
        assert!(dest.join("f.txt").exists());
        assert!(events.iter().any(|p| p.percent == 100), "{events:?}");

        assert!(
            clone_unchecked(&url, &base, "dst", &[], |_| {}).is_err(),
            "dest exists"
        );
        assert!(
            clone_unchecked(&url, &base, "../x", &[], |_| {}).is_err(),
            "bad name"
        );
        std::fs::remove_dir_all(&base).unwrap();
    }

    /// Needs network access: `cargo test -- --ignored`
    #[test]
    #[ignore]
    fn clones_from_github() {
        let base = std::env::temp_dir().join(format!("gitgud-gh-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        let mut events = Vec::new();
        let dest = clone(
            "https://github.com/octocat/Hello-World.git",
            &base,
            "hello",
            &[],
            |p| events.push(p),
        )
        .unwrap();
        assert!(dest.join("README").exists());
        assert!(!events.is_empty());
        std::fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn parses_log() {
        let raw = "h1\x1fs1\x1fAda\x1fa@x\x1f100\x1ffirst\x1e\nh2\x1fs2\x1fBo\x1fb@x\x1f200\x1fsecond | pipes\x1e\n";
        let c = parse_log(raw);
        assert_eq!(c.len(), 2);
        assert_eq!(c[1].subject, "second | pipes");
        assert_eq!(c[1].time, 200);
    }
}
