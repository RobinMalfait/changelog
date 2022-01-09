use crate::github::github_url::GitHubURL;
use crate::github::repo::Repo;
use crate::graphql::graphql;
use serde_json::json;
use std::fmt::{Debug, Display};
use std::process::Command;
use std::str::FromStr;

#[derive(Debug)]
pub struct Commit {
    hash: String,
    short_hash: String,
    title: String,
    repo: Repo,
}

impl Commit {
    pub fn from_local_commit(pwd: &str, maybe_hash: &str) -> Result<Self, std::io::Error> {
        let repo = Repo::from_git_repo(pwd);

        let long_hash = Commit::calculate_long_hash(pwd, maybe_hash)?;
        let short_hash = Commit::calculate_short_hash(pwd, maybe_hash)?;
        let title = Commit::calculate_commit_message(pwd, maybe_hash)?;

        Ok(Self {
            hash: long_hash.to_string(),
            short_hash: short_hash.to_string(),
            title: title.to_string(),
            repo,
        })
    }

    pub fn calculate_long_hash(pwd: &str, hash: &str) -> Result<String, std::io::Error> {
        Self::exec_git(pwd, vec!["log", "-1", "--format=%H", hash])
    }

    pub fn calculate_short_hash(pwd: &str, hash: &str) -> Result<String, std::io::Error> {
        Self::exec_git(pwd, vec!["log", "-1", "--format=%S", hash])
    }

    pub fn calculate_commit_message(pwd: &str, hash: &str) -> Result<String, std::io::Error> {
        match Self::exec_git(pwd, vec!["log", "-1", "--format=%B", hash]) {
            Ok(msg) => {
                if msg.is_empty() {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "No commit message found",
                    ))
                } else {
                    let msg = msg.trim().split_once("\n").unwrap().0;

                    Ok(msg.to_string())
                }
            }
            Err(e) => Err(e),
        }
    }

    fn exec_git(pwd: &str, args: Vec<&str>) -> Result<String, std::io::Error> {
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
            Err(e) => Err(e),
        }
    }
}

impl Display for Commit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} ([{}](https://github.com/{}/{}/commit/{}))",
            self.title, self.short_hash, self.repo.org, self.repo.repo, self.hash
        )
    }
}

impl FromStr for Commit {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.parse::<GitHubURL>() {
            Ok(url) => {
                let commit = url.parts.get("commit").expect("Missing commit hash in URL");

                let data = json!({
                    "query": include_str!("./graphql/commit-info/query.graphql"),
                    "variables": {
                        "org": url.repo.org,
                        "repo": url.repo.repo,
                        "hash": commit
                    }
                });

                let json = graphql(data)?;

                let title = json["data"]["repository"]["object"]["title"]
                    .as_str()
                    .unwrap();
                let short_hash = json["data"]["repository"]["object"]["short_hash"]
                    .as_str()
                    .unwrap();

                Ok(Self {
                    hash: commit.to_string(),
                    short_hash: short_hash.to_string(),
                    title: title.to_string(),
                    repo: url.repo,
                })
            }
            Err(_) => {
                // TODO: Get from root
                let pwd = std::fs::canonicalize(".").expect("File path doesn't seem to exist");

                match Commit::from_local_commit(pwd.to_str().unwrap(), s) {
                    Ok(commit) => Ok(commit),
                    Err(e) => Err(e.to_string()),
                }
            }
        }
    }
}
