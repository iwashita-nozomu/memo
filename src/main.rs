use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

const VERSION: &str = "0.1.0";

#[derive(Debug)]
struct Config {
    repo: PathBuf,
    remote: Option<String>,
    environment: Option<String>,
    auto_sync: bool,
}

#[derive(Debug)]
struct GitState {
    branch: String,
    upstream: Option<String>,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("memo: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.first().map(String::as_str) == Some("--version") {
        println!("memo {VERSION}");
        return Ok(());
    }
    if args.first().map(String::as_str) == Some("--help") {
        print_help();
        return Ok(());
    }
    if args.first().map(String::as_str) == Some("--sync") {
        return run_sync_worker(&args[1..]);
    }
    if args.first().map(String::as_str) == Some("--check-config") {
        if args.len() != 1 {
            return Err("--check-config does not accept additional arguments".into());
        }
        let config = Config::load()?;
        validate_repo(&config)?;
        println!("memo config: {}", config_path().display());
        println!("memo repo: {}", config.repo.display());
        return Ok(());
    }
    let mut tags = Vec::new();
    while !args.is_empty() {
        let first = args[0].clone();
        if first == "--tag" {
            if args.len() < 2 {
                return Err("--tag requires a value".into());
            }
            tags.push(normalize_tag(&args[1])?);
            args.drain(0..2);
        } else if let Some(value) = first.strip_prefix("--tag=") {
            tags.push(normalize_tag(value)?);
            args.remove(0);
        } else if let Some(value) = first.strip_prefix("tag:") {
            tags.push(normalize_tag(value)?);
            args.remove(0);
        } else {
            break;
        }
    }
    if args.is_empty() {
        return Err("a memo message is required".into());
    }

    let config = Config::load()?;
    validate_repo(&config)?;
    let timestamp = command_text("date", &["+%Y-%m-%dT%H:%M:%S%:z"])?;
    if !timestamp.contains('T') {
        return Err(format!("date returned an invalid timestamp: {timestamp}"));
    }
    let date = timestamp.split('T').next().unwrap_or("unknown-date");
    let timestamp_slug = timestamp.replace(':', "-");
    let environment = configured_environment(&config)?;
    let cwd = env::current_dir()
        .map_err(|error| format!("cannot resolve current directory: {error}"))?
        .canonicalize()
        .map_err(|error| format!("cannot resolve current directory: {error}"))?;
    let message = args.join(" ");
    let tags = unique_tags(tags);
    let tags_text = tags.join(",");
    let random = unique_suffix();
    let filename = format!("{timestamp_slug}_{random}.md");
    let data_root = config.repo.join("memo");
    let source = data_root
        .join("inbox")
        .join(&environment)
        .join(date)
        .join(&filename);
    let git_context = git_context(&cwd);
    let content = format_entry(
        &timestamp,
        &environment,
        &tags_text,
        &cwd,
        git_context.as_ref(),
        &message,
    );
    fs::create_dir_all(source.parent().expect("source has a parent"))
        .map_err(|error| format!("cannot create memo directory: {error}"))?;
    fs::write(&source, &content).map_err(|error| format!("cannot write memo: {error}"))?;

    let mut paths = vec![source.clone()];
    for tag in tags.iter().filter(|tag| tag.as_str() != "inbox") {
        let mirror = data_root
            .join(tag)
            .join(&environment)
            .join(date)
            .join(&filename);
        fs::create_dir_all(mirror.parent().expect("mirror has a parent"))
            .map_err(|error| format!("cannot create tag mirror directory: {error}"))?;
        fs::write(&mirror, &content)
            .map_err(|error| format!("cannot write tag mirror: {error}"))?;
        paths.push(mirror);
    }

    println!("saved: {}", source.display());
    if config.auto_sync {
        spawn_sync_worker(&config, &paths, &message)?;
    } else {
        eprintln!("memo: sync disabled (auto_sync=false)");
    }
    Ok(())
}

impl Config {
    fn load() -> Result<Self, String> {
        let path = config_path();
        let text = fs::read_to_string(&path).map_err(|error| {
            format!(
                "config not found at {} ({error}); copy config.example.toml to that path and set repo",
                path.display()
            )
        })?;
        let mut repo = None;
        let mut remote = None;
        let mut environment = None;
        let mut auto_sync = true;
        for (line_number, line) in text.lines().enumerate() {
            let line = line.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let (key, raw_value) = line
                .split_once('=')
                .ok_or_else(|| format!("invalid config line {}", line_number + 1))?;
            let value = unquote(raw_value.trim());
            match key.trim() {
                "repo" => repo = Some(expand_path(&value)?),
                "remote" => remote = Some(value),
                "environment" => environment = Some(value),
                "auto_sync" => {
                    auto_sync = match value.as_str() {
                        "true" | "1" | "yes" => true,
                        "false" | "0" | "no" => false,
                        _ => {
                            return Err(format!(
                                "invalid auto_sync value on line {}",
                                line_number + 1
                            ))
                        }
                    }
                }
                unknown => return Err(format!("unknown config key '{unknown}'")),
            }
        }
        let repo = repo.ok_or_else(|| "config must define repo = \"...\"".to_string())?;
        let auto_sync = env::var("MEMO_AUTO_SYNC")
            .ok()
            .map(|value| matches!(value.as_str(), "1" | "true" | "yes"))
            .unwrap_or(auto_sync);
        Ok(Self {
            repo,
            remote,
            environment,
            auto_sync,
        })
    }
}

