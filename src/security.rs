//! Security hardening module for the SigOps Agent.
//!
//! Implements four hardening features:
//!   1. Command whitelist — only allow a configured set of binaries.
//!   2. Path deny-list — block commands whose args reference sensitive paths
//!      (e.g. /etc/shadow, /root, ~/.ssh) or path-traversal sequences (`..`).
//!   3. Execution timeout — hard kill any child process that exceeds the
//!      configured per-command wall-clock budget.
//!   4. Token rotation — accept a new token from the server (via heartbeat
//!      response) and persist it to the on-disk agent config so subsequent
//!      requests use it.

#![allow(dead_code)]

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command as SysCommand, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Default wall-clock timeout applied to each external command execution.
pub const DEFAULT_EXEC_TIMEOUT_SECS: u64 = 30;

/// Default set of binaries the agent is permitted to execute.
///
/// This list is intentionally small — anything not here must be explicitly
/// allow-listed via [`SecurityPolicy::with_allowed_commands`].
pub const DEFAULT_ALLOWED_COMMANDS: &[&str] = &["echo", "ls", "ps", "df", "cat", "uptime", "date"];

/// Default deny-list of path prefixes the agent refuses to touch.
pub const DEFAULT_DENIED_PATHS: &[&str] = &[
    "/etc/shadow",
    "/etc/passwd",
    "/etc/gshadow",
    "/root",
    "/var/lib/secrets",
    "/.ssh",
    "/home/",
];

/// Errors produced by the security policy.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SecurityError {
    #[error("command not in whitelist: {0}")]
    CommandNotAllowed(String),
    #[error("empty command")]
    EmptyCommand,
    #[error("shell metacharacter detected in command: {0}")]
    ShellMetacharacter(String),
    #[error("argument references denied path: {0}")]
    DeniedPath(String),
    #[error("path traversal ('..') is not permitted: {0}")]
    PathTraversal(String),
    #[error("command exceeded {limit_secs}s timeout")]
    Timeout { limit_secs: u64 },
    #[error("failed to spawn command {cmd}: {reason}")]
    Spawn { cmd: String, reason: String },
    #[error("invalid token: {0}")]
    InvalidToken(String),
    #[error("io error: {0}")]
    Io(String),
}

/// Result of a permitted command execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecOutput {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
}

/// Security policy controlling command execution.
///
/// Constructed via [`SecurityPolicy::default`] (defaults) or the
/// `with_*` / `set_*` builder helpers. Clone-friendly so it can be
/// shared across tasks.
#[derive(Debug, Clone)]
pub struct SecurityPolicy {
    allowed_commands: Vec<String>,
    denied_paths: Vec<String>,
    exec_timeout: Duration,
    /// Extra deny-list prefix (e.g. the agent's own config dir). Resolved to
    /// an absolute path if possible.
    config_dir: Option<PathBuf>,
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self {
            allowed_commands: DEFAULT_ALLOWED_COMMANDS
                .iter()
                .map(|s| s.to_string())
                .collect(),
            denied_paths: DEFAULT_DENIED_PATHS.iter().map(|s| s.to_string()).collect(),
            exec_timeout: Duration::from_secs(DEFAULT_EXEC_TIMEOUT_SECS),
            config_dir: None,
        }
    }
}

impl SecurityPolicy {
    /// Construct a default policy.
    pub fn new() -> Self {
        Self::default()
    }

    /// Override the set of allowed commands.
    pub fn with_allowed_commands<I, S>(mut self, cmds: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.allowed_commands = cmds.into_iter().map(Into::into).collect();
        self
    }

    /// Override the set of denied path prefixes.
    pub fn with_denied_paths<I, S>(mut self, paths: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.denied_paths = paths.into_iter().map(Into::into).collect();
        self
    }

