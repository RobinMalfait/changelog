use color_eyre::eyre::{eyre, Result};
use std::process::Command;

#[derive(Debug)]
pub struct Git {}

impl Git {
    pub fn long_hash(pwd: &str, hash: &str) -> Result<String> {
        Self::exec(pwd, vec!["log", "-1", "--format=%H", hash])
    }

    pub fn short_hash(pwd: &str, hash: &str) -> Result<String> {
        Self::exec(pwd, vec!["log", "-1", "--format=%S", hash])
    }

    pub fn commit_message(pwd: &str, hash: &str) -> Result<String> {
        match Self::exec(pwd, vec!["log", "-1", "--format=%B", hash]) {
            Ok(msg) => {
                if msg.is_empty() {
                    Err(eyre!("No commit message found"))
                } else {
                    let msg = msg.trim().split('\n').next().unwrap_or(&msg);

                    Ok(msg.to_string())
                }
            }
            Err(e) => Err(e),
        }
    }

    pub fn is_git_repo(pwd: &str) -> bool {
        match Self::exec(pwd, vec!["rev-parse", "--is-inside-work-tree"]) {
            Ok(output) => output.trim() == "true",
            Err(_) => false,
        }
    }

    pub fn exec(pwd: &str, args: Vec<&str>) -> Result<String> {
        let mut cmd = Command::new("git");

        cmd.current_dir(pwd);

        for arg in args {
            cmd.arg(arg);
        }

        match cmd.output() {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stdout = stdout.trim();
                let stdout = stdout.to_string();

                Ok(stdout)
            }
            Err(e) => Err(eyre!(e)),
        }
    }
}