fn config_path() -> PathBuf {
    if let Ok(path) = env::var("MEMO_CONFIG") {
        return PathBuf::from(path);
    }
    let base = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("memo").join("config.toml")
}

fn expand_path(value: &str) -> Result<PathBuf, String> {
    if value == "~" {
        return env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or_else(|| "HOME is not set; cannot expand repo path".into());
    }
    if let Some(rest) = value.strip_prefix("~/") {
        return env::var_os("HOME")
            .map(|home| PathBuf::from(home).join(rest))
            .ok_or_else(|| "HOME is not set; cannot expand repo path".into());
    }
    Ok(PathBuf::from(value))
}

fn unquote(value: &str) -> String {
    if value.len() >= 2
        && ((value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\'')))
    {
        value[1..value.len() - 1].to_string()
    } else {
        value.to_string()
    }
}

fn validate_repo(config: &Config) -> Result<(), String> {
    let root = config.repo.canonicalize().map_err(|error| {
        format!(
            "memo repo {} is unavailable: {error}",
            config.repo.display()
        )
    })?;
    let git_root = command_text_in(&root, "git", &["rev-parse", "--show-toplevel"])?
        .trim()
        .to_string();
    let git_root = PathBuf::from(git_root)
        .canonicalize()
        .map_err(|error| format!("cannot resolve memo repo root: {error}"))?;
    if root != git_root {
        return Err(format!(
            "memo repo must be a Git repository root: {}",
            root.display()
        ));
    }
    if let Some(expected) = &config.remote {
        let actual = command_text_in(&root, "git", &["remote", "get-url", "origin"])?;
        if actual.trim() != expected {
            return Err(format!(
                "origin mismatch: expected '{expected}', got '{}'",
                actual.trim()
            ));
        }
    }
    Ok(())
}

fn configured_environment(config: &Config) -> Result<String, String> {
    let raw = config
        .environment
        .clone()
        .or_else(|| env::var("MEMO_ENVIRONMENT").ok())
        .or_else(|| {
            env::var("WSL_DISTRO_NAME")
                .ok()
                .map(|value| format!("wsl-{value}"))
        })
        .or_else(|| command_text("hostname", &["-s"]).ok())
        .unwrap_or_else(|| "unknown-environment".to_string());
    let slug = raw
        .trim()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-') {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    Ok(if slug.is_empty() {
        "unknown-environment".into()
    } else {
        slug
    })
}

fn normalize_tag(raw: &str) -> Result<String, String> {
    let value = raw.trim().to_ascii_lowercase();
    if value.is_empty()
        || value == "."
        || value == ".."
        || !value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-')
        })
    {
        return Err(format!("invalid tag: {raw}"));
    }
    Ok(value)
}

fn unique_tags(tags: Vec<String>) -> Vec<String> {
    let mut unique = Vec::new();
    for tag in tags {
        if !unique.contains(&tag) {
            unique.push(tag);
        }
    }
    unique
}

fn unique_suffix() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!("{}-{}", std::process::id(), nanos % 1_000_000_000_000u128)
}

fn git_context(cwd: &Path) -> Option<Vec<(String, String)>> {
    let root = command_text_in(cwd, "git", &["rev-parse", "--show-toplevel"])
        .ok()?
        .trim()
        .to_string();
    let branch = command_text_in(cwd, "git", &["branch", "--show-current"])
        .ok()?
        .trim()
        .to_string();
    let head = command_text_in(cwd, "git", &["rev-parse", "--short", "HEAD"])
        .ok()?
        .trim()
        .to_string();
    Some(vec![
        ("git-root".into(), root),
        (
            "git-branch".into(),
            if branch.is_empty() {
                "detached".into()
            } else {
                branch
            },
        ),
        ("git-head".into(), head),
    ])
}