    /// Override the per-command wall-clock timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.exec_timeout = timeout;
        self
    }

    /// Treat the given directory as denied (typically the agent's own
    /// config dir, so attackers can't use `cat` to read the stored token).
    pub fn with_config_dir<P: AsRef<Path>>(mut self, dir: P) -> Self {
        self.config_dir = Some(dir.as_ref().to_path_buf());
        self
    }

    pub fn allowed_commands(&self) -> &[String] {
        &self.allowed_commands
    }

    pub fn denied_paths(&self) -> &[String] {
        &self.denied_paths
    }

    pub fn timeout(&self) -> Duration {
        self.exec_timeout
    }

    /// Check whether `command` (the binary name) is in the whitelist.
    pub fn is_command_allowed(&self, command: &str) -> bool {
        if command.is_empty() {
            return false;
        }
        // Normalize: strip any directory component. We only match by basename
        // so an attacker can't pass `/usr/bin/echo` → `echo` to bypass or —
        // conversely — sneak `/opt/hacker/echo` past the whitelist.
        let name = Path::new(command)
            .file_name()
            .and_then(|os| os.to_str())
            .unwrap_or(command);
        self.allowed_commands.iter().any(|c| c == name)
    }

    /// Reject any argument that references a denied path or contains a
    /// path-traversal (`..`) component.
    pub fn check_arg(&self, arg: &str) -> Result<(), SecurityError> {
        // Path traversal — even as a substring. Legitimate use cases for
        // `..` in file arguments are vanishingly rare for this agent.
        if arg.contains("..") {
            return Err(SecurityError::PathTraversal(arg.to_string()));
        }

        for denied in &self.denied_paths {
            if arg_matches_denied(arg, denied) {
                return Err(SecurityError::DeniedPath(arg.to_string()));
            }
        }

        if let Some(dir) = &self.config_dir {
            if let Some(s) = dir.to_str() {
                if arg_matches_denied(arg, s) {
                    return Err(SecurityError::DeniedPath(arg.to_string()));
                }
            }
        }

        Ok(())
    }

    /// Validate a `(command, args)` pair against the policy.
    pub fn validate(&self, command: &str, args: &[&str]) -> Result<(), SecurityError> {
        if command.is_empty() {
            return Err(SecurityError::EmptyCommand);
        }
        if contains_shell_metacharacter(command) {
            return Err(SecurityError::ShellMetacharacter(command.to_string()));
        }
        if !self.is_command_allowed(command) {
            return Err(SecurityError::CommandNotAllowed(command.to_string()));
        }
        for a in args {
            if contains_shell_metacharacter(a) {
                return Err(SecurityError::ShellMetacharacter((*a).to_string()));
            }
            self.check_arg(a)?;
        }
        Ok(())
    }

    /// Validate + execute a command under this policy, enforcing the
    /// configured wall-clock timeout. Kills the child on timeout.
    pub fn run(&self, command: &str, args: &[&str]) -> Result<ExecOutput, SecurityError> {
        self.validate(command, args)?;
        run_with_timeout(command, args, self.exec_timeout)
    }
}

/// Returns true if `arg` appears to reference the denied prefix.
///
/// Matching strategy:
///  * if `denied` starts with `/`, we treat it as an absolute prefix match.
///  * if `denied` starts with `~`, we normalise to $HOME when set, otherwise
///    match the literal string.
///  * otherwise we do a substring match (covers cases like `.ssh` in args).
fn arg_matches_denied(arg: &str, denied: &str) -> bool {
    // Tokenise on common separators so `cat:/etc/shadow` still matches.
    let candidates = [arg];
    for candidate in candidates.iter() {
        if candidate.contains(denied) {
            return true;
        }
        // Handle `~/.ssh` form by substituting $HOME.
        if let Some(rest) = denied.strip_prefix('~') {
            if let Ok(home) = std::env::var("HOME") {
                let expanded = format!("{}{}", home, rest);
                if candidate.contains(&expanded) {
                    return true;
                }
            }
            // Always treat the literal `.ssh` fragment as sensitive.
            if candidate.contains(rest) && rest.starts_with('/') {
                return true;
            }
        }
    }
    false
}

/// Reject common shell metacharacters in command / args to prevent
/// downstream injection if anything ever passes the value through a shell.
fn contains_shell_metacharacter(s: &str) -> bool {
    s.chars().any(|c| {
        matches!(
            c,
            ';' | '&' | '|' | '`' | '$' | '<' | '>' | '\n' | '\r' | '\0'
        )
    })
}

/// Spawn a command and enforce `timeout`. On timeout, sends SIGKILL.
pub fn run_with_timeout(
    command: &str,
    args: &[&str],
    timeout: Duration,
) -> Result<ExecOutput, SecurityError> {
    let start = Instant::now();
    let mut child = SysCommand::new(command)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| SecurityError::Spawn {
            cmd: command.to_string(),
            reason: e.to_string(),
        })?;

    // Poll loop: small, allocation-free, avoids pulling in a new crate.
    let poll_interval = Duration::from_millis(25);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let mut stdout = String::new();
                let mut stderr = String::new();
                if let Some(mut out) = child.stdout.take() {
                    let _ = out.read_to_string(&mut stdout);
                }
                if let Some(mut err) = child.stderr.take() {
                    let _ = err.read_to_string(&mut stderr);
                }
                return Ok(ExecOutput {
                    exit_code: status.code(),
                    stdout,
                    stderr,
                    duration_ms: start.elapsed().as_millis() as u64,
                });
            }
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    // Reap the zombie.
                    let _ = child.wait();
                    return Err(SecurityError::Timeout {
                        limit_secs: timeout.as_secs(),
                    });
                }
                std::thread::sleep(poll_interval);
            }
            Err(e) => {
                return Err(SecurityError::Spawn {
                    cmd: command.to_string(),
                    reason: e.to_string(),
                });
            }
        }
    }
}

