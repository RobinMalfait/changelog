use crate::output::output;
use std::process::Command;

#[derive(Debug)]
pub struct Repo {
    pub org: String,
    pub repo: String,
}

impl Repo {
    pub fn new(org: String, repo: String) -> Self {
        Self { org, repo }
    }

    pub fn from_git_repo(pwd: &str) -> Self {
        let mut cmd = Command::new("git");

        cmd.current_dir(pwd)
            .arg("config")
            .arg("--get")
            .arg("remote.origin.url");

        match cmd.output() {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stdout = stdout.trim();
                let stdout = stdout.to_string();
                let stdout = stdout.replace(".git", "");

                let parts = stdout.split(':').collect::<Vec<&str>>()[1]
                    .split('/')
                    .collect::<Vec<&str>>();

                let owner = parts.get(0).unwrap().to_string();
                let repo = parts.get(1).unwrap().to_string();

                Self::new(owner, repo)
            }
            Err(e) => {
                output(format!("Failed running git: {}", e.to_string()));
                std::process::exit(1);
            }
        }
    }
}