fn format_entry(
    timestamp: &str,
    environment: &str,
    tags: &str,
    cwd: &Path,
    git: Option<&Vec<(String, String)>>,
    message: &str,
) -> String {
    let mut content = format!(
        "## {timestamp}\n\nenvironment: {environment}\ntags: {tags}\ncwd: {}\n",
        cwd.display()
    );
    if let Some(git) = git {
        for (key, value) in git {
            content.push_str(&format!("{key}: {value}\n"));
        }
    }
    content.push_str(&format!("\n{message}\n"));
    content
}

fn capture_state(repo: &Path) -> GitState {
    let branch = command_text_in(repo, "git", &["symbolic-ref", "--quiet", "--short", "HEAD"])
        .unwrap_or_default()
        .trim()
        .to_string();
    let upstream = command_text_in(
        repo,
        "git",
        &[
            "rev-parse",
            "--abbrev-ref",
            "--symbolic-full-name",
            "@{upstream}",
        ],
    )
    .ok()
    .map(|value| value.trim().to_string())
    .filter(|value| !value.is_empty());
    GitState { branch, upstream }
}

fn synchronize_before_entry(config: &Config) -> Result<GitState, String> {
    let before = capture_state(&config.repo);
    let Some(upstream) = before.upstream.clone() else {
        return Ok(before);
    };
    let (remote, branch) = upstream
        .split_once('/')
        .unwrap_or(("origin", &before.branch));
    command_in(
        &config.repo,
        "git",
        &["fetch".to_string(), remote.to_string()],
    )?;
    let _ = command_in(
        &config.repo,
        "git",
        &["rebase".to_string(), "--abort".to_string()],
    );
    let _ = command_in(
        &config.repo,
        "git",
        &["merge".to_string(), "--abort".to_string()],
    );
    command_in(
        &config.repo,
        "git",
        &["reset".to_string(), "--hard".to_string(), upstream.clone()],
    )?;
    eprintln!("memo: synchronized {remote}/{branch} (local changes reset)");
    Ok(capture_state(&config.repo))
}

fn sync_error_path(repo: &Path) -> PathBuf {
    repo.join(".memo-sync-error.log")
}

fn spawn_sync_worker(config: &Config, paths: &[PathBuf], message: &str) -> Result<(), String> {
    let executable = env::current_exe()
        .map_err(|error| format!("cannot locate memo executable for background sync: {error}"))?;
    let mut command = Command::new(executable);
    command
        .arg("--sync")
        .arg(message)
        .args(paths)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if let Err(error) = command.spawn() {
        write_sync_error(
            &config.repo,
            &format!("cannot start background sync: {error}"),
        );
    }
    Ok(())
}

fn run_sync_worker(args: &[String]) -> Result<(), String> {
    if args.len() < 2 {
        return Err("background sync requires a message and at least one path".into());
    }
    let message = args[0].clone();
    let paths = args[1..].iter().map(PathBuf::from).collect::<Vec<_>>();
    let config = Config::load()?;
    validate_repo(&config)?;
    let error_path = sync_error_path(&config.repo);
    let result = (|| {
        let _lock = acquire_sync_lock(&config.repo)?;
        let before = synchronize_before_entry(&config)?;
        sync_entry(&config, &paths, &message, &before)
    })();
    match result {
        Ok(()) => {
            let _ = fs::remove_file(error_path);
        }
        Err(error) => write_sync_error(&config.repo, &error),
    }
    Ok(())
}

fn acquire_sync_lock(repo: &Path) -> Result<SyncLock, String> {
    let lock_path = repo.join(".git").join("memo-sync.lock");
    for _ in 0..600 {
        match fs::create_dir(&lock_path) {
            Ok(()) => {
                fs::write(lock_path.join("pid"), std::process::id().to_string())
                    .map_err(|error| format!("cannot write sync lock: {error}"))?;
                return Ok(SyncLock { path: lock_path });
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                thread::sleep(Duration::from_millis(100));
            }
            Err(error) => return Err(format!("cannot create sync lock: {error}")),
        }
    }
    Err("background sync lock remained busy for 60 seconds".into())
}

struct SyncLock {
    path: PathBuf,
}

impl Drop for SyncLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(self.path.join("pid"));
        let _ = fs::remove_dir(&self.path);
    }
}

fn write_sync_error(repo: &Path, error: &str) {
    let path = sync_error_path(repo);
    let body = format!("memo background sync error\n{error}\n");
    let _ = fs::write(path, body);
}