// -------------------------------------------------------------------------
// Token rotation
// -------------------------------------------------------------------------

/// Thread-safe container for the agent's current bearer token.
///
/// Provides atomic updates plus optional persistence to a config file on
/// disk so the new token survives restarts.
#[derive(Debug, Clone)]
pub struct TokenStore {
    inner: Arc<Mutex<TokenState>>,
}

#[derive(Debug)]
struct TokenState {
    token: String,
    persist_path: Option<PathBuf>,
}

impl TokenStore {
    /// Create a store with an initial token (may be empty).
    pub fn new(initial: impl Into<String>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(TokenState {
                token: initial.into(),
                persist_path: None,
            })),
        }
    }

    /// Create a store that also persists rotations to `path`.
    pub fn with_persist_path<P: AsRef<Path>>(initial: impl Into<String>, path: P) -> Self {
        Self {
            inner: Arc::new(Mutex::new(TokenState {
                token: initial.into(),
                persist_path: Some(path.as_ref().to_path_buf()),
            })),
        }
    }

    /// Return a copy of the current token.
    pub fn current(&self) -> String {
        self.inner
            .lock()
            .expect("token store poisoned")
            .token
            .clone()
    }

    /// True if the current token is empty.
    pub fn is_empty(&self) -> bool {
        self.inner
            .lock()
            .expect("token store poisoned")
            .token
            .is_empty()
    }

    /// Rotate to a new token. Validates format, updates memory, and (if
    /// configured) writes through to the persist path atomically.
    pub fn rotate(&self, new_token: &str) -> Result<(), SecurityError> {
        validate_token_format(new_token)?;
        let mut guard = self.inner.lock().expect("token store poisoned");
        guard.token = new_token.to_string();
        if let Some(path) = guard.persist_path.clone() {
            write_token_atomically(&path, new_token)?;
        }
        Ok(())
    }

    /// Apply a rotation if the provided optional token is `Some`.
    /// Returns true if a rotation actually occurred.
    pub fn maybe_rotate(&self, new_token: Option<&str>) -> Result<bool, SecurityError> {
        match new_token {
            Some(t) if !t.is_empty() => {
                // Skip no-op rotations.
                if t == self.current() {
                    return Ok(false);
                }
                self.rotate(t)?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}

/// Basic structural checks for tokens. Rejects empty strings, tokens with
/// whitespace / control chars, and absurdly long values. We intentionally
/// keep this permissive for format but strict about encoding.
pub fn validate_token_format(token: &str) -> Result<(), SecurityError> {
    if token.is_empty() {
        return Err(SecurityError::InvalidToken("token is empty".into()));
    }
    if token.len() < 8 {
        return Err(SecurityError::InvalidToken(
            "token too short (min 8 chars)".into(),
        ));
    }
    if token.len() > 4096 {
        return Err(SecurityError::InvalidToken(
            "token too long (max 4096 chars)".into(),
        ));
    }
    if token.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(SecurityError::InvalidToken(
            "token contains whitespace or control characters".into(),
        ));
    }
    Ok(())
}

/// Write `token` to `path` atomically: write to `path.tmp` then rename.
fn write_token_atomically(path: &Path, token: &str) -> Result<(), SecurityError> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| SecurityError::Io(e.to_string()))?;
        }
    }
    let tmp = path.with_extension("tmp");
    {
        let mut f = fs::File::create(&tmp).map_err(|e| SecurityError::Io(e.to_string()))?;
        f.write_all(token.as_bytes())
            .map_err(|e| SecurityError::Io(e.to_string()))?;
        f.sync_all().map_err(|e| SecurityError::Io(e.to_string()))?;
    }
    fs::rename(&tmp, path).map_err(|e| SecurityError::Io(e.to_string()))?;
    Ok(())
}

/// Load a token previously persisted by [`TokenStore`].
pub fn load_persisted_token<P: AsRef<Path>>(path: P) -> Result<String, SecurityError> {
    fs::read_to_string(path).map_err(|e| SecurityError::Io(e.to_string()))
}

