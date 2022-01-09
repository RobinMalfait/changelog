use crate::git::Git;
use color_eyre::eyre::{eyre, Result};

#[derive(Debug)]
pub struct Repo {
    pub org: String,
    pub repo: String,
}

impl Repo {
    pub fn new(org: String, repo: String) -> Self {
        Self { org, repo }
    }

    pub fn from_git_repo(pwd: &str) -> Result<Self> {
        match Git::exec(pwd, vec!["config", "--get", "remote.origin.url"]) {
            Ok(output) => {
                let output = output.replace(".git", "");

                let parts = output
                    .split(':')
                    .collect::<Vec<&str>>()
                    .pop()
                    .unwrap()
                    .split('/')
                    .collect::<Vec<&str>>();

                match (parts.get(0), parts.get(1)) {
                    (Some(owner), Some(repo)) => Ok(Self::new(owner.to_string(), repo.to_string())),
                    _ => Err(eyre!("Could not parse git remote url")),
                }
            }
            Err(e) => Err(eyre!(format!("Failed running git: {}", e.to_string()))),
        }
    }
}
