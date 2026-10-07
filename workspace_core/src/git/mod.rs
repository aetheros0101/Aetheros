//! Safe git integration: scrubbed environment, status, diff, worktree, blame, log.

mod blame;
mod diff;
mod log;
mod status;
mod worktree;

pub use blame::blame;
pub use diff::diff;
pub use log::log;
pub use status::status;
pub use worktree::{worktree_add, worktree_list, worktree_remove};

use crate::error::{Result, WorkspaceError};
use std::path::Path;
use std::process::Command;

pub(crate) fn git(root: &Path) -> Command {
    let mut c = Command::new("git");
    c.current_dir(root).env_clear();
    for key in ["PATH", "LANG", "TMPDIR"] {
        if let Ok(v) = std::env::var(key) {
            c.env(key, v);
        }
    }
    c.env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_TERMINAL_PROMPT", "0")
        .args([
            "-c",
            "core.fsmonitor=false",
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "core.pager=cat",
        ]);
    c
}

pub(crate) fn run(mut c: Command) -> Result<std::process::Output> {
    c.output().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            WorkspaceError::Git("git programı bulunamadı".into())
        } else {
            WorkspaceError::Io(e)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn git_command_has_scrubbed_environment_and_safe_overrides() {
        std::env::set_var("AETHEROS_TEST_SECRET_API_KEY", "sirr");
        let c = git(Path::new("."));
        let envs: Vec<_> = c.get_envs().collect();
        assert!(!envs.iter().any(|(k, _)| k.to_string_lossy().contains("SECRET")));
        assert!(!envs.iter().any(|(k, _)| *k == "HOME"));
        let args: Vec<String> = c
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert!(args.contains(&"core.fsmonitor=false".to_string()));
        assert!(args.contains(&"core.hooksPath=/dev/null".to_string()));
    }
}