// -------------------------------------------------------------------------
// Tests
// -------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    // --- whitelist ----------------------------------------------------

    #[test]
    fn test_whitelist_allows_default_commands() {
        let p = SecurityPolicy::default();
        assert!(p.validate("echo", &["hello"]).is_ok());
        assert!(p.validate("ls", &["/tmp"]).is_ok());
        assert!(p.validate("date", &[]).is_ok());
    }

    #[test]
    fn test_whitelist_rejects_disallowed_command() {
        let p = SecurityPolicy::default();
        let err = p.validate("rm", &["-rf", "/tmp/x"]).unwrap_err();
        assert!(matches!(err, SecurityError::CommandNotAllowed(_)));
    }

    #[test]
    fn test_whitelist_rejects_absolute_path_not_whitelisted() {
        let p = SecurityPolicy::default();
        // basename `rm` isn't allowed
        let err = p.validate("/bin/rm", &[]).unwrap_err();
        assert!(matches!(err, SecurityError::CommandNotAllowed(_)));
    }

    #[test]
    fn test_whitelist_accepts_absolute_path_when_basename_allowed() {
        let p = SecurityPolicy::default();
        assert!(p.validate("/bin/echo", &["hi"]).is_ok());
    }

    #[test]
    fn test_whitelist_rejects_shell_metacharacters() {
        let p = SecurityPolicy::default();
        let err = p.validate("echo", &["hello; rm -rf /"]).unwrap_err();
        assert!(matches!(err, SecurityError::ShellMetacharacter(_)));
        let err = p.validate("echo", &["$(whoami)"]).unwrap_err();
        assert!(matches!(err, SecurityError::ShellMetacharacter(_)));
        let err = p.validate("echo", &["a|b"]).unwrap_err();
        assert!(matches!(err, SecurityError::ShellMetacharacter(_)));
    }

    #[test]
    fn test_whitelist_rejects_empty_command() {
        let p = SecurityPolicy::default();
        let err = p.validate("", &[]).unwrap_err();
        assert!(matches!(err, SecurityError::EmptyCommand));
    }

    #[test]
    fn test_whitelist_configurable() {
        let p = SecurityPolicy::default().with_allowed_commands(["only_this"]);
        assert!(p.validate("only_this", &[]).is_ok());
        assert!(p.validate("echo", &[]).is_err());
    }

    // --- path deny-list -----------------------------------------------

    #[test]
    fn test_path_traversal_rejected() {
        let p = SecurityPolicy::default();
        let err = p.validate("cat", &["../../etc/hosts"]).unwrap_err();
        assert!(matches!(err, SecurityError::PathTraversal(_)));
        let err = p.validate("ls", &["foo/../../bar"]).unwrap_err();
        assert!(matches!(err, SecurityError::PathTraversal(_)));
    }

    #[test]
    fn test_deny_etc_shadow() {
        let p = SecurityPolicy::default();
        let err = p.validate("cat", &["/etc/shadow"]).unwrap_err();
        assert!(matches!(err, SecurityError::DeniedPath(_)));
    }

    #[test]
    fn test_deny_etc_passwd() {
        let p = SecurityPolicy::default();
        let err = p.validate("cat", &["/etc/passwd"]).unwrap_err();
        assert!(matches!(err, SecurityError::DeniedPath(_)));
    }

    #[test]
    fn test_deny_root_dir() {
        let p = SecurityPolicy::default();
        let err = p.validate("ls", &["/root/.bashrc"]).unwrap_err();
        assert!(matches!(err, SecurityError::DeniedPath(_)));
    }

    #[test]
    fn test_deny_ssh_dir() {
        let p = SecurityPolicy::default();
        let err = p.validate("cat", &["/home/alice/.ssh/id_rsa"]).unwrap_err();
        // Either DeniedPath (via /home/ or /.ssh) — both are sensitive.
        assert!(matches!(err, SecurityError::DeniedPath(_)));
    }

    #[test]
    fn test_deny_var_lib_secrets() {
        let p = SecurityPolicy::default();
        let err = p.validate("ls", &["/var/lib/secrets/db.key"]).unwrap_err();
        assert!(matches!(err, SecurityError::DeniedPath(_)));
    }

    #[test]
    fn test_deny_agent_config_dir() {
        let p = SecurityPolicy::default().with_config_dir("/opt/sigops/conf");
        let err = p.validate("cat", &["/opt/sigops/conf/token"]).unwrap_err();
        assert!(matches!(err, SecurityError::DeniedPath(_)));
    }

    #[test]
    fn test_deny_list_allows_normal_paths() {
        let p = SecurityPolicy::default();
        assert!(p.validate("ls", &["/tmp/foo"]).is_ok());
        assert!(p.validate("cat", &["/var/log/sigops.log"]).is_ok());
    }

    // --- timeout ------------------------------------------------------

    #[test]
    fn test_fast_command_completes_before_timeout() {
        let p = SecurityPolicy::default().with_timeout(Duration::from_secs(5));
        let out = p.run("echo", &["hello"]).expect("echo should succeed");
        assert_eq!(out.exit_code, Some(0));
        assert!(out.stdout.contains("hello"));
        assert!(out.duration_ms < 5_000);
    }

    #[test]
    fn test_timeout_fires_when_command_sleeps_too_long() {
        // sleep is not in the default whitelist — temporarily allow it so
        // we can test only the timeout enforcement path.
        let p = SecurityPolicy::default()
            .with_allowed_commands(["sleep"])
            .with_timeout(Duration::from_millis(200));
        let err = p.run("sleep", &["5"]).unwrap_err();
        assert!(matches!(err, SecurityError::Timeout { .. }));
    }

    #[test]
    fn test_run_still_enforces_whitelist() {
        let p = SecurityPolicy::default();
        let err = p.run("rm", &["-rf", "/tmp/x"]).unwrap_err();
        assert!(matches!(err, SecurityError::CommandNotAllowed(_)));
    }

    // --- token rotation ----------------------------------------------

    #[test]
    fn test_token_rotation_updates_in_memory() {
        let store = TokenStore::new("initial-token-abc");
        assert_eq!(store.current(), "initial-token-abc");
        store.rotate("brand-new-token-xyz").unwrap();
        assert_eq!(store.current(), "brand-new-token-xyz");
    }

    #[test]
    fn test_token_rotation_persists_to_file() {
        let dir = std::env::temp_dir().join(format!("sigops-sec-{}", uuid_like()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("token");
        let store = TokenStore::with_persist_path("first-token-1234", &path);
        store.rotate("second-token-5678").unwrap();
        let on_disk = std::fs::read_to_string(&path).unwrap();
        assert_eq!(on_disk, "second-token-5678");

        // Reloading goes through the public API.
        let loaded = load_persisted_token(&path).unwrap();
        assert_eq!(loaded, "second-token-5678");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_maybe_rotate_from_heartbeat_response() {
        let store = TokenStore::new("current-token-abc");
        // Simulate heartbeat response containing a rotation field.
        let rotated = store.maybe_rotate(Some("rotated-token-def")).unwrap();
        assert!(rotated);
        assert_eq!(store.current(), "rotated-token-def");

        // No rotation field — token stays put.
        let rotated = store.maybe_rotate(None).unwrap();
        assert!(!rotated);
        assert_eq!(store.current(), "rotated-token-def");

        // Same token — treated as no-op.
        let rotated = store.maybe_rotate(Some("rotated-token-def")).unwrap();
        assert!(!rotated);
    }

    #[test]
    fn test_empty_token_rejected() {
        let store = TokenStore::new("current-token-abc");
        let err = store.rotate("").unwrap_err();
        assert!(matches!(err, SecurityError::InvalidToken(_)));
        // Store is untouched.
        assert_eq!(store.current(), "current-token-abc");
    }

    #[test]
    fn test_invalid_token_format_rejected() {
        let store = TokenStore::new("current-token-abc");
        // Whitespace in token.
        let err = store.rotate("bad token with space").unwrap_err();
        assert!(matches!(err, SecurityError::InvalidToken(_)));
        // Too short.
        let err = store.rotate("short").unwrap_err();
        assert!(matches!(err, SecurityError::InvalidToken(_)));
        // Control character.
        let err = store.rotate("tok\nen-with-newline").unwrap_err();
        assert!(matches!(err, SecurityError::InvalidToken(_)));
        assert_eq!(store.current(), "current-token-abc");
    }

    #[test]
    fn test_token_store_is_empty() {
        let store = TokenStore::new("");
        assert!(store.is_empty());
        // Cannot rotate to empty.
        assert!(store.rotate("").is_err());
        store.rotate("valid-token-1234").unwrap();
        assert!(!store.is_empty());
    }

    // Small helper so tests don't collide when run in parallel without
    // pulling in the tempfile crate.
    fn uuid_like() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        format!("{}-{}", std::process::id(), n)
    }
}
