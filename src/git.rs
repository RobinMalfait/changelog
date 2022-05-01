use color_eyre::eyre::{eyre, Result};
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug)]
pub struct Git {
    pwd: PathBuf,
}

impl Git {
    pub fn new(pwd: Option<&PathBuf>) -> Result<Self> {
        Ok(Self {
            pwd: match pwd {
                Some(pwd) => pwd.to_path_buf(),
                None => std::env::current_dir()?,
            },
        })
    }

    pub fn long_hash(&self, hash: &str) -> Result<String> {
        self.exec(vec!["log", "-1", "--format=%H", hash])
    }

    pub fn short_hash(&self, hash: &str) -> Result<String> {
        self.exec(vec!["log", "-1", "--format=%S", hash])
    }

    pub fn commit_message(&self, hash: &str) -> Result<String> {
        match self.exec(vec!["log", "-1", "--format=%B", hash]) {
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

    pub fn is_git_repo(&self) -> bool {
        match self.exec(vec!["rev-parse", "--is-inside-work-tree"]) {
            Ok(output) => output.trim() == "true",
            Err(_) => false,
        }
    }

    pub fn add(&self, path: &str) -> Result<&Self> {
        self.exec(vec!["add", path])?;
        Ok(self)
    }

    pub fn commit(&self, msg: &str) -> Result<&Self> {
        self.exec(vec!["commit", "-m", msg])?;
        Ok(self)
    }

    pub fn exec(&self, args: Vec<&str>) -> Result<String> {
        let mut cmd = Command::new("git");

        cmd.current_dir(&self.pwd);

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