fn sync_entry(
    config: &Config,
    paths: &[PathBuf],
    message: &str,
    before: &GitState,
) -> Result<(), String> {
    let mut add_args = vec!["add".to_string(), "--".to_string()];
    add_args.extend(paths.iter().map(|path| path.to_string_lossy().into_owned()));
    command_in(&config.repo, "git", &add_args)?;
    let summary = message.lines().next().unwrap_or("entry");
    let commit_paths = paths
        .iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let mut commit_args = vec![
        "commit".to_string(),
        "--only".to_string(),
        "--no-verify".to_string(),
        "-m".to_string(),
        format!("memo: {summary}"),
        "--".to_string(),
    ];
    commit_args.extend(commit_paths);
    command_in(&config.repo, "git", &commit_args)?;
    eprintln!("memo: committed {} file(s)", paths.len());
    let Some(upstream) = before.upstream.clone() else {
        eprintln!("memo: committed locally; no upstream configured, push skipped");
        return Ok(());
    };
    let (remote, branch) = upstream
        .split_once('/')
        .unwrap_or(("origin", &before.branch));
    let push_result = push(&config.repo, remote, branch);
    if push_result.is_ok() {
        eprintln!("memo: pushed {remote}/{branch}");
        return Ok(());
    }
    eprintln!("memo: push raced with a remote update; fetching and rebasing before retry");
    command_in(
        &config.repo,
        "git",
        &["fetch".to_string(), remote.to_string()],
    )?;
    rebase_with_auto_resolution(&config.repo, &format!("{remote}/{branch}"), paths)?;
    push(&config.repo, remote, branch)?;
    eprintln!("memo: pushed {remote}/{branch} after fetch/rebase");
    Ok(())
}

fn rebase_with_auto_resolution(
    repo: &Path,
    upstream: &str,
    memo_paths: &[PathBuf],
) -> Result<(), String> {
    let first_error = match command_in(repo, "git", &["rebase".to_string(), upstream.to_string()]) {
        Ok(()) => return Ok(()),
        Err(error) => error,
    };
    for _ in 0..10 {
        let conflicts = command_text_in(repo, "git", &["diff", "--name-only", "--diff-filter=U"])?
            .lines()
            .map(str::to_string)
            .filter(|path| !path.is_empty())
            .collect::<Vec<_>>();
        if conflicts.is_empty() {
            if command_in(repo, "git", &["rebase".to_string(), "--skip".to_string()]).is_ok() {
                return Ok(());
            }
            break;
        }
        for path in conflicts {
            let full_path = repo.join(&path);
            let side = if memo_paths.iter().any(|memo_path| memo_path == &full_path) {
                "theirs"
            } else {
                "ours"
            };
            command_in(
                repo,
                "git",
                &[
                    "checkout".to_string(),
                    side.to_string(),
                    "--".to_string(),
                    path.clone(),
                ],
            )?;
            command_in(repo, "git", &["add".to_string(), "--".to_string(), path])?;
        }
        if command_in(
            repo,
            "git",
            &[
                "-c".to_string(),
                "core.editor=true".to_string(),
                "rebase".to_string(),
                "--continue".to_string(),
            ],
        )
        .is_ok()
        {
            return Ok(());
        }
    }
    let _ = command_in(repo, "git", &["rebase".to_string(), "--abort".to_string()]);
    Err(format!("automatic rebase resolution failed: {first_error}"))
}

fn push(repo: &Path, remote: &str, branch: &str) -> Result<(), String> {
    command_in(
        repo,
        "git",
        &[
            "push".to_string(),
            remote.to_string(),
            format!("HEAD:{branch}"),
        ],
    )
}

fn command_text(program: &str, args: &[&str]) -> Result<String, String> {
    command_output(Command::new(program).args(args))
}

fn command_text_in(dir: &Path, program: &str, args: &[&str]) -> Result<String, String> {
    command_output(Command::new(program).current_dir(dir).args(args))
}

fn command_in(dir: &Path, program: &str, args: &[String]) -> Result<(), String> {
    let output = Command::new(program)
        .current_dir(dir)
        .args(args)
        .output()
        .map_err(|error| format!("{program} failed to start: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(command_failure(program, &output))
    }
}

fn command_output(command: &mut Command) -> Result<String, String> {
    let output = command
        .output()
        .map_err(|error| format!("command failed to start: {error}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout)
            .trim_end()
            .to_string())
    } else {
        Err(command_failure("command", &output))
    }
}

fn command_failure(program: &str, output: &Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if stderr.is_empty() {
        format!("{program} exited with status {}", output.status)
    } else {
        format!("{program}: {stderr}")
    }
}

fn print_help() {
    println!(
        "memo {VERSION}\n\nUsage: memo [--tag TAG|tag:TAG] MESSAGE...\n       memo --check-config\n\nConfiguration: $XDG_CONFIG_HOME/memo/config.toml (default: ~/.config/memo/config.toml)"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_validation_accepts_safe_names() {
        assert_eq!(normalize_tag("TODO").unwrap(), "todo");
        assert!(normalize_tag("../escape").is_err());
    }

    #[test]
    fn duplicate_tags_are_removed() {
        assert_eq!(
            unique_tags(vec!["todo".into(), "todo".into(), "work".into()]),
            vec!["todo", "work"]
        );
    }
}
